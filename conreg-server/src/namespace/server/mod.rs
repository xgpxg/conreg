pub mod api;

use crate::Args;
use crate::db::DbPool;
use crate::raft::RaftRequest;
use crate::raft::api::raft_write;
use anyhow::bail;
use chrono::{DateTime, Local};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tracing::log;

/// 命名空间
#[derive(sqlx::FromRow, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Namespace {
    /// 命名空间ID
    pub id: String,
    /// 命名空间名称
    pub name: String,
    /// 命名空间描述
    pub description: Option<String>,
    /// 是否需要认证
    pub is_auth: bool,
    /// 认证Token
    pub auth_token: Option<String>,
    /// 创建时间
    pub create_time: DateTime<Local>,
    /// 更新时间
    pub update_time: DateTime<Local>,
}

#[derive(Debug)]
pub struct NamespaceManager {
    /// 命名空间的缓存
    ///
    /// 该缓存是惰性缓存，仅当缓存中没有时才会从数据库中获取。
    /// 在新增、修改和删除时，将从缓存中移除对应的Namespace。
    ///
    /// 注意：移除操作不要在`upsert_namespace_and_sync`中进行，原因如下：
    /// - 增删改的操作需要首先由Raft同步到集群，然后各个节点收到消息后才会进行持久化操作
    /// - 如果在未持久化前移除缓存，则可能在持久化前的读操作重新写入了缓存，导致脏数据
    cache: DashMap<String, Namespace>,
}

impl NamespaceManager {
    pub async fn new(_args: &Args) -> anyhow::Result<Self> {
        Ok(Self {
            cache: DashMap::new(),
        })
    }

    pub async fn get_namespace(&self, id: &str) -> anyhow::Result<Option<Namespace>> {
        if let Some(namespace) = self.cache.get(id) {
            return Ok(Some(namespace.clone()));
        }
        let namespace: Option<Namespace> = sqlx::query_as("select * from namespace where id = ?")
            .bind(id)
            .fetch_optional(DbPool::get())
            .await?;
        if let Some(ref namespace) = namespace {
            self.cache.insert(namespace.id.clone(), namespace.clone());
        }
        Ok(namespace)
    }

    pub async fn upsert_namespace_and_sync(
        &self,
        id: &str,
        name: &str,
        description: Option<String>,
        is_auth: bool,
        auth_token: Option<String>,
    ) -> anyhow::Result<()> {
        let namespace = Namespace {
            id: id.to_string(),
            name: name.to_string(),
            description: description.clone(),
            is_auth,
            auth_token,
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
        sqlx::query("insert into namespace (id, name, description, is_auth, auth_token, create_time, update_time) values (?, ?, ?, ?, ?, ?, ?)")
            .bind(&namespace.id)
            .bind(&namespace.name)
            .bind(&namespace.description)
            .bind(namespace.is_auth)
            .bind(&namespace.auth_token)
            .bind(namespace.create_time)
            .bind(namespace.update_time)
            .execute(DbPool::get())
            .await?;
        self.cache.remove(&namespace.id);
        Ok(())
    }

    async fn update_namespace(&self, namespace: &Namespace) -> anyhow::Result<()> {
        sqlx::query("update namespace set name = ?, description = ?, is_auth = ?, auth_token = ?, update_time = ? where id = ?")
            .bind(&namespace.name)
            .bind(&namespace.description)
            .bind(namespace.is_auth)
            .bind(&namespace.auth_token)
            .bind(namespace.update_time)
            .bind(&namespace.id)
            .execute(DbPool::get())
            .await?;
        self.cache.remove(&namespace.id);
        Ok(())
    }

    pub async fn delete_namespace_and_sync(&self, id: &str) -> anyhow::Result<()> {
        if id == "public" {
            bail!("public is the system's default reserved namespace and cannot be deleted.");
        }
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
        self.cache.remove(id);
        Ok(())
    }

    async fn sync(&self, request: RaftRequest) -> anyhow::Result<()> {
        log::info!("sync namespace request: {:?}", request);
        let res = raft_write(request).await;
        if !res.is_success() {
            log::error!("sync namespace error: {:?}", res.msg);
            bail!("sync namespace error: {}", res.msg);
        }
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
        let total: u64 = sqlx::query_scalar("SELECT COUNT(1) FROM namespace")
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

    /// 验证请求中的Token
    pub async fn auth(&self, namespace_id: &str, auth_token: Option<&str>) -> anyhow::Result<bool> {
        let namespace = self.get_namespace(namespace_id).await?;
        if let Some(namespace) = namespace {
            // 需要认证
            if namespace.is_auth && namespace.auth_token.as_deref() != auth_token {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
