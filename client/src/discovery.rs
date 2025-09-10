use crate::conf::{ClientConfig, ConRegConfig, DiscoveryConfig};
use crate::network::HTTP;
use crate::protocol::Instance;
use crate::protocol::request::{GetInstancesReq, HeartbeatReq, RegisterReq};
use crate::protocol::response::HeartbeatResult;
use dashmap::DashMap;
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
    pub(crate) fn new(config: &ConRegConfig) -> Self {
        Self {
            service_id: config.service_id.clone(),
            client: config.client.clone(),
            config: config.discovery.clone().unwrap(),
        }
    }

    /// 注册服务实例，并返回注册的实例
    ///
    /// 服务注册后不会立即处于可用状态，而是处于`Ready`状态，需要等待注册中心收到一次心跳请求后才会变更为可用状态，
    /// 这样做是因为防止服务实例本身问题导致注册后在很短的时间内发生了异常并结束了进程，从而导致注册了不可用实例，进而影响其他服务的可用性。
    ///
    /// 这种“延时可用”的策略，也会导致服务实例自己会在下一个同步中期到来之前处于不可用状态，但是一般来说，服务本身没必要调用自己的服务实例，
    /// 如果需要调用自己，那么在服务内部通过函数调用可能更合适😏。
    pub(crate) async fn register(&self) -> anyhow::Result<Instance> {
        let req = RegisterReq {
            namespace_id: self.config.namespace.clone(),
            service_id: self.service_id.clone(),
            ip: self.client.address.clone(),
            port: self.client.port,
            meta: self.config.meta.clone(),
        };
        let instance = HTTP
            .post::<Instance>(
                &self
                    .config
                    .server_addr
                    .build_url("/discovery/instance/register")?,
                req,
            )
            .await?;
        log::info!("register instance with service id: {}", self.service_id);
        Ok(instance)
    }

    /// 获取可用服务实例
    ///
    /// 可用服务实例是指实例状态为`UP`的实例
    pub(crate) async fn fetch_instances(&self, service_id: &str) -> anyhow::Result<Vec<Instance>> {
        let req = GetInstancesReq {
            namespace_id: self.config.namespace.clone(),
            service_id: service_id.to_string(),
        };
        HTTP.get::<Vec<Instance>>(
            &self
                .config
                .server_addr
                .build_url("/discovery/instance/available")?,
            req,
        )
        .await
    }

    /// 发送心跳
    ///
    /// 心跳结果目前可能有3种：
    /// - Ok: 成功
    /// - NoInstanceFound: 找不到实例，需要重新注册
    /// - Unknown: 未知结果，可能出现在客户端和服务端版本不兼容时
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
    /// 服务实例缓存
    services: Arc<DashMap<String, Vec<Instance>>>,
    /// 服务发现client，负责与服务注册中心通信
    client: DiscoveryClient,
}

impl Discovery {
    pub(crate) async fn new(client: DiscoveryClient) -> Self {
        let discovery = Discovery {
            services: Arc::new(DashMap::new()),
            client,
        };
        // 启动同步任务
        discovery.start_fetch_task();
        // 启动心跳任务
        discovery.start_heartbeat();
        discovery
    }

    /// 定时从注册中心同步服务实例
    ///
    /// 同步间隔时间：30秒
    fn start_fetch_task(&self) {
        log::info!("start service instances fetch task");
        let client = Arc::new(self.client.clone());
        let services = self.services.clone();
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(Duration::from_secs(30));
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
    ///
    /// 心跳间隔：5秒
    fn start_heartbeat(&self) {
        let client = Arc::new(self.client.clone());
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(Duration::from_secs(5));
            loop {
                interval_timer.tick().await;
                log::debug!("ping");
                match client.heartbeat().await {
                    Ok(res) => match res {
                        HeartbeatResult::Ok => {
                            log::debug!("pong");
                        }
                        // 心跳时发现本实例在注册中心不存在了，尝试重新注册服务
                        HeartbeatResult::NoInstanceFound => {
                            log::warn!("no instance found, try re-register");
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

    /// 获取可用服务实例
    ///
    /// 优先取本地缓存，如果本地缓存不存在，则从注册中心同步
    pub(crate) async fn get_instances(&self, service_id: &str) -> Vec<Instance> {
        match self.services.get(service_id) {
            Some(instances) => instances.clone(),
            None => self.fetch_instances(service_id).await.unwrap_or_else(|e| {
                log::error!("Failed to fetch instances: {}", e);
                vec![]
            }),
        }
    }

    /// 从注册中心中同步可用的服务实例
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
