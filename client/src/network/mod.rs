use crate::conf::ServerAddr;
use crate::config::Res;
use anyhow::bail;
use rand::{Rng, rng};
use reqwest::StatusCode;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use std::sync::LazyLock;
use std::time::Duration;

pub struct Network {
    client: reqwest::Client,
}

pub static HTTP: LazyLock<Network> = LazyLock::new(|| {
    let client = reqwest::ClientBuilder::default()
        .connect_timeout(Duration::from_secs(1))
        .read_timeout(Duration::from_secs(60))
        .build()
        .unwrap();
    Network { client }
});

impl Network {
    pub async fn get<T: DeserializeOwned + Debug + Default>(
        &self,
        url: &str,
        query: impl Serialize + Debug,
    ) -> anyhow::Result<T> {
        log::debug!("GET {}, query: {:?}", url, query);
        let response = self.client.get(url).query(&query).send().await?;
        if response.status() != StatusCode::OK {
            bail!("{}", response.text().await?);
        }
        let result = response.json::<Res<T>>().await?;
        if result.code != 0 {
            bail!("{}", result.msg);
        }
        Ok(result.data.unwrap_or(Default::default()))
    }

    pub async fn post<T: DeserializeOwned + Debug + Default>(
        &self,
        url: &str,
        body: impl Serialize + Debug,
    ) -> anyhow::Result<T> {
        log::debug!("POST {}, body: {:?}", url, body);
        let response = self.client.post(url).json(&body).send().await?;
        if response.status() != StatusCode::OK {
            bail!("{}", response.text().await?);
        }
        let result = response.json::<Res<T>>().await?;
        if result.code != 0 {
            bail!("{}", result.msg);
        }
        Ok(result.data.unwrap_or(Default::default()))
    }
}

impl ServerAddr {
    pub fn build_url(&self, path: &str) -> anyhow::Result<String> {
        match self {
            ServerAddr::Single(address) => {
                let url = format!("http://{}{}", address, path);
                Ok(url)
            }
            ServerAddr::Cluster(addresses) => {
                let address = addresses[rng().random_range(0..addresses.len())].clone();
                let url = format!("http://{}{}", address, path);
                Ok(url)
            }
            ServerAddr::Unset => {
                bail!("discovery server address not set");
            }
        }
    }
}
