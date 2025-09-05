use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ServiceInstance {
    /// 服务实例ID
    id: String,
    /// 服务ID
    service_id: String,
    /// IP
    ip: String,
    /// 端口
    port: u16,
    /// 实例状态
    status: ServiceStatus,
    /// 元数据
    meta: HashMap<String, String>,
    /// 最后一次心跳时间
    last_heartbeat: std::time::Instant,
    /// 丢失心跳的周期数
    lost_heartbeats: usize,
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum ServiceStatus {
    /// 服务就绪
    ///
    /// 在服务注册后，收到第一次心跳前，处于该状态。
    /// 此状态的服务实例不会返回给客户端。
    ///
    /// 当注册中心重启时，从磁盘恢复已注册的服务实例，初始状态为Ready
    Ready,
    /// 服务正常
    ///
    /// 正常收到服务的心跳请求
    Up,
    /// 不健康
    ///
    /// 当超时未收到心跳时，处于该状态。
    /// 处于该状态的且丢失心跳周期数未超过3次（或者指定次数）的服务实例，仍然会返回给客户端。
    /// 当丢失心跳次周期数未超过3次（或者指定次数）后，状态更新为Down，不会再返回给客户端。
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

impl ServiceInstance {
    pub fn new(service_id: String, ip: String, port: u16, meta: HashMap<String, String>) -> Self {
        ServiceInstance {
            id: Self::generate_id(&ip, port),
            service_id,
            ip,
            port,
            status: ServiceStatus::Ready,
            meta,
            last_heartbeat: std::time::Instant::now(),
            lost_heartbeats: 0,
        }
    }

    pub fn generate_id(ip: &str, port: u16) -> String {
        let digest = md5::compute(format!("{}:{}", ip, port));
        format!("{:x}", digest)
    }

    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = std::time::Instant::now();
    }

    pub fn is_heartbeat_timeout(&self, timeout: std::time::Duration) -> bool {
        self.last_heartbeat.elapsed() > timeout
    }
}

pub struct Discovery {
    instances: Arc<DashMap<String, ServiceInstance>>,
}

impl Discovery {
    pub fn new() -> Self {
        Discovery {
            instances: Arc::new(DashMap::new()),
        }
    }

    /// 服务注册
    pub fn register(
        &self,
        service_id: &str,
        ip: &str,
        port: u16,
        meta: HashMap<String, String>,
    ) -> anyhow::Result<ServiceInstance> {
        let instance = ServiceInstance::new(service_id.to_string(), ip.to_string(), port, meta);
        self.instances.insert(instance.id.clone(), instance.clone());
        Ok(instance)
    }

    /// 注销服务
    ///
    /// 由客户端触发，在程序停止时尽可能的调用，
    /// 如果客户端没有调用也没关系，按照心跳超时机制处理。
    pub fn deregister(&self, service_id: String) -> anyhow::Result<()> {
        self.instances
            .retain(|_, instance| instance.service_id == service_id);
        Ok(())
    }

    /// 上线一个服务实例（仅通过手动触发）
    pub fn online(&self, service_instance_id: String) -> anyhow::Result<()> {
        unimplemented!()
    }

    /// 下线一个服务实例（仅通过手动触发）
    pub fn offline(&self, service_instance_id: String) -> anyhow::Result<()> {
        unimplemented!()
    }

    /// 按服务ID获取服务实例
    pub fn get_services(&self, service_id: &str) -> anyhow::Result<Vec<ServiceInstance>> {
        let list = self
            .instances
            .iter()
            .filter(|instance| instance.service_id == service_id)
            .map(|instance| instance.clone())
            .collect::<Vec<_>>();
        Ok(list)
    }

    /// 按服务ID获取可用服务实例
    pub fn get_available_services(
        &self,
        service_id: String,
    ) -> anyhow::Result<Vec<ServiceInstance>> {
        let list = self
            .instances
            .iter()
            .filter(|instance| {
                instance.service_id == service_id && instance.status == ServiceStatus::Up
            })
            .map(|instance| instance.clone())
            .collect::<Vec<_>>();
        Ok(list)
    }

    /// 更新服务实例心跳
    pub fn heartbeat(&self, instance_id: &str) -> anyhow::Result<()> {
        if let Some(mut instance) = self.instances.get_mut(instance_id) {
            instance.update_heartbeat();

            if instance.status == ServiceStatus::Ready || instance.status == ServiceStatus::Down {
                instance.status = ServiceStatus::Up;
            }

            Ok(())
        } else {
            Err(anyhow::anyhow!("Service instance not found"))
        }
    }

    /// 启动心跳检查
    pub async fn start_heartbeat_check_timer(
        &self,
        interval: std::time::Duration,
        timeout: std::time::Duration,
    ) {
        let instances = self.instances.clone();
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            loop {
                interval_timer.tick().await;
                instances.iter_mut().for_each(|mut instance| {
                    // 超过3个心跳周期超时的，状态更新为Down
                    if instance.lost_heartbeats >= 3 {
                        instance.status = ServiceStatus::Down;
                    } else if instance.is_heartbeat_timeout(timeout) {
                        instance.lost_heartbeats += 1;
                        instance.status = ServiceStatus::Sick(format!(
                            "lost heartbeats({})",
                            instance.lost_heartbeats
                        ))
                    }
                });
            }
        });
    }

    /// 清理服务实例
    pub fn start_cleanup_timer(&self, interval: std::time::Duration) {
        let instances = self.instances.clone();
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            loop {
                interval_timer.tick().await;
                // 清理状态为Down的实例
                instances.retain(|_, instance| instance.status != ServiceStatus::Down);
            }
        });
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
        discovery
            .start_heartbeat_check_timer(Duration::from_secs(5), Duration::from_secs(10))
            .await
            .underline();
        discovery.start_cleanup_timer(Duration::from_secs(15));
        let instance = discovery
            .register("test", "127.0.0.1", 8080, HashMap::default())
            .unwrap();
        println!("instance: {:?}", instance);

        discovery
            .heartbeat(&ServiceInstance::generate_id("127.0.0.1", 8080))
            .unwrap();
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;

            let services = discovery.get_services("test").unwrap();
            println!("services: {:?}", discovery.get_services("test"));
        }
    }
}
