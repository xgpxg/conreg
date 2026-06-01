use crate::auth::UserPrincipal;
use crate::cache;
use crate::cache::caches::CacheKey;
use crate::db::DbPool;
use crate::raft::RaftRequest;
use crate::raft::api::raft_write;
use crate::system::UserPermission;
use crate::system::api::{CreateUserReq, LoginReq, LoginRes, UpdatePasswordReq, UpdateUserReq};
use anyhow::bail;
use chrono::{DateTime, Local};
use rocket::serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::log;

#[derive(sqlx::FromRow, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    /// 用户名
    pub username: String,
    /// 密码
    pub password: String,
    /// 权限列表，JSON 格式: ["read:ns1", "write:ns2", "*"]
    pub permissions: Option<String>,
    /// 创建时间
    pub create_time: DateTime<Local>,
}

/// 用户信息（脱敏）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub username: String,
    pub permissions: Option<Vec<String>>,
    pub create_time: DateTime<Local>,
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

    let permissions = user
        .permissions
        .and_then(|p| serde_json::from_str(&p).ok())
        .unwrap_or_default();

    Ok(LoginRes {
        username: user.username,
        token,
        permissions,
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

/// 登出
pub async fn logout(user: UserPrincipal) -> anyhow::Result<()> {
    cache::remove(&CacheKey::UserToken(user.token).to_string()).await?;
    Ok(())
}

/// 查询用户列表（分页）
pub async fn list_users(page_num: i32, page_size: i32) -> anyhow::Result<(u64, Vec<UserInfo>)> {
    // 查询总数
    let total: u64 = sqlx::query_scalar("SELECT COUNT(1) FROM user")
        .fetch_one(DbPool::get())
        .await?;

    // 计算偏移量
    let offset = (page_num - 1) * page_size;

    // 查询分页数据
    let users: Vec<User> = sqlx::query_as(
        "SELECT * FROM user ORDER BY create_time DESC LIMIT ? OFFSET ?",
    )
    .bind(page_size)
    .bind(offset)
    .fetch_all(DbPool::get())
    .await?;

    let user_infos = users
        .into_iter()
        .map(|u| UserInfo {
            username: u.username,
            permissions: u.permissions.and_then(|p| serde_json::from_str(&p).ok()),
            create_time: u.create_time,
        })
        .collect();

    Ok((total, user_infos))
}

/// 创建用户并同步
pub async fn create_user_and_sync(req: CreateUserReq) -> anyhow::Result<()> {
    let exists = get_user(&req.username).await?;
    if exists.is_some() {
        bail!("user already exists");
    }
    let hashed = bcrypt::hash(req.password, bcrypt::DEFAULT_COST)?;

    sync(RaftRequest::CreateUser {
        username: req.username,
        password: hashed,
    })
    .await?;
    Ok(())
}
/// 创建用户
/// 注意：仅由raft调用
///
/// 新用户默认有`public`命名空间的读写权限
pub async fn create_user(username: &str, password: &str) -> anyhow::Result<()> {
    let exists = get_user(username).await?;
    if exists.is_some() {
        bail!("user already exists");
    }
    let now = chrono::Utc::now();
    sqlx::query(
        "insert into user (username, password, permissions, create_time) values (?, ?, ?, ?)",
    )
    .bind(username)
    .bind(password)
    .bind(serde_json::to_string(&vec![
        UserPermission::ReadWritePublicNs.to_string(),
    ])?)
    .bind(now)
    .execute(DbPool::get())
    .await?;
    Ok(())
}
pub async fn update_user_and_sync(req: UpdateUserReq) -> anyhow::Result<()> {
    let username = &req.username;
    if username == UserPrincipal::ADMIN_USERNAME {
        bail!("conreg is a built-in system user and cannot be updated");
    }

    let user = get_user(username).await?;
    if user.is_none() {
        bail!("user not found");
    }

    let update = RaftRequest::UpdateUser {
        username: username.into(),
        password: if let Some(password) = req.password {
            let hashed = bcrypt::hash(password, bcrypt::DEFAULT_COST)?;
            Some(hashed)
        } else {
            None
        },
        permissions: req.permissions,
    };

    sync(update).await?;
    Ok(())
}
/// 更新用户
/// 注意：仅由raft调用
pub async fn update_user(
    username: &str,
    password: Option<String>,
    permissions: Option<Vec<String>>,
) -> anyhow::Result<()> {
    if let Some(password) = password {
        sqlx::query("update user set password = ? where username = ?")
            .bind(password)
            .bind(username)
            .execute(DbPool::get())
            .await?;
    }
    if let Some(permissions) = permissions {
        let perm_json = serde_json::to_string(&permissions)?;
        sqlx::query("update user set permissions = ? where username = ?")
            .bind(perm_json)
            .bind(username)
            .execute(DbPool::get())
            .await?;
    }
    Ok(())
}

/// 删除用户
pub async fn delete_user_and_sync(username: &str) -> anyhow::Result<()> {
    if username == UserPrincipal::ADMIN_USERNAME {
        bail!("conreg is a built-in system user and cannot be deleted");
    }
    let user = get_user(username).await?;
    if user.is_none() {
        bail!("user not found");
    }
    sync(RaftRequest::DeleteUser {
        username: username.into(),
    })
    .await?;
    Ok(())
}

/// 删除用户
/// 注意：仅由raft调用
pub async fn delete_user(username: &str) -> anyhow::Result<()> {
    sqlx::query("delete from user where username = ?")
        .bind(username)
        .execute(DbPool::get())
        .await?;
    Ok(())
}

/// 追加用户权限
/// 如果权限已存在则跳过，通过 Raft 同步到集群
pub async fn append_user_permissions_and_sync(
    username: &str,
    new_permissions: Vec<String>,
) -> anyhow::Result<()> {
    let mut perms = get_user_permissions(username).await?;
    for p in new_permissions {
        if !perms.contains(&p) {
            perms.push(p);
        }
    }
    sync(RaftRequest::UpdateUser {
        username: username.into(),
        password: None,
        permissions: Some(perms),
    })
    .await?;
    Ok(())
}

/// 清理所有用户中与指定命名空间相关的权限
/// 删除命名空间时调用，通过 Raft 同步到集群
pub async fn clean_ns_permissions_and_sync(namespace_id: &str) -> anyhow::Result<()> {
    let users: Vec<User> = sqlx::query_as("select * from user")
        .fetch_all(DbPool::get())
        .await?;

    let ns_permissions = [
        format!("r:ns:{}", namespace_id),
        format!("w:ns:{}", namespace_id),
        format!("rw:ns:{}", namespace_id),
    ];

    for user in users {
        let mut perms: Vec<String> = match user.permissions {
            Some(ref p) => serde_json::from_str(p).unwrap_or_default(),
            None => vec![],
        };

        let original_len = perms.len();
        perms.retain(|p| !ns_permissions.iter().any(|prefix| p == prefix));

        if perms.len() != original_len {
            sync(RaftRequest::UpdateUser {
                username: user.username,
                password: None,
                permissions: Some(perms),
            })
            .await?;
        }
    }
    Ok(())
}

/// 获取用户权限列表
pub async fn get_user_permissions(username: &str) -> anyhow::Result<Vec<String>> {
    let user: Option<User> = sqlx::query_as("select * from user where username = ?")
        .bind(username)
        .fetch_optional(DbPool::get())
        .await?;

    let permissions = match user {
        Some(u) => match u.permissions {
            Some(perm_json) => serde_json::from_str::<Vec<String>>(&perm_json).unwrap_or_default(),
            None => vec![],
        },
        None => vec![],
    };

    log::debug!("get_user_permissions: {} -> {:?}", username, permissions);

    Ok(permissions)
}

async fn sync(request: RaftRequest) -> anyhow::Result<()> {
    log::debug!("sync user info request: {:?}", request);
    let res = raft_write(request).await;
    if !res.is_success() {
        log::error!("sync user info error: {:?}", res.msg);
        bail!("sync user info error: {}", res.msg);
    }
    log::debug!("sync user info success");
    Ok(())
}

/// 检查用户是否对指定命名空间有指定权限
pub async fn check_ns_permission(user: &UserPrincipal, permission: UserPermission) -> bool {
    if user.is_admin() {
        return true;
    }
    match get_user_permissions(&user.username).await {
        Ok(perms) => {
            let p = permission.to_string();
            for perm in &perms {
                if perm == &p {
                    return true;
                }
            }
            false
        }
        Err(e) => {
            log::error!("check permission error: {}", e);
            false
        }
    }
}

mod tests {
    #[test]
    pub fn gen_password() {
        let password = "conreg";
        let hashed = bcrypt::hash(password, bcrypt::DEFAULT_COST).unwrap();
        println!("{}", hashed);
    }
}
