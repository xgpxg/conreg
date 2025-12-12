use crate::Instance;
use crate::lb::{LoadBalance, LoadBalanceError};

#[derive(Debug, Default)]
pub struct RandomLoadBalance;

impl LoadBalance for RandomLoadBalance {
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
        let index = fastrand::usize(0..instances.len());
        Ok(instances[index].clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init;
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
