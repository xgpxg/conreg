use crate::utils;
use derive_builder::Builder;
use serde::Deserialize;
use serde_yaml::Value;
use std::collections::HashMap;

/// 配置/注册中心的整体配置
/// 包一层是因为适配bootstrap.yaml中顶层的key为conreg
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ConRegConfigWrapper {
    pub(crate) conreg: ConRegConfig,
}

/// 配置/注册中心配置
#[derive(Debug, Clone, Deserialize, Builder)]
#[serde(rename_all = "kebab-case")]
pub struct ConRegConfig {
    /// 服务ID
    #[allow(unused)]
    #[serde(default = "ConRegConfig::default_service_id")]
    #[builder(setter(into), default = "ConRegConfig::default_service_id()")]
    pub service_id: String,
    /// 客户端配置
    #[builder(default = "ClientConfig::default()")]
    pub client: ClientConfig,
    /// 配置中心配置项
    #[serde(default)]
    #[builder(setter(strip_option), default)]
    pub config: Option<ConfigConfig>,
    /// 注册中心配置项
    #[serde(default)]
    #[builder(setter(strip_option), default)]
    pub discovery: Option<DiscoveryConfig>,
}

impl Default for ConRegConfig {
    fn default() -> Self {
        ConRegConfig {
            client: ClientConfig::default(),
            service_id: utils::current_process_name(),
            config: None,
            discovery: None,
        }
    }
}

impl ConRegConfig {
    fn default_service_id() -> String {
        utils::current_process_name()
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum ServerAddr {
    Single(String),
    Cluster(Vec<String>),
    Unset,
}

impl Default for ServerAddr {
    fn default() -> Self {
        ServerAddr::Unset
    }
}

impl From<&str> for ServerAddr {
    fn from(value: &str) -> Self {
        ServerAddr::Single(value.to_string())
    }
}
impl From<Vec<&str>> for ServerAddr {
    fn from(value: Vec<&str>) -> Self {
        ServerAddr::Cluster(value.into_iter().map(|s| s.to_string()).collect())
    }
}
impl From<Vec<String>> for ServerAddr {
    fn from(value: Vec<String>) -> Self {
        ServerAddr::Cluster(value)
    }
}

#[derive(Debug, Deserialize, Clone, Builder)]
pub struct ClientConfig {
    #[builder(setter(into))]
    pub address: String,
    pub port: u16,
}
impl Default for ClientConfig {
    fn default() -> Self {
        ClientConfig {
            address: "127.0.0.1".to_string(),
            port: 8080,
        }
    }
}

impl ClientConfig {
    pub fn gen_instance_id(&self) -> String {
        let digest = md5::compute(format!("{}:{}", self.address, self.port));
        format!("{:x}", digest)
    }
}

#[derive(Debug, Clone, Deserialize, Default, Builder)]
#[serde(rename_all = "kebab-case")]
pub struct ConfigConfig {
    /// 配置中心地址
    #[builder(setter(into))]
    pub server_addr: ServerAddr,
    /// 命名空间，默认为：public
    #[serde(default = "ConfigConfig::default_namespace")]
    #[builder(setter(into), default = "ConfigConfig::default_namespace()")]
    pub namespace: String,
    /// 配置ID，如：`["application.yaml"]`
    #[serde(default)]
    pub config_ids: Vec<String>,
}

impl ConfigConfig {
    /// 默认命名空间
    fn default_namespace() -> String {
        "public".to_string()
    }
}

#[derive(Debug, Clone, Deserialize, Default, Builder)]
#[serde(rename_all = "kebab-case")]
pub struct DiscoveryConfig {
    /// 配置中心地址，如：127.0.0.1:8000
    #[builder(setter(into))]
    pub server_addr: ServerAddr,
    /// 命名空间，默认为：public
    #[serde(default = "DiscoveryConfig::default_namespace")]
    #[builder(setter(into), default = "DiscoveryConfig::default_namespace()")]
    pub namespace: String,
    #[serde(default = "HashMap::default")]
    #[builder(setter(into), default = "HashMap::default()")]
    /// 元数据
    pub meta: HashMap<String, Value>,
}

impl DiscoveryConfig {
    /// 默认命名空间
    fn default_namespace() -> String {
        "public".to_string()
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub enum LoadBalanceStrategy {
    /// 轮询
    #[default]
    RoundRobin,
    /// 加权轮询
    Weighted,
    /// 随机
    Random,
    /// 加权随机
    WeightedRandom,
}
