pub(crate) mod response;

use anyhow::bail;
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::sync::LazyLock;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Res<T> {
    code: i32,
    msg: String,
    data: Option<T>,
}

pub(crate) struct Network {
    client: reqwest::Client,
}

pub(crate) static HTTP: LazyLock<Network> = LazyLock::new(|| {
    let client = reqwest::ClientBuilder::default()
        .connect_timeout(Duration::from_secs(1))
        // Data synchronization may take a long time
        .read_timeout(Duration::from_secs(180))
        .build()
        .unwrap();
    Network { client }
});

impl Network {
    pub async fn get<T: DeserializeOwned + Debug>(
        &self,
        url: impl reqwest::IntoUrl,
        query: impl Serialize + Debug,
    ) -> anyhow::Result<Option<T>> {
        let response = self.client.get(url).query(&query).send().await?;
        if response.status() != StatusCode::OK {
            bail!("{}", response.text().await?);
        }
        let result = response.json::<Res<T>>().await?;
        if result.code != 0 {
            bail!("{}", result.msg);
        }
        Ok(result.data)
    }

    pub async fn post<T: DeserializeOwned + Debug>(
        &self,
        url: impl reqwest::IntoUrl,
        body: impl Serialize + Debug,
    ) -> anyhow::Result<Option<T>> {
        let response = self.client.post(url).json(&body).send().await?;
        if response.status() != StatusCode::OK {
            bail!("{}", response.text().await?);
        }
        let result = response.json::<Res<T>>().await?;
        if result.code != 0 {
            bail!("{}", result.msg);
        }
        Ok(result.data)
    }
}
