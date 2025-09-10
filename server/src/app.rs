use crate::config::ConfigApp;
use crate::discovery::DiscoveryApp;
use crate::namespace::NamespaceApp;
use crate::raft::store::StateMachineData;
use crate::raft::{LogStore, Network, NodeId, Raft, StateMachine};
use crate::{Args, config, discovery, namespace, raft};
use anyhow::Context;
use clap::Parser;
use openraft::Config;
use rocket::futures::executor::block_on;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;
use tracing::log;

pub struct App {
    /// 节点ID
    pub id: NodeId,
    /// 节点地址
    pub addr: String,
    /// Raft协议
    pub raft: Raft,
    /// 状态机
    /// 注意这个需要共享状态，Raft应用log后会修改这个，在读取数据时，也从这里读
    pub state_machine: Arc<RwLock<StateMachineData>>,
    /// 应用额外数据
    #[allow(unused)]
    pub other: Arc<RwLock<HashMap<String, String>>>,
    /// 配置中心
    pub config_app: ConfigApp,
    /// 命名空间
    pub namespace_app: NamespaceApp,
    /// 服务发现
    pub discovery_app: DiscoveryApp,
}

impl App {
    pub async fn new(args: &Args) -> App {
        let config = Config {
            heartbeat_interval: 500,
            election_timeout_min: 1500,
            election_timeout_max: 3000,
            ..Default::default()
        };

        // 校验配置是否有效
        let config = Arc::new(config.validate().unwrap());

        // 创建日志存储和状态机存储
        let (log_store, state_machine_store): (LogStore, StateMachine) =
            raft::store::new(&args.data_dir).await;

        // 创建网络
        let network = Network {};

        // 当前状态机数据
        let state_machine = state_machine_store.state_machine.clone();

        // 创建raft实例
        let raft = Raft::new(
            args.node_id,
            config.clone(),
            network,
            log_store.clone(),
            state_machine_store,
        )
        .await
        .unwrap();

        // 本机地址，用于节点间的通信
        let addr = format!("{}:{}", args.address, args.port);

        // 配置中心实例
        let config_app = config::new_config_app(&args).await;

        // 命名空间实例
        let namespace_app = namespace::new_namespace_app(&args).await;

        // 服务发现实例
        let discovery_app = discovery::new_discovery_app(&args).await;

        App {
            id: args.node_id,
            addr,
            raft,
            state_machine,
            other: Arc::new(Default::default()),
            config_app,
            namespace_app,
            discovery_app,
        }
    }
}

static APP: OnceLock<App> = OnceLock::new();

pub async fn init() -> anyhow::Result<()> {
    let app = App::new(&Args::parse()).await;
    APP.get_or_init(|| app);
    Ok(())
}

pub fn get_app() -> &'static App {
    APP.get().context("APP not init").unwrap()
}

impl App {
    /// 退出前清理资源
    pub fn clean(&self) {
        // TODO 当有客户端连接时，可能导致无法保存最终快照
        block_on(async {
            // 保存状态机快照
            if let Err(e) = self.raft.trigger().snapshot().await {
                log::error!("raft state machine snapshot error: {}", e);
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            log::info!("raft state persistence successful");
        })
    }
}

pub fn cleanup() {
    get_app().clean();
}
