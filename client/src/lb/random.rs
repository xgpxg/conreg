use crate::lb::LoadBalance;
use crate::{AppDiscovery, Instance};
use anyhow::bail;

#[derive(Debug, Default)]
pub struct RandomLoadBalance;

impl RandomLoadBalance {
    pub fn new() -> Self {
        Self::default()
    }
}

impl LoadBalance for RandomLoadBalance {
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
        let index = fastrand::usize(0..instances.len());
        Ok(instances[index].clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AppConfig, init};
    #[tokio::test]
    async fn test_random_load_balance() {
        let _ = init().await;
        let lb = RandomLoadBalance;
        let instances = lb
            .get_instance("conreg_client-ecdb9f5551f4f00c")
            .await
            .unwrap();
        println!("instances: {:?}", instances);
    }
}
