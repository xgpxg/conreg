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
pub mod client;
mod random;
mod round;
mod weight_random;
mod weight_round;

use crate::Instance;
pub use client::LoadBalanceClient;
pub use random::RandomLoadBalance;
pub use round::RoundRobinLoadBalance;
pub use weight_random::WeightRandomLoadBalance;
pub use weight_round::WeightRoundRobinLoadBalance;

pub trait LoadBalance {
    fn instances(
        &self,
        service_id: &str,
    ) -> impl Future<Output = anyhow::Result<Vec<Instance>>> + Send;
    fn get_instance(
        &self,
        service_id: &str,
    ) -> impl Future<Output = anyhow::Result<Instance>> + Send;
}
