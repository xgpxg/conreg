//! conreg是一个参考了Nacos设计的分布式服务注册和配置中心。详情请看：[conreg](https://github.com/xgpxg/conreg)
//!
//! conreg-client是conreg的客户端SDK，用于集成到您的服务中和conreg-server通信。
//!
//! ℹ️ 注意：当前conreg的0.1.x版本仍处于快速迭代中，API在未来可能会发生变化
//!
//! # 快速开始
//!
//! ## 基本使用
//! 在项目的根目录下添加`bootstrap.yaml`配置文件：
//! ```yaml
//! conreg:
//!   # 服务ID
//!   # 服务ID是服务的唯一标识，同一命名空间下的服务ID不能重复
//!   service-id: test
//!   # 客户端配置，这些信息将会作为服务实例的基本信息提交到注册中心
//!   client:
//!     # 监听地址
//!     address: 127.0.0.1
//!     # 端口
//!     port: 8000
//!   # 配置中心配置
//!   config:
//!     # 配置中心地址
//!     server-addr: 127.0.0.1:8000
//!     # 配置ID
//!     # 如果多个配置中存在同名配置key，则靠后的配置将会覆盖之前的配置
//!     config-ids:
//!       - test.yaml
//!   # 注册中心配置
//!   discovery:
//!     # 注册中心地址
//!     server-addr:
//!       - 127.0.0.1:8000
//!       - 127.0.0.1:8001
//!       - 127.0.0.1:8002
//! ```
//!
//! 然后，在`main`函数中初始化：
//! ```rust
//! #[tokio::main]
//! async fn main(){
//!     // 初始化
//!     init().await;
//!
//!     // 获取配置项
//!     println!("{:?}", AppConfig::get::<String>("name"));
//!
//!     // 获取服务实例
//!     let instances = AppDiscovery::get_instances("your_service_id").await.unwrap();
//!     println!("service instances: {:?}", instances);
//! }
//! ```
//!
//! ## 命名空间
//! conreg使用命名空间（Namespace）来对配置和服务进行隔离，默认命名空间为`public`。
//!
//! ## 配置中心
//! 从配置中心中加载，并使用这些配置。目前仅支持`yaml`格式的配置。
//!
//! ### 初始化并加载配置
//! ```rust
//! #[tokio::main]
//!  async fn main() {
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
//!
//!     )
//!     .await;
//!     println!("{:?}", AppConfig::get::<String>("name"));
//!     println!("{:?}", AppConfig::get::<u32>("age"));
//!  }
//! ```
//!
//! ### 从配置文件初始化
//! conreg-client默认从项目根目录下的bootstrap.yaml加载配置初始化配置，就像SpringCloud一样。
//!
//! 以下是`bootstrap.yaml`配置示例
//!
//! ```yaml
//! conreg:
//!   config:
//!     server-addr: 127.0.0.1:8000
//!     config-ids:
//!       - your_config.yaml
//!
//! ```
//!
//! 然后调用`init`方法即可初始化并获取配置内容。
//! ```rust
//! #[tokio::main]
//!  async fn main() {
//!     init().await;
//!     // 或者指定配置文件路径
//!     // init_from_file("config.yaml").await;
//!     println!("{:?}", AppConfig::get::<String>("name"));
//!     println!("{:?}", AppConfig::get::<u32>("age"));
//!  }
//! ```
//!
//! ## 注册中心
//! 用于服务注册和发现。
//!
//! ### 初始化并加载配置
//! ```rust
//! #[tokio::main]
//! async fn main() {
//! let config = ConRegConfigBuilder::default()
//!     .service_id("your_service_id")
//!     .client(
//!         ClientConfigBuilder::default()
//!             .address("127.0.0.1")
//!             .port(8080)
//!             .build()
//!             .unwrap(),
//!     )
//!     .discovery(
//!         DiscoveryConfigBuilder::default()
//!             .server_addr("127.0.0.1:8000")
//!             .build()
//!             .unwrap(),
//!     )
//!     .build()
//!     .unwrap();
//!     let service_id = config.service_id.clone();
//!     init_with(config).await;
//!     let instances = AppDiscovery::get_instances(&service_id).await.unwrap();
//!     println!("service instances: {:?}", instances);
//! }
//! ```
//!
//! ### 从配置文件初始化
//!
//! 默认从`bootstrap.yaml`中加载配置。
//!
//! 以下是示例配置：
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
//! ```rust
//! #[tokio::main]
//!  async fn main() {
//!     init().await;
//!     // 或者指定配置文件路径
//!     // init_from_file("config.yaml").await;
//!     init_with(config).await;
//!
//!     let service_id = "your_service_id";
//!     let instances = AppDiscovery::get_instances(service_id).await.unwrap();
//!     println!("service instances: {:?}", instances);
//!  }
//! ```

use crate::conf::{ConRegConfig, ConRegConfigWrapper};
use crate::config::Configs;
use crate::discovery::{Discovery, DiscoveryClient};
pub use crate::protocol::Instance;
use anyhow::bail;
use serde::de::DeserializeOwned;
use std::path::PathBuf;
use std::process::exit;
use std::sync::{Arc, OnceLock, RwLock};

pub mod conf;
mod config;
mod discovery;
mod network;
mod protocol;
mod utils;
mod lb;

struct Conreg;

/// 存储配置内容
static CONFIGS: OnceLock<Arc<RwLock<Configs>>> = OnceLock::new();
/// 服务发现全局实例
static DISCOVERY: OnceLock<Discovery> = OnceLock::new();

impl Conreg {
    /// 初始化配置中心和注册中心
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
        utils::init_log();

        if config.config.is_some() {
            let config_client = config::ConfigClient::new(&config);
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

/// 初始化配置中心和注册中心
pub async fn init() {
    match Conreg::init(None).await {
        Ok(_) => {}
        Err(e) => {
            log::error!("conreg init failed: {}", e);
            exit(1);
        }
    };
}

/// 从配置文件初始化配置中心和注册中心
pub async fn init_from_file(path: impl Into<PathBuf>) {
    match Conreg::init(Some(path.into())).await {
        Ok(_) => {}
        Err(e) => {
            log::error!("conreg init failed: {}", e);
            exit(1);
        }
    };
}

/// 从自定义配置初始化
pub async fn init_with(config: ConRegConfig) {
    match Conreg::init_with(&config).await {
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

    /// 绑定配置内容到一个struct。
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

pub struct AppDiscovery;
impl AppDiscovery {
    /// 获取指定服务的可用的服务实例
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
    use std::collections::HashMap;
    use super::*;
    use crate::conf::{ClientConfigBuilder, ConRegConfigBuilder, DiscoveryConfigBuilder};
    use serde::Deserialize;
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
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .await;*/
        println!("{:?}", AppConfig::get::<String>("name"));
        println!("{:?}", AppConfig::get::<u32>("age"));

        #[derive(Deserialize)]
        struct MyConfig {
            name: String,
        }
        let my_config = AppConfig::bind::<MyConfig>().unwrap();
        println!("my config, name: {:?}", my_config.name);

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
                let instances = AppDiscovery::get_instances(utils::current_process_name().as_str()).await.unwrap();
                println!("current: {:?}", instances);
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });
        tokio::join!(h);
    }
}
