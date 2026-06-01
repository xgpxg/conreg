use crate::auth::UserPrincipal;
use crate::protocol::res::{PageRes, Res};
use crate::system::user;
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};

pub fn routes() -> Vec<rocket::Route> {
    routes![
        login,
        update_password,
        logout,
        get_permissions,
        user_list,
        user_create,
        user_delete,
        user_update,
    ]
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct LoginReq {
    pub(crate) username: String,
    pub(crate) password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct LoginRes {
    pub(crate) username: String,
    pub(crate) token: String,
    pub(crate) permissions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct UpdatePasswordReq {
    pub(crate) password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct CreateUserReq {
    pub(crate) username: String,
    pub(crate) password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct UpdateUserReq {
    pub(crate) username: String,
    pub(crate) password: Option<String>,
    pub(crate) permissions: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct DeleteUserReq {
    pub(crate) username: String,
}

/// 登录
#[post("/login", data = "<req>")]
async fn login(req: Json<LoginReq>) -> Res<LoginRes> {
    match user::login(req.0).await {
        Ok(res) => Res::success(res),
        Err(e) => Res::error(&e.to_string()),
    }
}

/// 修改密码
#[post("/update_password", data = "<req>")]
async fn update_password(req: Json<UpdatePasswordReq>, user: UserPrincipal) -> Res<()> {
    match user::update_password(req.0, user).await {
        Ok(_) => Res::success(()),
        Err(e) => Res::error(&e.to_string()),
    }
}

/// 登出
#[post("/logout")]
async fn logout(user: UserPrincipal) -> Res<()> {
    match user::logout(user).await {
        Ok(_) => Res::success(()),
        Err(e) => Res::error(&e.to_string()),
    }
}

/// 用户列表（分页）
#[get("/user/list?<page_num>&<page_size>")]
async fn user_list(page_num: i32, page_size: i32, user: UserPrincipal) -> Res<PageRes<user::UserInfo>> {
    if !user.is_admin() {
        return Res::error("No permission");
    }
    match user::list_users(page_num, page_size).await {
        Ok(res) => Res::success(PageRes {
            page_num,
            page_size,
            total: res.0,
            list: res.1,
        }),
        Err(e) => Res::error(&e.to_string()),
    }
}

/// 创建用户
#[post("/user/add", data = "<req>")]
async fn user_create(req: Json<CreateUserReq>, user: UserPrincipal) -> Res<()> {
    if !user.is_admin() {
        return Res::error("No permission");
    }
    match user::create_user_and_sync(req.0).await {
        Ok(_) => Res::success(()),
        Err(e) => Res::error(&e.to_string()),
    }
}

/// 删除用户
#[post("/user/delete", data = "<req>")]
async fn user_delete(req: Json<DeleteUserReq>, user: UserPrincipal) -> Res<()> {
    if !user.is_admin() {
        return Res::error("No permission");
    }
    match user::delete_user_and_sync(&req.0.username).await {
        Ok(_) => Res::success(()),
        Err(e) => Res::error(&e.to_string()),
    }
}

/// 修改用户信息
#[post("/user/update", data = "<req>")]
async fn user_update(req: Json<UpdateUserReq>, user: UserPrincipal) -> Res<()> {
    if !user.is_admin() {
        return Res::error("No permission");
    }
    match user::update_user_and_sync(req.0).await {
        Ok(_) => Res::success(()),
        Err(e) => Res::error(&e.to_string()),
    }
}

/// 获取当前用户权限
#[get("/user/permissions")]
async fn get_permissions(user: UserPrincipal) -> Res<Vec<String>> {
    match user::get_user_permissions(&user.username).await {
        Ok(permissions) => Res::success(permissions),
        Err(e) => Res::error(&e.to_string()),
    }
}
