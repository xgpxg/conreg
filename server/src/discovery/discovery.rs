use anyhow::bail;
use chrono::{DateTime, Local};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInstance {
    /// 服务实例ID
    pub id: String,
    /// 服务ID
    pub service_id: String,
    /// IP
    pub ip: String,
    /// 端口
    pub port: u16,
    /// 实例状态
    status: InstanceStatus,
    /// 元数据
    pub meta: HashMap<String, String>,
    /// 最后一次心跳时间
    #[serde(skip)]
    last_heartbeat: DateTime<Local>,
    /// 丢失心跳的周期数
    #[serde(skip)]
    lost_heartbeats: usize,
}

#[derive(Debug, Clone, PartialOrd, PartialEq, Serialize, Deserialize)]
pub enum InstanceStatus {
    /// 服务就绪
    ///
    /// 服务实例初始化时或从Offline恢复而来，状态为Ready
    /// 此状态的服务实例不会返回给客户端。
    Ready,
    /// 服务正常
    ///
    /// 正常收到服务的心跳请求
    Up,
    /// 不健康
    ///
    /// 当超时未收到心跳时，处于该状态。
    /// 处于该状态的实例将不会返回给客户端。
    /// 当丢失心跳次周期数未超过3次（或者指定次数）后，状态更新为Down。
    Sick(String),
    /// 服务已下线
    ///
    /// 心跳超时导致服务处于临时不可用的状态。
    /// 处于该状态的实例在收到心跳后直接恢复为Up，
    /// 处于Down状态的实例在下个清理任务执行时将从实例列表中移除。
    Down,
    /// 离线
    ///
    /// 该状态仅可由手动操作而来
    /// 该状态的服务实例不会被自动清理
    /// 该状态的服务实例不允许自动恢复
    /// 可由手动调用上线接口来恢复，上线后初始状态未为Ready
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HeartbeatResult {
    /// Ok
    Ok,
    /// 找不到实例，需要重新注册服务实例
    NoInstanceFound,
}

impl ServiceInstance {
    pub fn new(service_id: &str, ip: &str, port: u16, meta: HashMap<String, String>) -> Self {
        ServiceInstance {
            id: Self::generate_id(&ip, port),
            service_id: service_id.to_string(),
            ip: ip.to_string(),
            port,
            status: InstanceStatus::Ready,
            meta,
            last_heartbeat: Local::now(),
            lost_heartbeats: 0,
        }
    }

    pub fn generate_id(ip: &str, port: u16) -> String {
        let digest = md5::compute(format!("{}:{}", ip, port));
        format!("{:x}", digest)
    }

    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = Local::now();
    }

    pub fn is_heartbeat_timeout(&self, timeout: std::time::Duration) -> bool {
        Local::now().signed_duration_since(self.last_heartbeat)
            > chrono::Duration::from_std(timeout).unwrap()
    }

    pub fn is_available(&self) -> bool {
        self.status == InstanceStatus::Up
    }
}

#[derive(Debug)]
pub struct Discovery {
    /// 服务实例
    /// service_id -> Vec<ServiceInstance>
    services: Arc<DashMap<String, Vec<ServiceInstance>>>,
}
impl Clone for Discovery {
    fn clone(&self) -> Self {
        Discovery {
            services: Arc::clone(&self.services),
        }
    }
}

impl Discovery {
    pub fn new() -> Self {
        Discovery {
            services: Arc::new(DashMap::new()),
        }
    }

    /// 注册服务
    ///
    /// 注册一个服务，同时注册0个或多个服务实例，
    /// 包含多个实例时，所有实例的service_id应该保持一致
    pub fn register_service(
        &self,
        service_id: &str,
        instances: Vec<ServiceInstance>,
    ) -> anyhow::Result<Vec<ServiceInstance>> {
        // 校验service_id是否一致
        for instance in &instances {
            if instance.service_id != service_id {
                bail!("The service_id of the registered service instances should be consistent");
            }
        }
        let instances = self
            .services
            .entry(service_id.to_string())
            .or_insert(instances)
            .clone();
        Ok(instances)
    }

    /// 注销服务
    ///
    /// 由客户端触发，在程序停止时尽可能的调用，
    /// 如果客户端没有调用也没关系，按照心跳超时机制处理。
    /// 注销服务后，该服务下的所有服务实例将被删除
    pub fn deregister_service(&self, service_id: &str) -> anyhow::Result<()> {
        self.services.remove(service_id);
        Ok(())
    }

    /// 注册服务实例
    pub fn register_instance(&self, instance: ServiceInstance) -> anyhow::Result<ServiceInstance> {
        let mut instances = self
            .services
            .entry(instance.service_id.clone())
            .or_insert(vec![]);
        // 删除旧实例
        instances.retain(|item| item.id != instance.id);
        // 添加新实例
        instances.push(instance.clone());
        Ok(instance)
    }

    /// 注销服务实例
    pub fn deregister_instance(&self, service_id: &str, instance_id: &str) -> anyhow::Result<()> {
        if let Some(mut service) = self.services.get_mut(service_id) {
            service.retain(|instance| instance.id != instance_id);
        }
        Ok(())
    }

    /// 上线一个服务实例（仅通过手动触发）
    #[allow(unused)]
    pub fn online(&self, service_instance_id: String) -> anyhow::Result<()> {
        unimplemented!()
    }

    /// 下线一个服务实例（仅通过手动触发）
    #[allow(unused)]
    pub fn offline(&self, service_instance_id: String) -> anyhow::Result<()> {
        unimplemented!()
    }

    /// 按服务ID获取服务实例
    pub fn get_service_instances(&self, service_id: &str) -> anyhow::Result<Vec<ServiceInstance>> {
        let list = self
            .services
            .get(service_id)
            .map(|item| item.value().clone())
            .unwrap_or_else(|| vec![]);
        Ok(list)
    }

    /// 按服务ID获取可用服务实例
    pub fn get_available_service_instances(
        &self,
        service_id: &str,
    ) -> anyhow::Result<Vec<ServiceInstance>> {
        let list = self
            .services
            .get(service_id)
            .map(|item| item.value().clone())
            .unwrap_or_else(|| vec![])
            .iter()
            .filter(|item| item.is_available())
            .cloned()
            .collect::<Vec<_>>();
        Ok(list)
    }

    /// 更新服务实例心跳
    pub fn heartbeat(
        &self,
        service_id: &str,
        instance_id: &str,
    ) -> anyhow::Result<HeartbeatResult> {
        if let Some(mut services) = self.services.get_mut(service_id) {
            for instance in services.iter_mut() {
                if instance.id == instance_id {
                    instance.update_heartbeat();
                    instance.status = InstanceStatus::Up;
                    return Ok(HeartbeatResult::Ok);
                }
            }
            Ok(HeartbeatResult::NoInstanceFound)
        } else {
            Ok(HeartbeatResult::NoInstanceFound)
        }
    }

    /// 启动心跳检查
    pub fn start_heartbeat_check_timer(
        &self,
        interval: std::time::Duration,
        timeout: std::time::Duration,
    ) {
        let services = self.services.clone();
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            loop {
                interval_timer.tick().await;
                services.iter_mut().for_each(|mut service| {
                    service.iter_mut().for_each(|instance| {
                        // 超过3个心跳周期超时的，状态更新为Down
                        if instance.lost_heartbeats >= 3 {
                            instance.status = InstanceStatus::Down;
                        } else if instance.is_heartbeat_timeout(timeout) {
                            instance.lost_heartbeats += 1;
                            instance.status = InstanceStatus::Sick(format!(
                                "lost heartbeats({})",
                                instance.lost_heartbeats
                            ))
                        }
                    });
                });
            }
        });
    }

    /// 清理服务实例
    pub fn start_cleanup_timer(&self, interval: std::time::Duration) {
        let services = self.services.clone();
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            loop {
                interval_timer.tick().await;
                // 清理状态为Down的实例
                services.iter_mut().for_each(|mut service| {
                    service.retain(|instance| instance.status != InstanceStatus::Down);
                })
            }
        });
    }

    pub fn services(&self) -> DashMap<String, Vec<ServiceInstance>> {
        self.services.deref().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocket::yansi::Paint;
    use std::time::Duration;
    #[tokio::test]
    async fn test_discovery() {
        let discovery = Discovery::new();
        discovery.start_heartbeat_check_timer(Duration::from_secs(5), Duration::from_secs(10));
        discovery.start_cleanup_timer(Duration::from_secs(15));
        let instance = discovery
            .register_service(
                "test",
                vec![ServiceInstance::new(
                    "test",
                    "127.0.0.1",
                    8080,
                    HashMap::default(),
                )],
            )
            .unwrap();
        println!("instance: {:?}", instance);

        let heartbeat = discovery
            .heartbeat("test", &ServiceInstance::generate_id("127.0.0.1", 8080))
            .unwrap();
        println!("heartbeat: {:?}", heartbeat);

        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;

            let services = discovery.get_service_instances("test").unwrap();
            println!("services: {:?}", discovery.get_service_instances("test"));
        }
    }
}
