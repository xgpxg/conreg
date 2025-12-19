//! # Conreg Client
//!
//! Conreg is a distributed service registry and configuration center designed with reference to Nacos. See details: [conreg](https://github.com/xgpxg/conreg)
//!
//! conreg-client is the client SDK for conreg, used to integrate into your services and communicate with conreg-server.
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! Add a `bootstrap.yaml` configuration file in your project's root directory:
//!
//! ```yaml
//! conreg:
//!   # Service ID is the unique identifier of the service. Service IDs in the same namespace cannot be duplicated.
//!   service-id: test
//!   # Client configuration, this information will be submitted to the registry as basic information of the service instance
//!   client:
//!     # Listening address
//!     address: 127.0.0.1
//!     # Port
//!     port: 8000
//!   # Configuration center configuration
//!   config:
//!   # Configuration center address
//!     server-addr: 127.0.0.1:8000
//!     # Configuration ID
//!     # If there are duplicate configuration keys in multiple configurations, the latter configuration will overwrite the previous one
//!     config-ids:
//!       - test.yaml
//!     auth-token: your_token
//!   # Registry configuration
//!   discovery:
//!     # Registry address
//!     server-addr:
//!       - 127.0.0.1:8000
//!       - 127.0.0.1:8001
//!       - 127.0.0.1:8002
//!     auth-token: your_token
//! ```
//!
//! Then, initialize in the `main` function:
//!
//! ```rust
//! #[tokio::main]
//! async fn main() {
//!     // Initialization
//!     init().await;
//!     // Get configuration item
//!     println!("{:?}", AppConfig::get::<String>("name"));
//!     // Get service instances
//!     let instances = AppDiscovery::get_instances("your_service_id").await.unwrap();
//!     println!("service instances: {:?}", instances);
//! }
//! ```
//!
//! ## Namespace
//!
//! Conreg uses namespaces to isolate configurations and services. The default namespace is `public`.
//!
//! ## Configuration Center
//!
//! Load and use configurations from the configuration center. Currently only `yaml` format configurations are supported.
//!
//! ### Initialize and Load Configuration
//!
//! ```rust
//! #[tokio::main]
//! async fn main() {
//!     init_with(
//!         ConRegConfigBuilder::default()
//!             .config(
//!                 ConfigConfigBuilder::default()
//!                     .server_addr("127.0.0.1:8000")
//!                     .namespace("public")
//!                     .config_ids(vec!["test.yaml".into()])
//!                     .build()
//!                     .unwrap(),
//!             )
//!             .build()
//!             .unwrap(),
//!     )
//!         .await;
//!     println!("{:?}", AppConfig::get::<String>("name"));
//!     println!("{:?}", AppConfig::get::<u32>("age"));
//! }
//! ```
//!
//! ### Initialize from Configuration File
//!
//! By default, conreg-client loads configurations from the bootstrap.yaml file in the project root directory to initialize configurations, just like SpringCloud.
//! The following is an example of `bootstrap.yaml` configuration:
//!
//! ```yaml
//! conreg:
//!   config:
//!     server-addr: 127.0.0.1:8000
//!     config-ids:
//!       - your_config.yaml
//! ```
//!
//! Then call the `init` method to initialize and get the configuration content.
//!
//! ```rust
//! #[tokio::main]
//! async fn main() {
//!     init().await;
//!     // Or specify the configuration file path
//!     // init_from_file("config.yaml").await;
//!     println!("{:?}", AppConfig::get::<String>("name"));
//!     println!("{:?}", AppConfig::get::<u32>("age"));
//! }
//! ```
//!
//! ## Registry Center
//!
//! Used for service registration and discovery.
//!
//! ### Initialize and Load Configuration
//!
//! ```rust
//! #[tokio::main]
//! async fn main() {
//!     let config = ConRegConfigBuilder::default()
//!         .service_id("your_service_id")
//!         .client(
//!             ClientConfigBuilder::default()
//!                 .address("127.0.0.1")
//!                 .port(8080)
//!                 .build()
//!                 .unwrap(),
//!         )
//!         .discovery(
//!             DiscoveryConfigBuilder::default()
//!                 .server_addr("127.0.0.1:8000")
//!                 .build()
//!                 .unwrap(),
//!         )
//!         .build()
//!         .unwrap();
//!     let service_id = config.service_id.clone();
//!     init_with(config).await;
//!     let instances = AppDiscovery::get_instances(&service_id).await.unwrap();
//!     println!("service instances: {:?}", instances);
//! }
//! ```
//!
//! ### Initialize from Configuration File
//!
//! By default, configurations are loaded from `bootstrap.yaml`.
//! The following is an example configuration:
//!
//! ```yaml
//! conreg:
//!   service-id: your_service_id
//!   client:
//!     address: 127.0.0.1
//!     port: 8000
//!   discovery:
//!     server-addr:
//!       - 127.0.0.1:8000
//!       - 127.0.0.1:8001
//!       - 127.0.0.1:8002
//! ```
//!
//! ```rust
//! #[tokio::main]
//! async fn main() {
//!     init().await;
//!     // Or specify the configuration file path
//!     // init_from_file("config.yaml").await;
//!     init_with(config).await;
//!     let service_id = "your_service_id";
//!     let instances = AppDiscovery::get_instances(service_id).await.unwrap();
//!     println!("service instances: {:?}", instances);
//! }
//! ```
//!
//! # Load Balancing
//!
//! conreg-client provides a load balancing client based on `reqwest`, supporting custom protocol requests in the format `lb://service_id`.
//! Reference: [lb](https://docs.rs/conreg-client/latest/conreg_client/lb/index.html)
//!
//! # Listen for Configuration Changes
//!
//! Add a handler function for the specified config_id, which will be called when the configuration changes.
//!
//! ```rust
//! AppConfig::add_listener("test.yaml", |config| {
//! println!("Config changed, new config: {:?}", config);
//! });
//! ```

use crate::conf::{ConRegConfig, ConRegConfigWrapper};
use crate::config::Configs;
use crate::discovery::{Discovery, DiscoveryClient};
pub use crate::protocol::Instance;
use anyhow::bail;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::exit;
use std::sync::{Arc, OnceLock, RwLock};

pub mod conf;
mod config;
mod discovery;
pub mod lb;
mod network;
mod protocol;
mod utils;

struct Conreg;

/// Store configuration content
static CONFIGS: OnceLock<Arc<RwLock<Configs>>> = OnceLock::new();
/// Global instance for service discovery
static DISCOVERY: OnceLock<Discovery> = OnceLock::new();
/// Request header for namespace authentication
const NS_TOKEN_HEADER: &str = "X-NS-Token";

impl Conreg {
    /// Initialize configuration center and registry center
    async fn init(file: Option<PathBuf>) -> anyhow::Result<()> {
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

        log::info!("conreg init completed");
        Ok(())
    }

    async fn init_with(config: &ConRegConfig) -> anyhow::Result<()> {
        #[cfg(feature = "logger")]
        utils::init_log();

        if config.config.is_some() {
            let config_client = config::ConfigClient::new(config);
            let configs = config_client.load().await?;
            CONFIGS.set(Arc::new(RwLock::new(configs))).map_err(|_| {
                anyhow::anyhow!(
                    "config has already been initialized, please do not initialize repeatedly"
                )
            })?;
        }

        if config.discovery.is_some() {
            let discovery_client = DiscoveryClient::new(config);
            discovery_client.register().await?;
            let discovery = Discovery::new(discovery_client).await;
            DISCOVERY.set(discovery).map_err(|_| {
                anyhow::anyhow!(
                    "discovery has already been initialized, please do not initialize repeatedly"
                )
            })?;
        }

        Ok(())
    }
}

/// Initialize configuration center and registry center
pub async fn init() {
    match Conreg::init(None).await {
        Ok(_) => {}
        Err(e) => {
            log::error!("conreg init failed: {}", e);
            exit(1);
        }
    };
}

/// Initialize configuration center and registry center from configuration file
pub async fn init_from_file(path: impl Into<PathBuf>) {
    match Conreg::init(Some(path.into())).await {
        Ok(_) => {}
        Err(e) => {
            log::error!("conreg init failed: {}", e);
            exit(1);
        }
    };
}

/// Initialize from custom configuration
pub async fn init_with(config: ConRegConfig) {
    match Conreg::init_with(&config).await {
        Ok(_) => {}
        Err(e) => {
            log::error!("conreg init failed: {}", e);
            exit(1);
        }
    };
}

/// Application Configuration
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

    /// Get configuration value
    ///
    /// `key` is the key of the configuration item, such as `app.name`.
    ///
    /// Note: The type of the obtained value needs to be consistent with the type of the value in the configuration.
    /// If they are inconsistent, it may cause conversion failure. When conversion fails, `None` will be returned.
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

    /// Add configuration listener
    ///
    /// - `config_id`: Configuration ID
    /// - `handler`: Configuration listener function, parameter is the changed, merged and flattened configuration content
    pub fn add_listener(config_id: &str, handler: fn(&HashMap<String, serde_yaml::Value>)) {
        Configs::add_listener(config_id, handler);
    }
}

/// Service Discovery
pub struct AppDiscovery;
impl AppDiscovery {
    /// Get available service instances for the specified service
    pub async fn get_instances(service_id: &str) -> anyhow::Result<Vec<Instance>> {
        match DISCOVERY.get() {
            Some(discovery) => {
                let instances = discovery.get_instances(service_id).await;
                Ok(instances)
            }
            None => {
                bail!("discovery not initialized")
            }
        }
    }
}

#[cfg(test)]
#[allow(unused)]
mod tests {
    use super::*;
    use crate::conf::{ClientConfigBuilder, ConRegConfigBuilder, DiscoveryConfigBuilder};
    use serde::Deserialize;
    use std::collections::HashMap;
    #[tokio::test]
    async fn test_config() {
        //init_log();
        init().await;
        //init_from_file("bootstrap.yaml").await;
        /*init_with(
            ConRegConfigBuilder::default()
                .config(
                    ConfigConfigBuilder::default()
                        .server_addr("127.0.0.1:8000")
                        .namespace("public")
                        .config_ids(vec!["test.yaml".into()])
                        .auth_token(Some("2cTtsBUpor".to_string()))
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .await;*/
        println!("{:?}", AppConfig::get::<String>("name"));
        println!("{:?}", AppConfig::get::<u32>("age"));

        AppConfig::add_listener("test.yaml", |config| {
            println!("Listen config change1: {:?}", config);
        });
        AppConfig::add_listener("test2.yml", |config| {
            println!("Listen config change2: {:?}", config);
        });
        let h = tokio::spawn(async move {
            loop {
                println!("{:?}", AppConfig::get::<String>("name"));
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });
        tokio::join!(h);
    }

    #[tokio::test]
    async fn test_discovery() {
        //init_log();
        init().await;
        // let config = ConRegConfigBuilder::default()
        //     .service_id("your_service_id")
        //     .client(
        //         ClientConfigBuilder::default()
        //             .address("127.0.0.1")
        //             .port(8080)
        //             .build()
        //             .unwrap(),
        //     )
        //     .discovery(
        //         DiscoveryConfigBuilder::default()
        //             .server_addr(vec!["127.0.0.1:8000", "127.0.0.1:8001"])
        //             .build()
        //             .unwrap(),
        //     )
        //     .build()
        //     .unwrap();
        // // println!("config: {:?}", config);
        // let service_id = config.service_id.clone();
        // init_with(config).await;
        let h = tokio::spawn(async move {
            loop {
                println!("{:?}", AppConfig::get::<String>("name"));
                let instances = AppDiscovery::get_instances(utils::current_process_name().as_str())
                    .await
                    .unwrap();
                println!("current: {:?}", instances);
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });
        tokio::join!(h);
    }
}
