use crate::app::get_app;
use crate::namespace::server::Namespace;
use crate::protocol::res::{PageRes, Res};
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};

pub fn routes() -> Vec<rocket::Route> {
    routes![upsert, delete, list]
}

#[derive(Debug, Serialize, Deserialize)]
struct UpsertConfigReq {
    id: String,
    name: String,
    description: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
struct DeleteConfigReq {
    id: String,
}

/// 创建或更新命名空间
#[post("/upsert", data = "<req>")]
pub async fn upsert(req: Json<UpsertConfigReq>) -> Res<()> {
    match get_app()
        .namespace_app
        .manager
        .upsert_namespace_and_sync(&req.id, &req.name, req.description.clone())
        .await
    {
        Ok(_) => Res::success(()),
        Err(e) => Res::error(&e.to_string()),
    }
}

/// 删除命名空间
#[post("/delete", data = "<req>")]
pub async fn delete(req: Json<DeleteConfigReq>) -> Res<()> {
    match get_app()
        .namespace_app
        .manager
        .delete_namespace_and_sync(&req.id)
        .await
    {
        Ok(_) => Res::success(()),
        Err(e) => Res::error(&e.to_string()),
    }
}

/// 列表查询（分页）
#[get("/list?<page_num>&<page_size>")]
pub async fn list(page_num: i32, page_size: i32) -> Res<PageRes<Namespace>> {
    match get_app()
        .namespace_app
        .manager
        .list_namespace_with_page(page_num, page_size)
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
