//! # 负载均衡客户端
//! - 从指定负载策略中获取服务实例
//! - 通过service_id调用服务实例
//!
//! http客户端使用reqwest，通过解析lb://xxx格式的负载协议，负载到具体的服务实例上。
//!
//! 支持负载协议：
//! - `lb`：按照已设置的策略获取服务实例（如未设置则默认使用随机负载）
//! - `lb-r`：按照随机负载策略获取服务实例
//! - `lb-wr`：按照加权随机负载策略获取服务实例
//! - `lb-rr`：按照轮询负载策略获取服务实例
//! - `lb-wrr`：按照加权轮询负载策略获取服务实例

use crate::Instance;
use crate::lb::{
    LoadBalance, LoadBalanceError, RandomLoadBalance, RoundRobinLoadBalance,
    WeightRandomLoadBalance, WeightRoundRobinLoadBalance,
};
use dashmap::DashMap;
use reqwest::{Client, Method, RequestBuilder, Url};
use std::time::Duration;

/// 负载均衡策略
#[derive(Debug)]
pub enum LoadBalanceStrategy {
    /// 轮询
    RoundRobin,
    /// 加权轮询
    WeightedRoundRobin,
    /// 随机
    Random,
    /// 加权随机
    WeightedRandom,
}

impl Default for LoadBalanceStrategy {
    fn default() -> Self {
        Self::Random
    }
}

impl LoadBalanceStrategy {
    pub fn as_schema(&self) -> &str {
        match self {
            LoadBalanceStrategy::RoundRobin => "lb-rr",
            LoadBalanceStrategy::WeightedRoundRobin => "lb-wrr",
            LoadBalanceStrategy::Random => "lb-r",
            LoadBalanceStrategy::WeightedRandom => "lb-wr",
        }
    }
}

/// 负载均衡客户端
pub struct LoadBalanceClient {
    /// HTTP客户端
    client: Client,
    /// 服务负载策略配置，key为service_id，value为负载策略
    strategies: DashMap<String, LoadBalanceStrategy>,
    /// 随机负载均衡
    random_lb: RandomLoadBalance,
    /// 加权随机负载均衡
    weight_random_lb: WeightRandomLoadBalance,
    /// 轮询负载均衡
    round_robin_lb: RoundRobinLoadBalance,
    /// 加权轮询负载均衡
    weight_round_robin_lb: WeightRoundRobinLoadBalance,
}

/// 解析url。
///
/// 将lb://xxx格式的url解析为http://xxx:port的url
///
macro_rules! impl_parse_url {
    ($self:expr, $scheme:expr, $strategy:expr, $url:expr, $parsed_url:expr) => {{
        // 服务ID
        let service_id = $parsed_url.host_str().unwrap();
        let instance = $self.get_instance(service_id, $strategy).await?;
        let res = $url.replace(
            &format!("{}://{}", $scheme, service_id),
            &format!(
                "{}{}:{}",
                LoadBalanceClient::HTTP_PREFIX,
                instance.ip,
                instance.port
            ),
        );
        Ok(res)
    }};
}

impl LoadBalanceClient {
    pub fn new() -> Self {
        Self::new_with_connect_timeout(Duration::from_secs(5))
    }

    pub fn new_with_connect_timeout(timeout: Duration) -> Self {
        let client = Client::builder()
            .connect_timeout(timeout)
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            strategies: Default::default(),
            random_lb: RandomLoadBalance::default(),
            weight_random_lb: WeightRandomLoadBalance::default(),
            round_robin_lb: RoundRobinLoadBalance::default(),
            weight_round_robin_lb: WeightRoundRobinLoadBalance::default(),
        }
    }

    /// 设置服务的负载策略
    ///
    /// - service_id：服务id
    pub fn set_strategy(&mut self, service_id: impl Into<String>, strategy: LoadBalanceStrategy) {
        self.strategies.insert(service_id.into(), strategy);
    }

    /// 获取服务实例
    ///
    /// 优先按传入的负载策略获取实例，如果不指定策略则使用已设置的，如果未设置则使用默认的负载策略
    ///
    /// # Errors
    /// - 当没有可用实例时。
    /// - 当获取实例失败时。
    async fn get_instance(
        &self,
        service_id: &str,
        specify_strategy: Option<LoadBalanceStrategy>,
    ) -> Result<Instance, LoadBalanceError> {
        // 如果指定了strategy，使用指定的strategy获取实例
        if let Some(strategy) = specify_strategy {
            return self.get_instance_(service_id, &strategy).await;
        }

        // 从服务的负载策略中查找并获取实例
        if let Some(strategy) = self.strategies.get(service_id) {
            return self.get_instance_(service_id, &strategy).await;
        }

        // 缓存中没有，即未设置过负载策略，使用默认的策略获取实例
        let default_strategy = LoadBalanceStrategy::default();
        let result = self.get_instance_(service_id, &default_strategy).await;

        // 添加默认的到strategies
        self.strategies
            .insert(service_id.to_string(), default_strategy);

        result
    }

    /// 按负载策略获取服务实例
    /// - service_id：服务id
    /// - strategy：负载策略
    async fn get_instance_(
        &self,
        service_id: &str,
        strategy: &LoadBalanceStrategy,
    ) -> Result<Instance, LoadBalanceError> {
        match strategy {
            LoadBalanceStrategy::Random => self.random_lb.get_instance(service_id).await,
            LoadBalanceStrategy::WeightedRandom => {
                self.weight_random_lb.get_instance(service_id).await
            }
            LoadBalanceStrategy::RoundRobin => self.round_robin_lb.get_instance(service_id).await,
            LoadBalanceStrategy::WeightedRoundRobin => {
                self.weight_round_robin_lb.get_instance(service_id).await
            }
        }
    }
    const HTTP_PREFIX: &'static str = "http://";

    /// 解析url。
    ///
    /// 将lb://xxx格式的url解析为http://xxx:port的url
    ///
    async fn parse_url(&self, url: &str) -> Result<String, LoadBalanceError> {
        let parsed_url = Url::parse(url).unwrap();
        let scheme = parsed_url.scheme();
        match scheme {
            "lb" => {
                impl_parse_url!(self, "lb", None, url, parsed_url)
            }
            "lb-r" => impl_parse_url!(
                self,
                "lb-r",
                Some(LoadBalanceStrategy::Random),
                url,
                parsed_url
            ),
            "lb-wr" => impl_parse_url!(
                self,
                "lb-wr",
                Some(LoadBalanceStrategy::WeightedRandom),
                url,
                parsed_url
            ),
            "lb-rr" => impl_parse_url!(
                self,
                "lb-rr",
                Some(LoadBalanceStrategy::RoundRobin),
                url,
                parsed_url
            ),
            "lb-wrr" => impl_parse_url!(
                self,
                "lb-wrr",
                Some(LoadBalanceStrategy::WeightedRoundRobin),
                url,
                parsed_url
            ),
            _ => Ok(url.to_string()),
        }
    }

    pub async fn get(&self, url: &str) -> Result<RequestBuilder, LoadBalanceError> {
        Ok(self.client.get(self.parse_url(url).await?))
    }

    pub async fn post(&self, url: &str) -> Result<RequestBuilder, LoadBalanceError> {
        Ok(self.client.post(self.parse_url(url).await?))
    }

    pub async fn put(&self, url: &str) -> Result<RequestBuilder, LoadBalanceError> {
        Ok(self.client.put(self.parse_url(url).await?))
    }

    pub async fn delete(&self, url: &str) -> Result<RequestBuilder, LoadBalanceError> {
        Ok(self.client.delete(self.parse_url(url).await?))
    }

    pub async fn patch(&self, url: &str) -> Result<RequestBuilder, LoadBalanceError> {
        Ok(self.client.patch(self.parse_url(url).await?))
    }

    pub async fn head(&self, url: &str) -> Result<RequestBuilder, LoadBalanceError> {
        Ok(self.client.head(self.parse_url(url).await?))
    }

    pub async fn request(
        &self,
        method: Method,
        url: &str,
    ) -> Result<RequestBuilder, LoadBalanceError> {
        Ok(self.client.request(method, self.parse_url(url).await?))
    }

    pub fn get_client(&self) -> &Client {
        &self.client
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
    use crate::conf::{ClientConfigBuilder, ConRegConfigBuilder, DiscoveryConfigBuilder};
    use crate::init_with;

    #[tokio::test]
    async fn test_load_balance_client() {
        let _ = init_client().await;
        let mut client = LoadBalanceClient::new();

        client.set_strategy("test", LoadBalanceStrategy::WeightedRandom);
        client.set_strategy("test", LoadBalanceStrategy::RoundRobin);

        let response = client
            .get("lb://test-server/hello")
            .await
            .unwrap()
            .send()
            .await;
        println!("Response: {:?}", response.unwrap().text().await.unwrap());
    }

    async fn init_client() {
        let config = ConRegConfigBuilder::default()
            .client(ClientConfigBuilder::default().port(8001).build().unwrap())
            .discovery(
                DiscoveryConfigBuilder::default()
                    .server_addr("127.0.0.1:8000")
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();

        init_with(config).await;
    }
}
