pub mod api;

use crate::Args;
use crate::db::DbPool;
use crate::discovery::discovery::{Discovery, HeartbeatResult, ServiceInstance};
use crate::raft::RaftRequest;
use anyhow::bail;
use chrono::{DateTime, Local};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use sqlx::sqlite::SqliteRow;
use std::collections::HashMap;
use std::ops::Deref;
use std::time::Duration;
use tracing::log;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Service {
    service_id: String,
    namespace_id: String,
    meta: HashMap<String, String>,
    create_time: DateTime<Local>,
    /// 实例数量，包含所有状态的
    total_instances: usize,
}
impl sqlx::FromRow<'_, SqliteRow> for Service {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        let meta_str: Option<String> = row.try_get("meta")?;
        let meta = meta_str
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        Ok(Service {
            service_id: row.try_get("service_id")?,
            namespace_id: row.try_get("namespace_id")?,
            meta,
            create_time: row.try_get("create_time")?,
            total_instances: 0,
        })
    }
}

/// 服务发现管理
///
/// 持久化：服务基本信息需要持久化，使用Raft同步；服务实例信息不需要持久化，但也需要使用Raft同步到集群。
///
/// 在Nacos中，有临时实例和非临时实例之分，临时实例会在服务超时后被清理掉，非临时实例会被保留，
/// 在Conreg中，所有实例都是临时实例，在达到清理条件后都会被清理，所有服务注册由客户端发起，
/// 并向注册中心推送心跳，注册中心不会主动向客户端发起心跳请求，原因如下：
/// 1. 客户端与注册中心之间可能只能单向通信，注册中心无法直接访问客户端，虽然可以建立双向通信通道，但是增加了系统的复杂性。
/// 2. 如果注册中心向客户端发起心跳，需要客户端支持接收心跳请求，需要客户端改造。
/// 3. 当注册的非临时实例过多时，由注册中心主动发起并维护实例心跳时会占用过多资源
///
/// 对于非http服务或者无法集成客户端sdk的服务（如语言不支持），考虑提供一个平台无关的工具，
/// 用这个工具来自定义验证实例是否正常的逻辑，并维护心跳。
#[derive(Debug)]
pub struct DiscoveryManager {
    /// 启动参数
    args: Args,
    /// Http客户端，主要用于提交raft命令
    http_client: reqwest::Client,
    /// 命名空间ID -> 服务发现组件实例
    discoveries: DashMap<String, Discovery>,
}

impl DiscoveryManager {
    pub async fn new(args: &Args) -> anyhow::Result<Self> {
        let http_client = reqwest::ClientBuilder::new()
            .connect_timeout(Duration::from_secs(3))
            .read_timeout(Duration::from_secs(5))
            .build()?;
        Ok(DiscoveryManager {
            args: args.clone(),
            http_client,
            discoveries: DashMap::default(),
        })
    }

    async fn sync(&self, request: RaftRequest) -> anyhow::Result<()> {
        log::debug!("sync discovery request: {:?}", request);
        self.http_client
            .post(format!("http://127.0.0.1:{}/write", self.args.port))
            .json(&request)
            .send()
            .await?;
        log::debug!("sync discovery success");
        Ok(())
    }

    /// 检查discoveries中的命名空间是否存在
    ///
    /// [`DiscoveryManager::discoveries`]会在启动时初始化为空map，使用懒加载的方式，在具体调用时再初始化命名空间对应的discovery
    async fn try_get_discovery(&self, namespace_id: &str) -> anyhow::Result<Discovery> {
        let discovery = self.discoveries.get(namespace_id);
        if discovery.is_none() {
            // 检查库中是否存在
            let namespace = self.get_namespace(namespace_id).await?;
            if namespace.is_none() {
                bail!("namespace [{}] not found", namespace_id);
            }
            let discovery = Discovery::new();
            discovery.start_heartbeat_check_timer(Duration::from_secs(6), Duration::from_secs(5));
            discovery.start_cleanup_timer(Duration::from_secs(10));

            self.discoveries
                .insert(namespace_id.to_string(), discovery.clone());

            return Ok(discovery.clone());
        }
        Ok(discovery.unwrap().deref().clone())
    }

    /// 持久化服务的基本信息到数据库。
    ///
    /// 一个服务被注册后，即使没有实例，也不会自动从注册中心自动移除，需要手动调用API或者从后台删除。
    async fn upsert_service(
        &self,
        namespace_id: &str,
        service_id: &str,
        meta: Option<HashMap<String, String>>,
    ) -> anyhow::Result<()> {
        let count: u64 = sqlx::query_scalar(
            "select count(1) from service where namespace_id = ? and service_id = ?",
        )
        .bind(namespace_id)
        .bind(service_id)
        .fetch_one(DbPool::get())
        .await?;

        let meta_json = meta.map(|m| serde_json::to_string(&m).unwrap_or_default());
        if count == 0 {
            sqlx::query("insert into service (namespace_id, service_id, meta, create_time, update_time) values (?, ?, ?, ?, ?)")
                .bind(namespace_id.to_string())
                .bind(service_id.to_string())
                .bind(meta_json)
                .bind(Local::now())
                .bind(Local::now())
                .execute(DbPool::get())
                .await?;
        } else {
            sqlx::query("update service set meta = ?, update_time = ? where namespace_id = ? and service_id = ?")
                .bind(meta_json)
                .bind(Local::now())
                .bind(namespace_id.to_string())
                .bind(service_id.to_string())
                .execute(DbPool::get())
                .await?;
        }
        Ok(())
    }

    async fn get_namespace(&self, namespace_id: &str) -> anyhow::Result<Option<String>> {
        let id: Option<String> = sqlx::query_scalar("select id from namespace where id = ?")
            .bind(namespace_id)
            .fetch_optional(DbPool::get())
            .await?;

        Ok(id)
    }

    /// 注册服务基本信息（不含实例），并同步到集群
    pub async fn register_service_and_sync(
        &self,
        namespace_id: &str,
        service_id: &str,
        meta: HashMap<String, String>,
    ) -> anyhow::Result<()> {
        let _ = self.try_get_discovery(namespace_id).await?;

        self.sync(RaftRequest::RegisterService {
            service: Service {
                service_id: service_id.to_string(),
                namespace_id: namespace_id.to_string(),
                meta,
                create_time: Local::now(),
                total_instances: 0,
            },
        })
        .await?;
        Ok(())
    }

    /// 注册服务基本信息（不含实例）
    pub async fn register_service(&self, service: Service) -> anyhow::Result<()> {
        self.upsert_service(
            &service.namespace_id,
            &service.service_id,
            Some(service.meta),
        )
        .await?;

        let discovery = self.try_get_discovery(&service.namespace_id).await?;
        discovery.register_service(&service.service_id, vec![])?;

        Ok(())
    }

    /// 获取服务列表
    pub async fn list_services(&self, namespace_id: &str) -> anyhow::Result<Vec<Service>> {
        let mut list: Vec<Service> = sqlx::query_as("select * from service where namespace_id = ?")
            .bind(namespace_id)
            .fetch_all(DbPool::get())
            .await?;
        for service in list.iter_mut() {
            let total_instances = match self.discoveries.get(namespace_id) {
                Some(discovery) => discovery
                    .get_service_instances(service.service_id.as_str())?
                    .len(),
                None => 0,
            };
            service.total_instances = total_instances;
        }
        Ok(list)
    }

    /// 注销服务，并同步到集群
    pub async fn deregister_service_and_sync(
        &self,
        namespace_id: &str,
        service_id: &str,
    ) -> anyhow::Result<()> {
        let _ = self.try_get_discovery(namespace_id).await?;

        self.sync(RaftRequest::DeregisterService {
            namespace_id: namespace_id.to_string(),
            service_id: service_id.to_string(),
        })
        .await?;
        Ok(())
    }

    /// 注销服务
    pub async fn deregister_service(
        &self,
        namespace_id: &str,
        service_id: &str,
    ) -> anyhow::Result<()> {
        let discovery = self.try_get_discovery(namespace_id).await?;
        // 从内存中移除服务以及服务下的所有服务实例
        discovery.deregister_service(service_id)?;
        // 从数据库中移除
        sqlx::query("delete from service where namespace_id = ? and service_id = ?")
            .bind(namespace_id)
            .bind(service_id)
            .execute(DbPool::get())
            .await?;
        Ok(())
    }

    /// 注册服务实例，并同步到集群
    pub async fn register_service_instance_and_sync(
        &self,
        namespace_id: &str,
        instance: ServiceInstance,
    ) -> anyhow::Result<ServiceInstance> {
        let _ = self.try_get_discovery(namespace_id).await?;

        self.sync(RaftRequest::RegisterServiceInstance {
            namespace_id: namespace_id.to_string(),
            instance: instance.clone(),
        })
        .await?;
        Ok(instance)
    }

    /// 注册服务实例
    ///
    /// 如果注册的实例对应的service_id不存在，则自动注册到discovery，并持久化。
    /// 仅服务基本信息需要持久化，服务实例不需要持久化。
    pub async fn register_service_instance(
        &self,
        namespace_id: &str,
        instance: ServiceInstance,
    ) -> anyhow::Result<ServiceInstance> {
        let discovery = self.try_get_discovery(namespace_id).await?;
        // 注册实例，如果service_id不存在则自动注册service
        let instance = discovery.register_instance(instance)?;
        // 持久化，如果已存在则更新
        self.upsert_service(namespace_id, &instance.service_id, None)
            .await?;
        Ok(instance)
    }

    /// 注销服务实例
    pub async fn deregister_instance_and_sync(
        &self,
        namespace_id: &str,
        service_id: &str,
        instance_id: &str,
    ) -> anyhow::Result<()> {
        let _ = self.try_get_discovery(namespace_id).await?;

        self.sync(RaftRequest::DeregisterServiceInstance {
            namespace_id: namespace_id.to_string(),
            service_id: service_id.to_string(),
            instance_id: instance_id.to_string(),
        })
        .await?;
        Ok(())
    }

    pub async fn deregister_instance(
        &self,
        namespace_id: &str,
        service_id: &str,
        instance_id: &str,
    ) -> anyhow::Result<()> {
        let discovery = self.try_get_discovery(namespace_id).await?;
        let instances = discovery.deregister_instance(service_id, instance_id)?;
        Ok(instances)
    }

    /// 获取服务实例
    pub async fn get_instances(
        &self,
        namespace_id: &str,
        service_id: &str,
    ) -> anyhow::Result<Vec<ServiceInstance>> {
        let discovery = self.try_get_discovery(namespace_id).await?;
        let instances = discovery.get_service_instances(service_id)?;
        Ok(instances)
    }

    /// 获取可用服务实例
    pub async fn get_available_instances(
        &self,
        namespace_id: &str,
        service_id: &str,
    ) -> anyhow::Result<Vec<ServiceInstance>> {
        let discovery = self.try_get_discovery(namespace_id).await?;
        let instances = discovery.get_available_services(service_id)?;
        Ok(instances)
    }

    /// 更新心跳，并同步到集群
    pub async fn heartbeat_and_sync(
        &self,
        namespace_id: &str,
        service_id: &str,
        instance_id: &str,
    ) -> anyhow::Result<HeartbeatResult> {
        let _ = self.try_get_discovery(namespace_id).await?;

        let res = self
            .heartbeat(namespace_id, service_id, instance_id)
            .await?;

        self.sync(RaftRequest::Heartbeat {
            namespace_id: namespace_id.to_string(),
            service_id: service_id.to_string(),
            instance_id: instance_id.to_string(),
        })
        .await?;

        Ok(res)
    }
    /// 更新心跳
    pub async fn heartbeat(
        &self,
        namespace_id: &str,
        service_id: &str,
        instance_id: &str,
    ) -> anyhow::Result<HeartbeatResult> {
        let discovery = self.try_get_discovery(namespace_id).await?;
        let hr = discovery.heartbeat(service_id, instance_id)?;
        Ok(hr)
    }
}
