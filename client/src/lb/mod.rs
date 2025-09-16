mod random;
mod round;
mod weight_round;
mod weight_random;
mod client;

use crate::Instance;

pub trait LoadBalance {
    async fn instances(&self, service_id: &str) -> anyhow::Result<Vec<Instance>>;
    async fn get_instance(&self, service_id: &str) -> anyhow::Result<Instance>;
}
