//! # 负载均衡组件
//!
//! ## [`RandomLoadBalance`]
//! 随机：从可用服务列表中随机选择一个。
//!
//! ## [`RoundRobinLoadBalance`]
//! 轮询：从服务列表中按顺序轮询选择。
//!
//! ## [`WeightRandomLoadBalance`]
//! 加权随机：从服务列表中按权重进行轮询选择。
//!
//! ## [`WeightRoundRobinLoadBalance`]
//! 加权轮询：从服务列表中按权重进行轮询选择。
//!
//! ## 关于权重
//! 权重可通过服务的元数据进行设置，通常建议权重范围为1-100。
//!
//! # Usage
//! ```rust
//! // 初始化Discovery
//! let _ = init().await;
//!
//! // 创建负载均衡客户端
//! let mut client = LoadBalanceClient::new();
//!
//! // 设置某个服务的负载均衡策略
//! client.set_strategy("your_service_id", LoadBalanceStrategy::Random);
//!
//! // 发起请求
//! let response = client
//!     .get("lb://your_service_id/hello")
//!     .await
//!     .unwrap()
//!     .send()
//!     .await;
//!
//! println!("Response: {:?}", response.unwrap().text().await.unwrap());
//! ```
pub mod client;
mod random;
mod round;
mod weight_random;
mod weight_round;

use crate::{AppDiscovery, Instance};
pub use client::LoadBalanceClient;
pub use random::RandomLoadBalance;
pub use round::RoundRobinLoadBalance;
pub use weight_random::WeightRandomLoadBalance;
pub use weight_round::WeightRoundRobinLoadBalance;

pub trait LoadBalance {
    /// 获取服务实例列表
    fn instances(
        &self,
        service_id: &str,
    ) -> impl Future<Output = Result<Vec<Instance>, LoadBalanceError>> + Send {
        async {
            AppDiscovery::get_instances(service_id)
                .await
                .map_err(|e| LoadBalanceError::GetInstancesError(e.to_string()))
        }
    }

    /// 获取服务实例
    fn get_instance(
        &self,
        service_id: &str,
    ) -> impl Future<Output = Result<Instance, LoadBalanceError>> + Send;
}

#[derive(Debug)]
pub enum LoadBalanceError {
    /// 获取服务实例列表失败
    GetInstancesError(String),
    /// 无可用实例
    NoAvailableInstance(String),
}

impl std::fmt::Display for LoadBalanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadBalanceError::GetInstancesError(e) => write!(f, "Failed to get instances: {}", e),
            LoadBalanceError::NoAvailableInstance(s) => {
                write!(f, "No available instance for service: {}", s)
            }
        }
    }
}

impl std::error::Error for LoadBalanceError {}
