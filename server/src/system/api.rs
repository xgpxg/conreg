use crate::auth::UserPrincipal;
use crate::protocol::res::Res;
use crate::system::user;
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};

pub fn routes() -> Vec<rocket::Route> {
    routes![login, update_password, logout]
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
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct UpdatePasswordReq {
    pub(crate) password: String,
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
