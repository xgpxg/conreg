use crate::app::get_app;
use crate::raft::RaftRequest;
use std::sync::LazyLock;
use tokio::sync::mpsc;
use tracing::log;

pub enum Event {
    RaftRequestEvent(RaftRequest),
}

impl Event {
    pub fn send(self) -> Result<(), mpsc::error::SendError<Event>> {
        EVENT_BUS.send(self)
    }
}

pub struct EventBus {
    sender: mpsc::UnboundedSender<Event>,
}

impl EventBus {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel::<Event>();
        let handler = EventHandler::new(receiver);

        tokio::spawn(async move {
            handler.handle_events().await;
        });

        Self { sender }
    }

    pub fn send(&self, event: Event) -> Result<(), mpsc::error::SendError<Event>> {
        self.sender.send(event)
    }
}

static EVENT_BUS: LazyLock<EventBus> = LazyLock::new(|| EventBus::new());

pub struct EventHandler {
    receiver: mpsc::UnboundedReceiver<Event>,
}

impl EventHandler {
    pub fn new(receiver: mpsc::UnboundedReceiver<Event>) -> Self {
        Self { receiver }
    }

    pub async fn handle_events(mut self) {
        while let Some(event) = self.receiver.recv().await {
            self.process_event(event).await;
        }
    }

    async fn process_event(&self, event: Event) {
        match event {
            Event::RaftRequestEvent(req) => {
                self.handle_raft_request(req).await;
            }
        }
    }

    async fn handle_raft_request(&self, req: RaftRequest) {
        match req {
            // 这两个在apply时已经处理
            RaftRequest::Set { .. } | RaftRequest::Delete { .. } => {}
            // 配置中心配置变更
            RaftRequest::SetConfig { entry } => {
                match get_app().config_app.manager.insert_config(entry).await {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("Error processing SetConfig request: {}", e);
                    }
                };
            }
            // 配置中心删除配置
            RaftRequest::DeleteConfig { namespace_id, id } => {
                match get_app()
                    .config_app
                    .manager
                    .delete_config(&namespace_id, &id)
                    .await
                {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("Error processing DeleteConfig request: {}", e);
                    }
                };
            }
            RaftRequest::UpdateConfig { entry } => {
                match get_app().config_app.manager.update_config(entry).await {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("Error processing UpdateConfig request: {}", e);
                    }
                };
            }
            RaftRequest::UpsertNamespace { namespace } => {
                match get_app()
                    .namespace_app
                    .manager
                    .upsert_namespace(namespace)
                    .await
                {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("Error processing UpsertNamespace request: {}", e);
                    }
                };
            }
            RaftRequest::DeleteNamespace { id } => {
                match get_app().namespace_app.manager.delete_namespace(&id).await {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("Error processing DeleteNamespace request: {}", e);
                    }
                };
            }
            RaftRequest::RegisterService { service } => {
                match get_app()
                    .discovery_app
                    .manager
                    .register_service(service)
                    .await
                {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("Error processing RegisterService request: {}", e);
                    }
                };
            }
            RaftRequest::DeregisterService {
                namespace_id,
                service_id,
            } => {
                match get_app()
                    .discovery_app
                    .manager
                    .deregister_service(&namespace_id, &service_id)
                    .await
                {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("Error processing DeregisterService request: {}", e);
                    }
                };
            }
            RaftRequest::RegisterServiceInstance {
                namespace_id,
                instance,
            } => {
                match get_app()
                    .discovery_app
                    .manager
                    .register_service_instance(&namespace_id, instance)
                    .await
                {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("Error processing RegisterServiceInstance request: {}", e);
                    }
                };
            }
            RaftRequest::DeregisterServiceInstance {
                namespace_id,
                service_id,
                instance_id,
            } => {
                match get_app()
                    .discovery_app
                    .manager
                    .deregister_instance(&namespace_id, &service_id, &instance_id)
                    .await
                {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("Error processing DeregisterServiceInstance request: {}", e);
                    }
                };
            }
            RaftRequest::Heartbeat {
                namespace_id,
                service_id,
                instance_id,
            } => {
                match get_app()
                    .discovery_app
                    .manager
                    .heartbeat(&namespace_id, &service_id, &instance_id)
                    .await
                {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("Error processing Heartbeat request: {}", e);
                    }
                };
            }
        }
    }
}
