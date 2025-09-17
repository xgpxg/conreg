use crate::Instance;
use crate::lb::{LoadBalance, LoadBalanceError};
use dashmap::DashMap;

#[derive(Debug, Default)]
pub struct RoundRobinLoadBalance {
    index: DashMap<String, usize>,
}
impl RoundRobinLoadBalance {
    pub fn new() -> Self {
        Self::default()
    }
}

impl LoadBalance for RoundRobinLoadBalance {
    async fn get_instance(&self, service_id: &str) -> Result<Instance, LoadBalanceError> {
        let instances = self.instances(service_id).await?;

        if instances.is_empty() {
            return Err(LoadBalanceError::NoAvailableInstance(
                service_id.to_string(),
            ));
        }
        if instances.len() == 1 {
            return Ok(instances[0].clone());
        }
        let mut index = self.index.entry(service_id.to_string()).or_insert(0);
        *index = (*index + 1) % instances.len();
        Ok(instances[*index].clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init;
    #[tokio::test]
    async fn test_round_robin_balance() {
        let _ = init().await;
        let lb = RoundRobinLoadBalance::default();
        let instances = lb
            .get_instance("conreg_client-ecdb9f5551f4f00c")
            .await
            .unwrap();
        println!("instances: {:?}", instances);
    }
}
