pub mod api;

use crate::Args;
use crate::db::DbPool;
use crate::raft::RaftRequest;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::log;

/// 命名空间
#[derive(sqlx::FromRow, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Namespace {
    /// 命名空间ID，默认为16位随机字符
    pub id: String,
    /// 命名空间名称
    pub name: String,
    /// 命名空间描述
    pub description: Option<String>,
    /// 创建时间
    pub create_time: DateTime<Local>,
    /// 更新时间
    pub update_time: DateTime<Local>,
}

#[derive(Debug)]
pub struct NamespaceManager {
    /// Http客户端，主要用于同步log到集群
    http_client: reqwest::Client,
    /// 启动参数
    args: Args,
}

impl NamespaceManager {
    pub async fn new(args: &Args) -> anyhow::Result<Self> {
        let http_client = reqwest::ClientBuilder::new()
            .connect_timeout(Duration::from_secs(3))
            .read_timeout(Duration::from_secs(60))
            .build()?;

        Ok(Self {
            http_client,
            args: args.clone(),
        })
    }

    pub async fn get_namespace(&self, id: &str) -> anyhow::Result<Option<Namespace>> {
        let namespace = sqlx::query_as("select * from namespace where id = ?")
            .bind(id)
            .fetch_optional(DbPool::get())
            .await?;
        Ok(namespace)
    }

    pub async fn upsert_namespace_and_sync(
        &self,
        id: &str,
        name: &str,
        description: Option<String>,
    ) -> anyhow::Result<()> {
        let namespace = Namespace {
            id: id.to_string(),
            name: name.to_string(),
            description: description.clone(),
            create_time: Local::now(),
            update_time: Local::now(),
        };
        // 同步数据
        self.sync(RaftRequest::UpsertNamespace { namespace })
            .await?;
        Ok(())
    }

    pub async fn upsert_namespace(&self, namespace: Namespace) -> anyhow::Result<()> {
        let old = self.get_namespace(&namespace.id).await?;
        match old {
            None => {
                self.insert_namespace(&namespace).await?;
            }
            Some(_) => {
                self.update_namespace(&namespace).await?;
            }
        }
        Ok(())
    }
    async fn insert_namespace(&self, namespace: &Namespace) -> anyhow::Result<()> {
        sqlx::query("insert into namespace (id, name, description, create_time, update_time) values (?, ?, ?, ?, ?)")
            .bind(&namespace.id)
            .bind(&namespace.name)
            .bind(&namespace.description)
            .bind(namespace.create_time)
            .bind(namespace.update_time)
            .execute(DbPool::get())
            .await?;
        Ok(())
    }

    async fn update_namespace(&self, namespace: &Namespace) -> anyhow::Result<()> {
        sqlx::query("update namespace set name = ?, description = ?, update_time = ? where id = ?")
            .bind(&namespace.name)
            .bind(&namespace.description)
            .bind(namespace.update_time)
            .bind(&namespace.id)
            .execute(DbPool::get())
            .await?;
        Ok(())
    }

    pub async fn delete_namespace_and_sync(&self, id: &str) -> anyhow::Result<()> {
        self.sync(RaftRequest::DeleteNamespace { id: id.to_string() })
            .await?;
        Ok(())
    }

    pub async fn delete_namespace(&self, id: &str) -> anyhow::Result<()> {
        // 删除配置
        sqlx::query("delete from config where namespace_id = ?")
            .bind(id)
            .execute(DbPool::get())
            .await?;
        sqlx::query("delete from namespace where id = ?")
            .bind(id)
            .execute(DbPool::get())
            .await?;
        Ok(())
    }

    async fn sync(&self, request: RaftRequest) -> anyhow::Result<()> {
        log::info!("sync namespace request: {:?}", request);
        self.http_client
            .post(format!("http://127.0.0.1:{}/write", self.args.port))
            .json(&request)
            .send()
            .await?;
        log::info!("sync namespace success");
        Ok(())
    }

    #[allow(unused)]
    pub async fn get_all_namespace(&self) -> anyhow::Result<Vec<Namespace>> {
        let namespaces = sqlx::query_as(
            r#"
            SELECT * FROM namespace
            "#,
        )
        .fetch_all(DbPool::get())
        .await?;
        Ok(namespaces)
    }

    /// 列表查询（分页）
    async fn list_namespace_with_page(
        &self,
        page_num: i32,
        page_size: i32,
    ) -> anyhow::Result<(u64, Vec<Namespace>)> {
        let total: u64 = sqlx::query_scalar("SELECT COUNT(1) FROM config")
            .fetch_one(DbPool::get())
            .await?;

        let offset = (page_num - 1) * page_size;

        let rows: Vec<Namespace> =
            sqlx::query_as("SELECT * FROM namespace ORDER BY create_time DESC LIMIT ?, ?")
                .bind(offset)
                .bind(page_size)
                .fetch_all(DbPool::get())
                .await?;

        Ok((total, rows))
    }
}
