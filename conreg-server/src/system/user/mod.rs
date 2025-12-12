use crate::auth::UserPrincipal;
use crate::cache;
use crate::cache::caches::CacheKey;
use crate::db::DbPool;
use crate::system::api::{LoginReq, LoginRes, UpdatePasswordReq};
use anyhow::bail;
use rocket::serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(sqlx::FromRow, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    /// 用户名
    pub username: String,
    /// 密码
    pub password: String,
}

async fn get_user(username: &str) -> anyhow::Result<Option<User>> {
    let user: Option<User> = sqlx::query_as("select * from user where username = ?")
        .bind(username)
        .fetch_optional(DbPool::get())
        .await?;
    Ok(user)
}
pub(crate) async fn login(req: LoginReq) -> anyhow::Result<LoginRes> {
    let user = get_user(&req.username).await?;
    if user.is_none() {
        bail!("Username or password is incorrect");
    };
    let user = user.unwrap();
    if !bcrypt::verify(req.password, &user.password).unwrap_or(false) {
        bail!("Username or password is incorrect");
    }

    let token = uuid::Uuid::new_v4().to_string();

    let user_principal = UserPrincipal {
        username: user.username.clone(),
        token: token.clone(),
    };
    cache::set_and_sync(
        CacheKey::UserToken(token.clone()).to_string(),
        &user_principal,
        Some(Duration::from_secs(3600 * 24 * 7).as_secs()),
    )
    .await?;

    Ok(LoginRes {
        username: user.username,
        token,
    })
}

pub async fn update_password(req: UpdatePasswordReq, user: UserPrincipal) -> anyhow::Result<()> {
    let user = get_user(&user.username).await?;
    if user.is_none() {
        bail!("User not found");
    }
    let user = user.unwrap();

    let hashed = bcrypt::hash(req.password, bcrypt::DEFAULT_COST)?;
    sqlx::query("update user set password = ? where username = ?")
        .bind(hashed)
        .bind(user.username)
        .execute(DbPool::get())
        .await?;
    Ok(())
}

pub async fn logout(user: UserPrincipal) -> anyhow::Result<()> {
    cache::remove(&CacheKey::UserToken(user.token).to_string()).await?;
    Ok(())
}

mod tests {
    #[test]
    pub fn gen_password() {
        let password = "conreg";
        let hashed = bcrypt::hash(password, bcrypt::DEFAULT_COST).unwrap();
        println!("{}", hashed);
    }
}
