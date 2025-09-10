use crate::app::get_app;
use crate::auth::UserPrincipal;
use crate::discovery::discovery::{HeartbeatResult, ServiceInstance};
use crate::discovery::server::Service;
use crate::protocol::res::{PageRes, Res};
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub fn routes() -> Vec<rocket::Route> {
    routes![
        register_service,
        deregister_service,
        list_service,
        register_instance,
        deregister_instance,
        list_instances,
        available,
        heartbeat
    ]
}

#[derive(Debug, Serialize, Deserialize)]
struct RegisterServiceReq {
    namespace_id: String,
    service_id: String,
    meta: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DeregisterServiceReq {
    namespace_id: String,
    service_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct RegisterServiceInstanceReq {
    namespace_id: String,
    service_id: String,
    ip: String,
    port: u16,
    meta: HashMap<String, String>,
}
impl Into<ServiceInstance> for RegisterServiceInstanceReq {
    fn into(self) -> ServiceInstance {
        ServiceInstance::new(&self.service_id, &self.ip, self.port, self.meta)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct DeregisterServiceInstanceReq {
    namespace_id: String,
    service_id: String,
    instance_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct HeartbeatReq {
    namespace_id: String,
    service_id: String,
    instance_id: String,
}

/// 注册一个空服务，不包含任何实例
///
/// 该接口仅后台调用
#[post("/service/register", data = "<req>")]
async fn register_service(req: Json<RegisterServiceReq>, _user: UserPrincipal) -> Res<()> {
    match get_app()
        .discovery_app
        .manager
        .register_service_and_sync(&req.namespace_id, &req.service_id, req.meta.clone())
        .await
    {
        Ok(_) => Res::success(()),
        Err(e) => Res::error(&e.to_string()),
    }
}

/// 注销服务
///
/// 删除服务以及服务下的所有实例
/// 该接口仅在后台调用
#[post("/service/deregister", data = "<req>")]
async fn deregister_service(req: Json<DeregisterServiceReq>, _user: UserPrincipal) -> Res<()> {
    match get_app()
        .discovery_app
        .manager
        .deregister_service_and_sync(&req.namespace_id, &req.service_id)
        .await
    {
        Ok(_) => Res::success(()),
        Err(e) => Res::error(&e.to_string()),
    }
}

/// 获取服务列表
///
/// 该接口仅在后台调用
#[get("/service/list?<namespace_id>&<page_num>&<page_size>")]
async fn list_service(
    namespace_id: &str,
    page_num: i32,
    page_size: i32,
    _user: UserPrincipal,
) -> Res<PageRes<Service>> {
    match get_app()
        .discovery_app
        .manager
        .list_services(namespace_id, page_num, page_size)
        .await
    {
        Ok(res) => Res::success(PageRes {
            page_num,
            page_size,
            total: res.0,
            list: res.1,
        }),
        Err(e) => Res::error(&e.to_string()),
    }
}

/// 注册一个服务实例
#[post("/instance/register", data = "<req>")]
async fn register_instance(req: Json<RegisterServiceInstanceReq>) -> Res<ServiceInstance> {
    match get_app()
        .discovery_app
        .manager
        .register_service_instance_and_sync(&req.0.namespace_id.clone(), req.0.into())
        .await
    {
        Ok(res) => Res::success(res),
        Err(e) => Res::error(&e.to_string()),
    }
}

/// 注销一个服务实例
#[post("/instance/deregister", data = "<req>")]
async fn deregister_instance(req: Json<DeregisterServiceInstanceReq>) -> Res<()> {
    match get_app()
        .discovery_app
        .manager
        .deregister_instance_and_sync(&req.0.namespace_id, &req.0.service_id, &req.0.instance_id)
        .await
    {
        Ok(res) => Res::success(res),
        Err(e) => Res::error(&e.to_string()),
    }
}

/// 获取服务实例列表
///
/// 该接口仅在后台调用
#[get("/instance/list?<namespace_id>&<service_id>")]
async fn list_instances(
    namespace_id: &str,
    service_id: &str,
    _user: UserPrincipal,
) -> Res<Vec<ServiceInstance>> {
    match get_app()
        .discovery_app
        .manager
        .get_instances(namespace_id, service_id)
        .await
    {
        Ok(instances) => Res::success(instances),
        Err(e) => Res::error(&e.to_string()),
    }
}

/// 获取可用服务实例列表
#[get("/instance/available?<namespace_id>&<service_id>")]
async fn available(namespace_id: &str, service_id: &str) -> Res<Vec<ServiceInstance>> {
    match get_app()
        .discovery_app
        .manager
        .get_available_instances(namespace_id, service_id)
        .await
    {
        Ok(instances) => Res::success(instances),
        Err(e) => Res::error(&e.to_string()),
    }
}

/// 接收客户端心跳
#[post("/heartbeat", data = "<req>")]
async fn heartbeat(req: Json<HeartbeatReq>) -> Res<HeartbeatResult> {
    match get_app()
        .discovery_app
        .manager
        .heartbeat_and_sync(&req.namespace_id, &req.service_id, &req.instance_id)
        .await
    {
        Ok(result) => Res::success(result),
        Err(e) => Res::error(&e.to_string()),
    }
}
