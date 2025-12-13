//! # Load Balance Component
//!
//! ## [`RandomLoadBalance`]
//! Random: Select one randomly from the list of available services.
//!
//! ## [`RoundRobinLoadBalance`]
//! Round Robin: Select from the service list in sequential order.
//!
//! ## [`WeightRandomLoadBalance`]
//! Weighted Random: Select from the service list according to weights.
//!
//! ## [`WeightRoundRobinLoadBalance`]
//! Weighted Round Robin: Select from the service list according to weights.
//!
//! ## About Weights
//! Weights can be set through service metadata, typically with a suggested weight range of 1-100.
//!
//! # Usage
//! ```rust
//! // Initialize Discovery
//! let _ = init().await;
//!
//! // Create a load balance client
//! let mut client = LoadBalanceClient::new();
//!
//! // Set the load balancing strategy for a service
//! client.set_strategy("your_service_id", LoadBalanceStrategy::Random);
//!
//! // Make a request
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
    /// Get the list of service instances
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

    /// Get a service instance
    fn get_instance(
        &self,
        service_id: &str,
    ) -> impl Future<Output = Result<Instance, LoadBalanceError>> + Send;
}

#[derive(Debug)]
pub enum LoadBalanceError {
    /// Failed to get the list of service instances
    GetInstancesError(String),
    /// No available instance
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
