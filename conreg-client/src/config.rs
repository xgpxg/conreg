use crate::conf::{ConfigConfig, ServerAddr};
use crate::network::HTTP;
use crate::protocol::request::{GetConfigReq, WatchConfigChangeReq};
use crate::{AppConfig, ConRegConfig};
use anyhow::Context;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_yaml::{Mapping, Value, from_str};
use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::Duration;

pub struct ConfigClient {
    // 配置的配置😅
    config: ConfigConfig,
}

impl ConfigClient {
    pub fn new(config: &ConRegConfig) -> Self {
        ConfigClient {
            config: config
                .config
                .clone()
                .context("config not set, unable to create config client")
                .unwrap(),
        }
    }

    /// 初始化配置
    pub(crate) async fn load(&self) -> anyhow::Result<Configs> {
        let mut contents = vec![];
        for id in self.config.config_ids.iter() {
            contents.push(
                Self::fetch_config(
                    &self.config.server_addr,
                    &self.config.namespace,
                    id,
                    &self.config.auth_token,
                )
                .await?,
            );
        }

        // 启动监听，监听配置变化
        self.start_watch().await?;

        // 启动补偿任务，定时拉取配置
        self.start_compensate().await?;

        Configs::from_contents(contents)
    }

    /// 从配置中心加载指定配置ID的配置内容
    ///
    /// - server_addr: 配置中心地址
    /// - namespace: 命名空间
    /// - config_id: 配置ID
    /// - auth_token: 鉴权token
    async fn fetch_config(
        server_addr: &ServerAddr,
        namespace: &str,
        config_id: &str,
        auth_token: &Option<String>,
    ) -> anyhow::Result<String> {
        let url = server_addr.build_url("/api/config/get")?;
        let query = GetConfigReq {
            namespace_id: namespace.to_string(),
            id: config_id.to_string(),
        };

        let result = HTTP
            .get::<HashMap<String, Value>>(
                &url,
                query,
                match auth_token {
                    Some(token) => Some(vec![(crate::NS_TOKEN_HEADER, token.as_str())]),
                    None => None,
                },
            )
            .await?;

        let content = result
            .get("content")
            // if content is none, maybe config id not exists
            .ok_or(anyhow::anyhow!(
                "config id [ {} ] not found in server",
                config_id
            ))?
            .as_str()
            .unwrap();
        log::info!("config {} fetched", config_id);

        Ok(content.to_string())
    }

    /// 开启配置变更监听任务
    ///
    /// 目前使用长轮询的方式，在没有配置变更时，server会阻塞29秒后返回false；
    /// 在有配置变更时，server会立即返回true，然后重新从server拉取配置。
    async fn start_watch(&self) -> anyhow::Result<()> {
        let config_clone = self.config.clone();
        tokio::spawn(async move {
            log::info!(
                "start watch config changes in namespace: {}",
                config_clone.namespace
            );
            let url = config_clone
                .server_addr
                .build_url("/api/config/watch")
                .context("build url error from server addr")
                .unwrap();
            let query = WatchConfigChangeReq {
                namespace_id: config_clone.namespace.clone(),
            };

            loop {
                match HTTP.get::<Option<String>>(&url, &query, None).await {
                    Ok(changed_config_id) => {
                        if changed_config_id.is_none() {
                            log::info!("config no changed");
                            continue;
                        }
                        log::info!("config changed, reloading config");
                        let mut contents = vec![];
                        for id in config_clone.config_ids.iter() {
                            contents.push(
                                Self::fetch_config(
                                    &config_clone.server_addr,
                                    &config_clone.namespace,
                                    id,
                                    &config_clone.auth_token,
                                )
                                .await
                                .unwrap(),
                            );
                        }
                        // 新配置
                        let config = Configs::from_contents(contents).unwrap();
                        // 展平后的配置
                        let new_configs = config.get_all().clone();

                        // 重新加载
                        AppConfig::reload(config);
                        log::info!("config reloaded");

                        // 通知listeners配置变更
                        Self::notify_config_change(
                            &changed_config_id.unwrap(), // SAFE: 已经校验了None
                            &new_configs,
                        );
                    }
                    Err(e) => {
                        log::error!("watch config changes error: {}", e);
                        // when some error, sleep 0.5s and retry
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                };
            }
        });
        Ok(())
    }

    /// 开启配置补偿任务
    ///
    /// 每60秒从配置中心同步一次配置
    async fn start_compensate(&self) -> anyhow::Result<()> {
        let config_clone = self.config.clone();
        tokio::spawn(async move {
            log::info!(
                "start config compensate in namespace: {}",
                config_clone.namespace
            );

            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;

                log::debug!("starting fetch config");
                let mut contents = vec![];
                for id in config_clone.config_ids.iter() {
                    match Self::fetch_config(
                        &config_clone.server_addr,
                        &config_clone.namespace,
                        id,
                        &config_clone.auth_token,
                    )
                    .await
                    {
                        Ok(res) => contents.push(res),
                        Err(e) => {
                            log::error!("fetch config error: {}", e);
                            tokio::time::sleep(Duration::from_millis(500)).await;
                        }
                    };
                }
                AppConfig::reload(Configs::from_contents(contents).unwrap());
                log::debug!("config fetch success");
            }
        });
        Ok(())
    }

    /// 配置变更通知
    fn notify_config_change(config_id: &str, changed_configs: &HashMap<String, Value>) {
        let listeners = CONFIG_LISTENER.listeners.get(config_id);
        if let Some(listeners) = listeners
            && !listeners.is_empty()
        {
            for handler in &*listeners {
                handler(changed_configs)
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Configs {
    /// 展平后的配置
    pub configs: HashMap<String, Value>,
    /// 配置内容，目前为yaml格式
    pub content: Value,
}

type ConfigListeners = DashMap<String, Vec<fn(&HashMap<String, Value>)>>;
/// 配置变更监听
struct ConfigListener {
    /// key为配置ID，value为监听函数
    listeners: ConfigListeners,
}
static CONFIG_LISTENER: LazyLock<ConfigListener> = LazyLock::new(|| ConfigListener {
    listeners: DashMap::new(),
});

impl Configs {
    fn from_contents(contents: Vec<String>) -> anyhow::Result<Self> {
        let mut merged_config = Value::Mapping(Mapping::new());

        // 依次解析并合并每个配置文件
        // 后面的配置会覆盖前面相同键的配置
        for content in contents {
            if !content.trim().is_empty() {
                let config_value: Value = from_str(&content)?;
                Self::merge_yaml_values(&mut merged_config, config_value);
            }
        }

        // 配置键展开
        let mut configs = HashMap::new();
        Self::flatten_yaml_value(&mut configs, "", &merged_config);

        Ok(Configs {
            configs,
            content: merged_config,
        })
    }

    /// 递归合并两个 YAML 值
    /// 后面的值会覆盖前面相同键的值
    fn merge_yaml_values(target: &mut Value, source: Value) {
        match (target, source) {
            // 当两个值都是mapping类型时，递归合并
            (Value::Mapping(target_map), Value::Mapping(source_map)) => {
                for (key, value) in source_map {
                    if target_map.contains_key(&key) {
                        // 如果目标中已存在该key，则递归合并
                        Self::merge_yaml_values(target_map.get_mut(&key).unwrap(), value);
                    } else {
                        // 如果目标中不存在该key，则直接插入
                        target_map.insert(key, value);
                    }
                }
            }
            // 其他情况下，直接用源值覆盖目标值
            (target, source) => {
                *target = source;
            }
        }
    }

    /// 展开yaml的key，通过"."分隔
    fn flatten_yaml_value(result: &mut HashMap<String, Value>, prefix: &str, value: &Value) {
        match value {
            Value::Mapping(mapping) => {
                for (key, val) in mapping {
                    let key_str = match key {
                        Value::String(s) => s.clone(),
                        Value::Number(num) => num.to_string(),
                        _ => "unknown".to_string(),
                    };

                    let new_prefix = if prefix.is_empty() {
                        key_str
                    } else {
                        format!("{}.{}", prefix, key_str)
                    };

                    Self::flatten_yaml_value(result, &new_prefix, val);
                }
            }
            _ => {
                // 叶子节点
                result.insert(prefix.to_string(), value.clone());
            }
        }
    }

    /// 获取配置项
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.configs.get(key)
    }

    /// 获取所有配置项
    #[allow(unused)]
    pub fn get_all(&self) -> &HashMap<String, Value> {
        &self.configs
    }

    /// 检查配置是否存在
    #[allow(unused)]
    pub fn contains(&self, key: &str) -> bool {
        self.configs.contains_key(key)
    }

    /// 添加配置监听器
    pub fn add_listener(config_id: &str, handler: fn(&HashMap<String, Value>)) {
        if let Some(mut handlers) = CONFIG_LISTENER.listeners.get_mut(config_id) {
            handlers.push(handler);
        } else {
            CONFIG_LISTENER
                .listeners
                .insert(config_id.to_string(), vec![handler]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_app_config() {
        let contents = vec![
            r#"
            a: 1
            b: 2
            c:
              d: 3
              e: 4
              f:
                g: 0
            h:
              - 1
              - 2
            "#
            .to_string(),
            r#"
            a: 5
            b: 6
            c:
              d: 7
              e: 8
              f: x
            1: -1
            h:
              - 1
              - 3
            "#
            .to_string(),
        ];
        let config = Configs::from_contents(contents).unwrap();
        println!("{:?}", config);
        println!("{:?}", config.get("a"));
        println!("{:?}", config.get("h"));
    }
}
