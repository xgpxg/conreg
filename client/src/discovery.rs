use crate::conf::{ClientConfig, ConRegConfig, DiscoveryConfig};
use crate::network::HTTP;
use crate::protocol::Instance;
use crate::protocol::request::{GetInstancesReq, HeartbeatReq, RegisterReq};
use crate::protocol::response::HeartbeatResult;
use dashmap::DashMap;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct DiscoveryClient {
    /// 服务ID
    service_id: String,
    /// 客户端配置
    client: ClientConfig,
    /// 注册中心配置
    config: DiscoveryConfig,
}

impl DiscoveryClient {
    pub fn new(config: &ConRegConfig) -> Self {
        Self {
            service_id: config.service_id.clone(),
            client: config.client.clone(),
            config: config.discovery.clone().unwrap(),
        }
    }

    /// 注册服务实例
    pub async fn register(&self) -> anyhow::Result<()> {
        let req = RegisterReq {
            namespace_id: self.config.namespace.clone(),
            service_id: self.service_id.clone(),
            ip: self.client.address.clone(),
            port: self.client.port,
            meta: HashMap::default(),
        };
        HTTP.post::<Instance>(
            &self
                .config
                .server_addr
                .build_url("/discovery/instance/register")?,
            req,
        )
        .await?;
        Ok(())
    }

    /// 获取服务实例
    pub async fn fetch_instances(&self, service_id: &str) -> anyhow::Result<Vec<Instance>> {
        let req = GetInstancesReq {
            namespace_id: self.config.namespace.clone(),
            service_id: service_id.to_string(),
        };
        HTTP.get::<Vec<Instance>>(
            &self
                .config
                .server_addr
                .build_url("/discovery/instance/list")?,
            req,
        )
        .await
    }

    /// 发送心跳
    async fn heartbeat(&self) -> anyhow::Result<HeartbeatResult> {
        let req = HeartbeatReq {
            namespace_id: self.config.namespace.clone(),
            service_id: self.service_id.to_string(),
            instance_id: self.client.gen_instance_id(),
        };
        HTTP.post::<HeartbeatResult>(
            &self.config.server_addr.build_url("/discovery/heartbeat")?,
            req,
        )
        .await
    }
}

#[derive(Debug)]
pub struct Discovery {
    services: Arc<DashMap<String, Vec<Instance>>>,
    client: DiscoveryClient,
}

impl Discovery {
    pub async fn new(client: DiscoveryClient) -> Self {
        let discovery = Discovery {
            services: Arc::new(DashMap::new()),
            client,
        };
        discovery.start_sync_timer();
        discovery.start_heartbeat();
        discovery
    }

    /// 定时从注册中心同步服务实例
    fn start_sync_timer(&self) {
        log::info!("start service instances fetch timer");
        let client = Arc::new(self.client.clone());
        let mut interval_timer = tokio::time::interval(Duration::from_secs(5));
        let services = self.services.clone();
        tokio::spawn(async move {
            loop {
                interval_timer.tick().await;
                let service_ids: Vec<String> =
                    services.iter().map(|entry| entry.key().clone()).collect();
                for service_id in service_ids {
                    match Self::fetch_instances_(&client, &service_id).await {
                        Ok(instances) => {
                            services.insert(service_id, instances);
                        }
                        Err(e) => {
                            log::error!(
                                "fetch service instance error, service id: {}, error: {}",
                                service_id,
                                e
                            );
                        }
                    }
                }
            }
        });
    }

    /// 开启定时心跳
    fn start_heartbeat(&self) {
        let client = Arc::new(self.client.clone());
        let mut interval_timer = tokio::time::interval(Duration::from_secs(5));
        tokio::spawn(async move {
            loop {
                interval_timer.tick().await;
                log::info!("ping");
                match client.heartbeat().await {
                    Ok(res) => match res {
                        HeartbeatResult::Ok => {
                            log::info!("pong");
                        }
                        // 心跳时发现本实例在注册中心不存在了，尝试重新注册服务
                        HeartbeatResult::NoInstanceFound => {
                            log::info!("no instance found, re-register");
                            if let Err(e) = client.register().await {
                                log::error!("register error:{}", e);
                            }
                        }
                        // 未知结果，可能客户端和服务端版本不匹配
                        HeartbeatResult::Unknown => {
                            log::error!("Unknown heartbeat result");
                        }
                    },
                    Err(e) => {
                        log::error!("heartbeat error: {}", e);
                    }
                }
            }
        });
    }

    /// 获取服务实例
    pub async fn get_instances(&self, service_id: &str) -> Vec<Instance> {
        match self.services.get(service_id) {
            Some(instances) => instances.clone(),
            None => self.fetch_instances(service_id).await.unwrap_or_else(|e| {
                log::error!("Failed to fetch instances: {}", e);
                vec![]
            }),
        }
    }

    /// 从注册中心中同步服务实例
    async fn fetch_instances(&self, service_id: &str) -> anyhow::Result<Vec<Instance>> {
        let instances = self.client.fetch_instances(service_id).await?;
        self.services
            .insert(service_id.to_string(), instances.clone());
        Ok(instances)
    }

    async fn fetch_instances_(
        client: &DiscoveryClient,
        service_id: &str,
    ) -> anyhow::Result<Vec<Instance>> {
        let instances = client.fetch_instances(service_id).await?;
        Ok(instances)
    }
}
