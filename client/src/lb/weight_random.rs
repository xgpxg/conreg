use crate::lb::LoadBalance;
use crate::{AppDiscovery, Instance};
use anyhow::bail;

#[derive(Debug,Default)]
pub struct WeightRandomLoadBalance {}


impl WeightRandomLoadBalance {
    pub fn new() -> Self {
        Self::default()
    }
}

impl LoadBalance for WeightRandomLoadBalance {
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

        // 计算总权重
        let total_weight: u64 = instances.iter().map(|instance| instance.get_weight()).sum();

        // 生成0到总权重之间的随机数
        let random_weight: u64 = fastrand::u64(0..total_weight);

        // 根据随机数和权重选择实例
        let mut current_weight = 0;
        for instance in &instances {
            let weight = instance.get_weight();
            current_weight += weight;
            if random_weight < current_weight {
                return Ok(instance.clone());
            }
        }

        // 理论上不会执行到这里
        Ok(instances[0].clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init;
    #[tokio::test]
    async fn test_random_weight_load_balance() {
        let _ = init().await;
        let lb = WeightRandomLoadBalance::default();
        let instances = lb
            .get_instance("conreg_client-ecdb9f5551f4f00c")
            .await
            .unwrap();
        println!("instances: {:?}", instances);
    }
}
