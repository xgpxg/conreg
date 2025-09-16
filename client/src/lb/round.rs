use crate::lb::LoadBalance;
use crate::{AppDiscovery, Instance};
use anyhow::bail;
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
    async fn instances(&self, service_id: &str) -> anyhow::Result<Vec<Instance>> {
        AppDiscovery::get_instances(service_id).await
    }

    async fn get_instance(&self, service_id: &str) -> anyhow::Result<Instance> {
        let instances = self.instances(service_id).await?;
        if instances.is_empty() {
            bail!("no instance found with service id: {}", service_id);
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
