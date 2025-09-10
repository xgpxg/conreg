//! Token鉴权

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
