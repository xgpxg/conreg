//! # Conreg配置和注册中心客户端
//! 示例：
//! ```rust
//! #[tokio::main]
//!  async fn test_init() {
//!     // 从默认的bootstrap.yaml初始化
//!     //init().await;
//!     // 从指定的文件初始化
//!     //init_from_file("your_config_file.yml").await;
//!     // 从指定的配置初始化
//!     init_with(ConRegConfig {
//!             service_id: "test".to_string(),
//!             config: Config {
//!                 server_addr: "127.0.0.1:8000".to_string(),
//!                 namespace: "public".to_string(),
//!                 config_ids: vec!["app.yaml".to_string()],
//!             },
//!         })
//!     .await;
//!
//!     // 获取配置
//!     println!("{:?}", AppConfig::get::<String>("name"));
//!     println!("{:?}", AppConfig::get::<u32>("age"));
//!
//!     // 绑定配置内容到一个struct
//!     #[derive(Deserialize)]
//!     struct MyConfig {
//!         name: String,
//!     }
//!     let my_config = AppConfig::bind::<MyConfig>().unwrap();
//!     println!("my config, name: {:?}", my_config.name);
//!  }
//!
//! ```
//!

use crate::config::Configs;
use anyhow::bail;
use env_logger::WriteStyle;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::path::PathBuf;
use std::process::exit;
use std::sync::{Arc, OnceLock, RwLock};

mod config;
mod reg;

struct ConReg;

/// 配置/注册中心的整体配置
/// 包一层是因为适配bootstrap.yaml中顶层的key为conreg
#[derive(Debug, Deserialize, Clone)]
struct ConRegConfigWrapper {
    conreg: ConRegConfig,
}

/// 配置/注册中心配置
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct ConRegConfig {
    /// 服务ID
    #[allow(unused)]
    service_id: String,
    /// 配置中心配置项
    #[serde(default = "ConRegConfig::default_config")]
    config: Config,
}

impl ConRegConfig {
    fn default_config() -> Config {
        Config::default()
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
struct Config {
    /// 配置中心地址，如：127.0.0.1:8000
    server_addr: String,
    /// 命名空间，默认为：public
    #[serde(default = "Config::default_namespace")]
    namespace: String,
    /// 配置ID，如：`["application.yaml"]`
    #[serde(default)]
    config_ids: Vec<String>,
}

impl Config {
    /// 默认命名空间
    fn default_namespace() -> String {
        "public".to_string()
    }
}

#[derive(Debug, Deserialize, Default, Clone)]
struct Discovery {}

/// 存储配置内容
static CONFIGS: OnceLock<Arc<RwLock<Configs>>> = OnceLock::new();

impl ConReg {
    /// 初始化配置中心和注册中心
    async fn init(file: Option<PathBuf>) -> anyhow::Result<()> {
        init_log();
        let mut file = file.unwrap_or("bootstrap.yaml".into());
        if !file.exists() {
            file = "bootstrap.yml".into();
        }
        let s = match std::fs::read_to_string(&file) {
            Ok(s) => s,
            Err(e) => {
                log::error!("no bootstrap.yaml found, {}", e);
                exit(1);
            }
        };

        log::info!("loaded bootstrap config from {}", file.display());

        let config = match serde_yaml::from_str::<ConRegConfigWrapper>(&s) {
            Ok(config) => config,
            Err(e) => {
                log::error!("parse bootstrap.yaml failed, {}", e);
                exit(1);
            }
        };

        Self::init_with(&config.conreg).await?;

        Ok(())
    }

    async fn init_with(config: &ConRegConfig) -> anyhow::Result<()> {
        init_log();
        let config = config::ConfigClient::new(&config.config).load().await?;
        CONFIGS.set(Arc::new(RwLock::new(config))).map_err(|_| {
            anyhow::anyhow!(
                "config has already been initialized, please do not initialize repeatedly"
            )
        })?;
        Ok(())
    }
}

/// 初始化配置中心和注册中心
pub async fn init() {
    match ConReg::init(None).await {
        Ok(_) => {}
        Err(e) => {
            log::error!("conreg init failed: {}", e);
            exit(1);
        }
    };
}

/// 从配置文件初始化配置中心和注册中心
pub async fn init_from_file(path: &str) {
    match ConReg::init(Some(path.into())).await {
        Ok(_) => {}
        Err(e) => {
            log::error!("conreg init failed: {}", e);
            exit(1);
        }
    };
}

/// 从自定义配置初始化
pub async fn init_with(config: ConRegConfig) {
    match ConReg::init_with(&config).await {
        Ok(_) => {}
        Err(e) => {
            log::error!("conreg init failed: {}", e);
            exit(1);
        }
    };
}

pub struct AppConfig;
impl AppConfig {
    fn reload(configs: Configs) {
        match CONFIGS.get() {
            None => {
                log::error!("config not init");
            }
            Some(config) => {
                *config.write().unwrap() = configs;
            }
        }
    }

    /// 获取配置值
    ///
    /// 注意：获取的值类型需要与配置中的值类型保持一致，如果不一致，可能会导致转换失败，
    /// 转换失败时将返回`None`
    pub fn get<V: DeserializeOwned>(key: &str) -> Option<V> {
        match CONFIGS.get() {
            None => {
                log::error!("config not init");
                None
            }
            Some(config) => match config.read().expect("read lock error").get(key) {
                None => None,
                Some(value) => match serde_yaml::from_value::<V>(value.clone()) {
                    Ok(value) => Some(value),
                    Err(e) => {
                        log::error!("parse config failed, {}", e);
                        None
                    }
                },
            },
        }
    }

    pub fn bind<T: DeserializeOwned>() -> anyhow::Result<T> {
        match CONFIGS.get() {
            None => {
                bail!("config not init");
            }
            Some(config) => {
                let value: T = serde_yaml::from_value(
                    config.read().expect("read lock error").content.clone(),
                )?;
                Ok(value)
            }
        }
    }
}

fn init_log() {
    use std::io::Write;
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .format(|buf, record| {
            let level = record.level().as_str();
            writeln!(
                buf,
                "[{}][{}] - {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                level,
                record.args()
            )
        })
        .write_style(WriteStyle::Always)
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_init() {
        //init_log();
        //init().await;
        //init_from_file("bootstrap.yaml").await;
        init_with(ConRegConfig {
            service_id: "test".to_string(),
            config: Config {
                server_addr: "127.0.0.1:8000".to_string(),
                namespace: "public".to_string(),
                config_ids: vec!["test.yaml".to_string()],
            },
        })
        .await;
        println!("{:?}", AppConfig::get::<String>("name"));
        println!("{:?}", AppConfig::get::<u32>("age"));

        #[derive(Deserialize)]
        struct MyConfig {
            name: String,
        }
        let my_config = AppConfig::bind::<MyConfig>().unwrap();
        println!("my config, name: {:?}", my_config.name);

        tokio::spawn(async move {
            loop {
                println!("{:?}", AppConfig::get::<String>("name"));
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });
        tokio::time::sleep(std::time::Duration::from_secs(50)).await;
    }
}
