use crate::app::get_app;
use crate::auth::UserPrincipal;
use crate::namespace::server::Namespace;
use crate::protocol::res::{PageRes, Res};
use crate::system::UserPermission;
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
    is_auth: bool,
    auth_token: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
struct DeleteConfigReq {
    id: String,
}

/// 创建或更新命名空间
/// 如果是新建命名空间，自动给当前用户赋予读写权限
#[post("/upsert", data = "<req>")]
async fn upsert(req: Json<UpsertConfigReq>, user: UserPrincipal) -> Res<()> {
    let manager = &get_app().namespace_app.manager;
    // 先看看是否存在
    let exists = match manager.exists_namespace(&req.id).await {
        Ok(exists) => exists,
        Err(e) => return Res::error(&e.to_string()),
    };
    // 创建或更新命名空间
    match manager
        .upsert_namespace_and_sync(
            &req.id,
            &req.name,
            req.description.clone(),
            req.is_auth,
            req.auth_token.clone(),
        )
        .await
    {
        Ok(is_new) => is_new,
        Err(e) => return Res::error(&e.to_string()),
    };

    // 新建命名空间时，给当前用户自动赋予读写权限
    // 对于admin，不需要处理，它默认拥有所有权限
    if !exists && !user.is_admin() {
        if let Err(e) = crate::system::append_user_permissions_and_sync(
            &user.username,
            vec![format!("rw:ns:{}", &req.id)],
        )
        .await
        {
            return Res::error(&format!(
                "namespace created, but failed to grant permission: {}",
                e
            ));
        }
    } else {
        let has_permission =
            crate::system::check_ns_permission(&user, UserPermission::ReadWriteNs(req.id.clone()))
                .await;
        if !has_permission {
            return Res::error("no permission");
        }
    }

    Res::success(())
}

/// 删除命名空间
/// 删除后自动清理所有用户中与该命名空间相关的权限
#[post("/delete", data = "<req>")]
async fn delete(req: Json<DeleteConfigReq>, _user: UserPrincipal) -> Res<()> {
    if let Err(e) = get_app()
        .namespace_app
        .manager
        .delete_namespace_and_sync(&req.id)
        .await
    {
        return Res::error(&e.to_string());
    }

    // 清理所有用户的该命名空间的权限
    if let Err(e) = crate::system::clean_ns_permissions_and_sync(&req.id).await {
        return Res::error(&e.to_string());
    }

    Res::success(())
}

/// 列表查询（分页）
#[get("/list?<page_num>&<page_size>")]
async fn list(page_num: i32, page_size: i32, user: UserPrincipal) -> Res<PageRes<Namespace>> {
    // 获取权限
    let permissions = match crate::system::get_user_permissions(&user.username).await {
        Ok(permissions) => permissions,
        Err(e) => {
            return Res::error(&e.to_string());
        }
    };
    match get_app()
        .namespace_app
        .manager
        .list_namespace_with_page(page_num, page_size, user.is_admin(), permissions)
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
