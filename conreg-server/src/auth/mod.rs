//! Token鉴权

use crate::app::get_app;
use crate::cache;
use crate::cache::caches::CacheKey;
use rocket::Request;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use serde::{Deserialize, Serialize};
use tracing::log;

/// 当前登录用户信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPrincipal {
    /// 用户名
    pub username: String,
    /// token
    #[serde(skip)]
    pub token: String,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for UserPrincipal {
    type Error = &'r str;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let token = req.headers().get_one("Authorization");
        let token = match token {
            Some(token) => match token.trim().split(' ').nth(1) {
                None => return Outcome::Error((Status::Unauthorized, "Need Login")),
                Some(token) => token,
            },
            None => return Outcome::Error((Status::Unauthorized, "Need Login")),
        };

        let mut user =
            match cache::get::<UserPrincipal>(&CacheKey::UserToken(token.to_string()).to_string())
                .await
            {
                Ok(value) => match value {
                    Some(value) => value,
                    None => return Outcome::Error((Status::Unauthorized, "Need Login")),
                },
                Err(e) => {
                    log::error!("get token error: {}", e);
                    return Outcome::Error((Status::Unauthorized, "Need Login"));
                }
            };
        user.token = token.to_string();

        Outcome::Success(user)
    }
}

/// Namespace访问验证
///
/// 目前系统按照Namespace访问隔离，每个Namespace可单独配置访问Token，
/// 在客户端获取配置时，先检查对应的Namespace是否需要认证，如果需要则检查`auth_token`
///
/// 注意：该校验仅在客户端获取配置时使用，其他接口不需要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamespaceAuth;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for NamespaceAuth {
    type Error = &'r str;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // 允许console的登录用户访问
        let is_console = req.headers().get_one("X-Console").is_some();
        if is_console {
            // 尝试解析Token，如果成功则认为是从Console访问的
            let user = req.guard::<UserPrincipal>().await;
            if user.succeeded().is_some() {
                return Outcome::Success(NamespaceAuth);
            }
        }

        let token = req.headers().get_one("X-NS-Token");

        let namespace_id = match req.query_value::<&str>("namespace_id") {
            Some(namespace_id) => match namespace_id {
                Ok(namespace_id) => namespace_id,
                Err(_) => return Outcome::Error((Status::Unauthorized, "No Permission")),
            },
            None => return Outcome::Error((Status::BadRequest, "Namespace ID is required")),
        };

        match get_app()
            .namespace_app
            .manager
            .auth(namespace_id, token)
            .await
        {
            Ok(pass) => {
                if pass {
                    Outcome::Success(NamespaceAuth)
                } else {
                    Outcome::Error((Status::Unauthorized, "No Permission"))
                }
            }
            Err(e) => {
                log::error!("auth error: {}", e);
                Outcome::Error((Status::InternalServerError, "Auth Error"))
            }
        }
    }
}
