pub mod server;

use crate::Args;
use crate::namespace::server::NamespaceManager;
use std::process::exit;
use tracing::log;

#[derive(Debug)]
pub struct NamespaceApp {
    pub manager: NamespaceManager,
}

pub async fn new_namespace_app(args: &Args) -> NamespaceApp {
    let manager = NamespaceManager::new(args).await;
    if let Err(e) = manager {
        log::error!("create namespace app error: {}", e);
        exit(1);
    }
    NamespaceApp {
        manager: manager.unwrap(),
    }
}
