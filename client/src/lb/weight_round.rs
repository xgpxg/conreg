use crate::lb::LoadBalance;
use crate::{AppDiscovery, Instance};
use anyhow::bail;
use dashmap::DashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Default)]
pub struct WeightRoundRobinLoadBalance {
    /// 每个服务的当前权重索引
    current_weight: DashMap<String, AtomicUsize>,
}

impl WeightRoundRobinLoadBalance {
    pub fn new() -> Self {
        Self::default()
    }
}

impl LoadBalance for WeightRoundRobinLoadBalance {
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

        let mut current_pos = self
            .current_weight
            .entry(service_id.to_string())
            .or_insert_with(|| AtomicUsize::new(0))
            .fetch_add(1, Ordering::Relaxed);

        current_pos %= total_weight as usize;

        // 根据权重选择实例
        let mut current_weight = 0;
        for instance in &instances {
            let weight = instance.get_weight();
            current_weight += weight;
            if current_pos < current_weight as usize {
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
    async fn test_weight_load_balance() {
        let _ = init().await;
        let lb = WeightRoundRobinLoadBalance::default();
        for _ in 0..20 {
            let instances = lb
                .get_instance("conreg_client-ecdb9f5551f4f00c")
                .await
                .unwrap();
            println!("instances: {:?}", instances);
        }
    }
}
