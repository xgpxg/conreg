pub mod server;

use crate::Args;
use std::process::exit;
use tracing::log;

use crate::config::server::ConfigManager;

#[derive(Debug)]
pub struct ConfigApp {
    pub manager: ConfigManager,
}

pub async fn new_config_app(args: &Args) -> ConfigApp {
    let manager = ConfigManager::new(args).await;
    if let Err(e) = manager {
        log::error!("create config app error: {}", e);
        exit(1);
    }
    ConfigApp {
        manager: manager.unwrap(),
    }
}
