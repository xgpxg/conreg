use crate::Args;
use sqlx::Pool;
use sqlx::sqlite::SqlitePoolOptions;
use std::sync::OnceLock;
use tracing::log;

pub struct DbPool {
    pool: Pool<sqlx::Sqlite>,
}
impl DbPool {
    pub async fn new(args: &Args) -> anyhow::Result<DbPool> {
        let db_url = &format!("sqlite:{}/{}/{}", args.data_dir, "db", "config.db");
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(db_url)
            .await?;
        log::info!("connect to database: {}", db_url);
        // 初始化数据库
        let sql = include_str!("init.sql");
        sqlx::query(sql).execute(&pool).await?;
        log::info!("database loaded");
        Ok(DbPool { pool })
    }
}

static DB_POOL: OnceLock<DbPool> = OnceLock::new();

pub async fn init(args: &Args) -> anyhow::Result<()> {
    let db_pool = DbPool::new(args).await?;
    DB_POOL
        .set(db_pool)
        .map_err(|_| anyhow::anyhow!("database already initialized"))?;
    Ok(())
}

impl DbPool {
    pub fn get() -> &'static Pool<sqlx::Sqlite> {
        &DB_POOL.get().unwrap().pool
    }
}
