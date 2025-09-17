#[macro_use]
extern crate rocket;

use crate::app::get_app;
use anyhow::Context;
use clap::{Parser, ValueEnum};
use rocket::Config;
use rocket::data::{ByteUnit, Limits};
use rocket::fairing::AdHoc;
use std::collections::BTreeSet;
use std::fs;
use std::net::IpAddr;
use std::path::Path;
use std::str::FromStr;
use tracing::log;

mod app;
mod config;
mod db;
mod discovery;
mod event;
mod namespace;
mod protocol;
mod raft;

mod auth;
mod cache;
mod system;
#[cfg(not(debug_assertions))]
mod web;

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Server listen address
    #[arg(short, long, default_value = "127.0.0.1")]
    address: String,
    /// Server listen port
    #[arg(short, long, default_value_t = 8000)]
    port: u16,
    /// Data directory, storage all data
    #[arg(short, long, default_value = "./data")]
    data_dir: String,
    /// Node id, used for raft cluster, must be unique, and must be greater than 0
    #[arg(short, long, default_value_t = 1)]
    node_id: u64,
    #[arg(short, long, default_value = "standalone")]
    mode: Mode,
    /// Whether to enable configuration cache
    #[arg(long, default_value_t = false)]
    enable_cache_config: bool,
}

#[derive(Parser, Debug, Clone, ValueEnum)]
pub enum Mode {
    /// 单机模式
    #[clap(name = "standalone")]
    Standalone,
    /// 集群模式
    #[clap(name = "cluster")]
    Cluster,
}

#[rocket::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // 初始化日志
    init_log();

    // 初始化目录
    init_dir(&args)?;

    // 初始化ID生成器
    protocol::id::init();

    // 初始化数据库
    db::init(&args).await?;

    // 初始化缓存
    cache::init(&args)?;

    // 初始化app
    app::init().await?;

    start_http_server(&args).await?;

    app::cleanup();

    Ok(())
}

async fn start_http_server(args: &Args) -> anyhow::Result<()> {
    let mut builder = rocket::build().configure(Config {
        address: IpAddr::from_str(&args.address)?,
        port: args.port,
        limits: Limits::default()
            .limit("json", ByteUnit::Mebibyte(5))
            .limit("data-form", ByteUnit::Mebibyte(100))
            .limit("file", ByteUnit::Mebibyte(100)),
        cli_colors: false,
        ..Config::debug_default()
    });

    builder = builder.mount("/api/cluster", raft::api::routes());
    builder = builder.mount("/api/config", config::server::api::routes());
    builder = builder.mount("/api/namespace", namespace::server::api::routes());
    builder = builder.mount("/api/discovery", discovery::server::api::routes());
    builder = builder.mount("/api/system", system::api::routes());

    // 前端
    #[cfg(not(debug_assertions))]
    {
        builder = builder.mount("/", routes![web::web]);
    }

    //builder = builder.manage(App::new(&args).await);

    let args_clone = args.clone();
    builder = builder.attach(AdHoc::on_liftoff("Post-startup tasks", move |_| {
        Box::pin(async move {
            after_http_server_start(&args_clone).await.unwrap();
        })
    }));

    builder.launch().await?;

    Ok(())
}

fn init_dir(args: &Args) -> anyhow::Result<()> {
    // 数据目录
    let data_dir = Path::new(&args.data_dir);
    fs::create_dir_all(data_dir).context("Failed to create data dir")?;

    // 数据库文件
    let db_file = data_dir.join("db").join("conreg.db");
    if !Path::exists(&db_file) {
        fs::create_dir_all(db_file.parent().unwrap())?;
        fs::File::create(db_file)?;
    }

    // raft 日志目录
    let raft_dir = data_dir.join("raft");
    if !Path::exists(&raft_dir) {
        fs::create_dir_all(raft_dir)?;
    }

    Ok(())
}

fn init_log() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "info,rocket=warn,rocket::response::debug=error,rocket::launch=error".into()
            }),
        )
        .with_level(true)
        .with_ansi(true)
        .with_line_number(true)
        .with_timer(tracing_subscriber::fmt::time::ChronoLocal::new(
            "%Y-%m-%d %H:%M:%S.%3f".to_string(),
        ))
        .compact()
        .init();
}

pub(crate) async fn after_http_server_start(args: &Args) -> anyhow::Result<()> {
    match args.mode {
        #[rustfmt::skip]
        Mode::Standalone => {
            let app = get_app();
            let is_initialized = app.raft.is_initialized().await?;
            if !is_initialized {
                app.raft.initialize(BTreeSet::from([args.node_id])).await?;
            }
            let is_initialized = app.raft.is_initialized().await?;
            log::info!("┌─────────────────────────────────────────────────┐");
            log::info!("│               CONREG STANDALONE MODE            │");
            log::info!("├─────────────────────────────────────────────────┤");
            log::info!("│ Address        : {:<30} │", args.address);
            log::info!("│ Port           : {:<30} │", args.port);
            log::info!("│ Node Id        : {:<30} │", args.node_id);
            log::info!("│ Data Dir       : {:<30} │", args.data_dir);
            log::info!("│ Initialized    : {:<30} │", is_initialized);
            log::info!("└─────────────────────────────────────────────────┘");
        }
        Mode::Cluster => {
            let app = get_app();
            let is_initialized = app.raft.is_initialized().await?;
            let leader = app
                .raft
                .current_leader()
                .await
                .map(|id| id.to_string())
                .unwrap_or("No Leader".to_string());
            let nodes_count = app
                .raft
                .metrics()
                .borrow()
                .clone()
                .membership_config
                .membership()
                .nodes()
                .count();
            log::info!("┌─────────────────────────────────────────────────┐");
            log::info!("│               CONREG CLUSTER MODE               │");
            log::info!("├─────────────────────────────────────────────────┤");
            log::info!("│ Address        : {:<30} │", args.address);
            log::info!("│ Port           : {:<30} │", args.port);
            log::info!("│ Node Id        : {:<30} │", args.node_id);
            log::info!("│ Data Dir       : {:<30} │", args.data_dir);
            log::info!("│ Initialized    : {:<30} │", is_initialized);
            log::info!("│ Leader         : {:<30} │", leader);
            log::info!("│ Nodes Count    : {:<30} │", nodes_count);
            log::info!("└─────────────────────────────────────────────────┘");
        }
    }

    Ok(())
}
