/// Configuration component
use crate::utils;
use derive_builder::Builder;
use serde::Deserialize;
use serde_yaml::Value;
use std::collections::HashMap;

/// Overall configuration for config/registry center
/// Wrapped because the top-level key in bootstrap.yaml is conreg
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ConRegConfigWrapper {
    pub(crate) conreg: ConRegConfig,
}

/// Config/Registry Center Configuration
#[derive(Debug, Clone, Deserialize, Builder)]
#[serde(rename_all = "kebab-case")]
pub struct ConRegConfig {
    /// Service ID
    #[allow(unused)]
    #[serde(default = "ConRegConfig::default_service_id")]
    #[builder(setter(into), default = "ConRegConfig::default_service_id()")]
    pub service_id: String,
    /// Client configuration
    #[builder(default = "ClientConfig::default()")]
    pub client: ClientConfig,
    /// Configuration center configuration
    #[serde(default)]
    #[builder(setter(strip_option), default)]
    pub config: Option<ConfigConfig>,
    /// Registry center configuration
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

#[derive(Debug, Default, Deserialize, Clone)]
#[serde(untagged)]
pub enum ServerAddr {
    Single(String),
    Cluster(Vec<String>),
    #[default]
    Unset,
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
    #[builder(setter(into), default = "ClientConfig::default_address()")]
    pub address: String,
    pub port: u16,
}
impl Default for ClientConfig {
    fn default() -> Self {
        ClientConfig {
            address: ClientConfig::default_address(),
            port: 8080,
        }
    }
}

impl ClientConfig {
    pub fn gen_instance_id(&self) -> String {
        let digest = md5::compute(format!("{}:{}", self.address, self.port));
        format!("{:x}", digest)
    }
    pub fn default_address() -> String {
        "127.0.0.1".to_string()
    }
}

#[derive(Debug, Clone, Deserialize, Default, Builder)]
#[serde(rename_all = "kebab-case")]
pub struct ConfigConfig {
    /// Configuration center address
    #[builder(setter(into))]
    pub server_addr: ServerAddr,
    /// Namespace, default: public
    #[serde(default = "ConfigConfig::default_namespace")]
    #[builder(setter(into), default = "ConfigConfig::default_namespace()")]
    pub namespace: String,
    /// Configuration IDs, e.g.: `["application.yaml"]`
    #[serde(default)]
    pub config_ids: Vec<String>,
    /// Namespace authentication token
    #[builder(setter(into), default = "Default::default()")]
    pub auth_token: Option<String>,
}

impl ConfigConfig {
    /// Default namespace
    fn default_namespace() -> String {
        "public".to_string()
    }
}

#[derive(Debug, Clone, Deserialize, Default, Builder)]
#[serde(rename_all = "kebab-case")]
pub struct DiscoveryConfig {
    /// Configuration center address, e.g.: 127.0.0.1:8000
    #[builder(setter(into))]
    pub server_addr: ServerAddr,
    /// Namespace, default: public
    #[serde(default = "DiscoveryConfig::default_namespace")]
    #[builder(setter(into), default = "DiscoveryConfig::default_namespace()")]
    pub namespace: String,
    /// Metadata
    #[serde(default = "HashMap::default")]
    #[builder(setter(into), default = "HashMap::default()")]
    pub meta: HashMap<String, Value>,
    /// Namespace authentication token
    #[builder(setter(into), default = "Default::default()")]
    pub auth_token: Option<String>,
}

impl DiscoveryConfig {
    /// Default namespace
    fn default_namespace() -> String {
        "public".to_string()
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub enum LoadBalanceStrategy {
    /// Round Robin
    #[default]
    RoundRobin,
    /// Weighted Round Robin
    Weighted,
    /// Random
    Random,
    /// Weighted Random
    WeightedRandom,
}
