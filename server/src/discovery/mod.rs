use crate::discovery::server::DiscoveryManager;
use std::process::exit;
use tracing::log;

#[allow(clippy::module_inception)]
mod discovery;
pub mod server;
use crate::Args;
pub use discovery::ServiceInstance;

#[derive(Debug)]
pub struct DiscoveryApp {
    pub manager: DiscoveryManager,
}
pub async fn new_discovery_app(args: &Args) -> DiscoveryApp {
    let manager = DiscoveryManager::new(args).await;
    if let Err(e) = manager {
        log::error!("create discovery app error: {}", e);
        exit(1);
    }

    DiscoveryApp {
        manager: manager.unwrap(),
    }
}
