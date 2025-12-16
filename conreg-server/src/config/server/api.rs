use crate::app::get_app;
use crate::auth::UserPrincipal;
use crate::config::server::ConfigEntry;
use crate::protocol::res::{PageRes, Res};
use rocket::form::Form;
use rocket::fs::TempFile;
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};
use tracing::log;

pub fn routes() -> Vec<rocket::Route> {
    routes![
        upsert,
        get,
        delete,
        recover,
        list,
        list_history,
        watch,
        export,
        import
    ]
}

/// 创建或更新配置
#[derive(Debug, Serialize, Deserialize)]
struct UpsertConfigReq {
    namespace_id: String,
    id: String,
    content: String,
    description: Option<String>,
    format: String,
}

/// 删除配置
#[derive(Debug, Serialize, Deserialize)]
struct DeleteConfigReq {
    namespace_id: String,
    id: String,
}

/// 恢复配置
#[derive(Debug, Serialize, Deserialize)]
struct RecoverConfigReq {
    id_: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct ExportConfigReq {
    namespace_id: String,
    ids: Vec<String>,
    is_all: bool,
}

#[derive(Debug, FromForm)]
struct ImportConfigReq<'a> {
    namespace_id: String,
    file: TempFile<'a>,
    is_overwrite: bool,
}
/// 创建或更新配置
///
/// 该接口仅在后台调用
#[post("/upsert", data = "<req>")]
async fn upsert(req: Json<UpsertConfigReq>, _user: UserPrincipal) -> Res<()> {
    match get_app()
        .config_app
        .manager
        .upsert_config_and_sync(
            &req.namespace_id,
            &req.id,
            &req.content,
            req.description.clone(),
            &req.format,
        )
        .await
    {
        Ok(_) => Res::success(()),
        Err(e) => Res::error(&e.to_string()),
    }
}

/// 获取配置
#[get("/get?<namespace_id>&<id>")]
async fn get(namespace_id: &str, id: &str) -> Res<Option<ConfigEntry>> {
    match get_app()
        .config_app
        .manager
        .get_config(namespace_id, id)
        .await
    {
        Ok(entry) => Res::success(entry),
        Err(e) => Res::error(&e.to_string()),
    }
}

/// 删除配置
///
/// 该接口仅在后台调用
#[post("/delete", data = "<req>")]
async fn delete(req: Json<DeleteConfigReq>, _user: UserPrincipal) -> Res<()> {
    match get_app()
        .config_app
        .manager
        .delete_config_and_sync(&req.namespace_id, &req.id)
        .await
    {
        Ok(_) => Res::success(()),
        Err(e) => Res::error(&e.to_string()),
    }
}

/// 恢复配置
///
/// 该接口仅在后台调用
#[post("/recover", data = "<req>")]
async fn recover(req: Json<RecoverConfigReq>, _user: UserPrincipal) -> Res<()> {
    match get_app().config_app.manager.recovery(req.id_).await {
        Ok(_) => Res::success(()),
        Err(e) => Res::error(&e.to_string()),
    }
}

/// 获取配置列表（分页）
///
/// 该接口仅在后台调用
#[get("/list?<namespace_id>&<page_num>&<page_size>&<filter_text>")]
async fn list(
    namespace_id: &str,
    page_num: i32,
    page_size: i32,
    filter_text: Option<String>,
    _user: UserPrincipal,
) -> Res<PageRes<ConfigEntry>> {
    match get_app()
        .config_app
        .manager
        .list_configs_with_page(namespace_id, page_num, page_size, filter_text)
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

/// 获取配置历史列表
///
/// 该接口仅在后台调用
#[get("/histories?<namespace_id>&<id>&<page_num>&<page_size>")]
async fn list_history(
    namespace_id: &str,
    id: &str,
    page_num: i32,
    page_size: i32,
    _user: UserPrincipal,
) -> Res<PageRes<ConfigEntry>> {
    match get_app()
        .config_app
        .manager
        .list_config_history_with_page(namespace_id, id, page_num, page_size)
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

/// 监听配置变化。
/// 返回值不为None时，表示配置有变化，由客户端调用`config/get`接口重新拉取配置
/// 客户端也应该定时从`config/get`拉取配置，作为补偿操作。
#[get("/watch?<namespace_id>")]
async fn watch(namespace_id: &str) -> Res<Option<String>> {
    let mut receiver = get_app().config_app.manager.sender.subscribe();
    // 客户端超时时间为30秒，这里设置为29秒，留1秒防止客户端超时报错。
    let res = tokio::time::timeout(std::time::Duration::from_secs(29), async {
        match receiver.recv().await {
            Ok(event) => {
                if event.namespace_id == namespace_id {
                    log::info!("config changed, namespace id: {}", event.namespace_id);
                    Res::success(Some(event.config_id))
                } else {
                    Res::success(None)
                }
            }
            Err(_) => Res::success(None),
        }
    })
    .await;
    res.unwrap_or_else(|_| Res::success(None))
}

/// 导出配置
///
/// 支持导出命名空间下选中的配置或者全部配置
#[post("/export", data = "<req>")]
async fn export(
    req: Json<ExportConfigReq>,
    _user: UserPrincipal,
) -> Result<Vec<u8>, rocket::http::Status> {
    let req = req.into_inner();
    let namespace_id = req.namespace_id;
    let ids = req.ids;
    let is_all = req.is_all;
    match get_app()
        .config_app
        .manager
        .export(&namespace_id, ids, is_all)
        .await
    {
        Ok(res) => Ok(res),
        Err(e) => {
            log::error!("export config error: {}", e);
            Err(rocket::http::Status::InternalServerError)
        }
    }
}

/// 导入配置
#[post("/import", data = "<req>")]
async fn import(req: Form<ImportConfigReq<'_>>, _user: UserPrincipal) -> Res<()> {
    let req = req.into_inner();
    match get_app()
        .config_app
        .manager
        .import(&req.namespace_id, req.file, req.is_overwrite)
        .await
    {
        Ok(_) => Res::success(()),
        Err(e) => Res::error(&e.to_string()),
    }
}
