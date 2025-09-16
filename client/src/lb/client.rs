use crate::Instance;
use crate::lb::{
    LoadBalance, RandomLoadBalance, RoundRobinLoadBalance, WeightRandomLoadBalance,
    WeightRoundRobinLoadBalance,
};
use dashmap::DashMap;
use reqwest::{Client, Method, RequestBuilder, Url};
use std::ops::Deref;
use std::str::FromStr;
use std::time::Duration;

/// 负载均衡策略
#[derive(Debug)]
pub enum LoadBalanceStrategy {
    /// 轮询
    RoundRobin(RoundRobinLoadBalance),
    /// 加权轮询
    WeightedRoundRobin(WeightRoundRobinLoadBalance),
    /// 随机
    Random(RandomLoadBalance),
    /// 加权随机
    WeightedRandom(WeightRandomLoadBalance),
}
impl LoadBalanceStrategy {
    pub fn default() -> Self {
        Self::Random(RandomLoadBalance::default())
    }
}

/// 负载均衡客户端
pub struct LoadBalanceClient {
    client: Client,
    strategies: DashMap<String, LoadBalanceStrategy>,
}

impl LoadBalanceClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            strategies: DashMap::new(),
        }
    }

    /// 设置负载策略
    pub fn set_strategy(&mut self, key: impl Into<String>, strategy: LoadBalanceStrategy) {
        self.strategies.insert(key.into(), strategy);
    }

    /// 获取服务实例
    async fn get_instance(&self, key: &str) -> anyhow::Result<Instance> {
        let strategy = self
            .strategies
            .entry(key.to_string())
            .or_insert(LoadBalanceStrategy::default());
        let strategy = strategy.value();
        match strategy {
            LoadBalanceStrategy::RoundRobin(lb) => lb.get_instance(key).await,
            LoadBalanceStrategy::WeightedRoundRobin(lb) => lb.get_instance(key).await,
            LoadBalanceStrategy::Random(lb) => lb.get_instance(key).await,
            LoadBalanceStrategy::WeightedRandom(lb) => lb.get_instance(key).await,
        }
    }

    const LB_PREFIX: &'static str = "lb://";
    const HTTP_PREFIX: &'static str = "http://";
    async fn parse_url(&self, url: &str) -> String {
        if !url.starts_with(Self::LB_PREFIX) {
            return url.to_string();
        }

        let parsed_url = Url::parse(url).unwrap();

        let service_id = parsed_url.host_str().unwrap();
        let domain = parsed_url.domain().unwrap();

        let instance = self.get_instance(service_id).await.unwrap();

        url.replace(
            &format!("{}{}", Self::LB_PREFIX, domain),
            &format!("{}{}:{}", Self::HTTP_PREFIX, instance.ip, instance.port),
        )
    }

    pub async fn get(&self, url: &str) -> RequestBuilder {
        self.client.get(self.parse_url(url).await)
    }

    pub async fn post(&self, url: &str) -> RequestBuilder {
        self.client.post(self.parse_url(url).await)
    }

    pub async fn put(&self, url: &str) -> RequestBuilder {
        self.client.put(self.parse_url(url).await)
    }

    pub async fn delete(&self, url: &str) -> RequestBuilder {
        self.client.delete(self.parse_url(url).await)
    }

    pub async fn patch(&self, url: &str) -> RequestBuilder {
        self.client.patch(self.parse_url(url).await)
    }

    pub async fn head(&self, url: &str) -> RequestBuilder {
        self.client.head(self.parse_url(url).await)
    }

    pub async fn request(&self, method: Method, url: &str) -> RequestBuilder {
        self.client.request(method, self.parse_url(url).await)
    }
}

impl Default for LoadBalanceClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init;
    use serde_yaml::Value;

    #[tokio::test]
    async fn test_load_balance_client() {
        let _ = init().await;
        let mut client = LoadBalanceClient::new();

        client.set_strategy(
            "some_key",
            LoadBalanceStrategy::WeightedRandom(WeightRandomLoadBalance::default()),
        );
        client.set_strategy(
            "some_key",
            LoadBalanceStrategy::RoundRobin(RoundRobinLoadBalance::default()),
        );

        let response = client
                .get("lb://conreg_client-ecdb9f5551f4f00c/api/discovery/instance/available?namespace_id=public&service_id=conreg_client-ecdb9f5551f4f00c")
                .await
                .send()
                .await;
        println!(
            "Response: {:?}",
            response.unwrap().json::<Value>().await.unwrap()
        );
    }
}
