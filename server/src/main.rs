#[macro_use]
extern crate rocket;

use anyhow::Context;
use clap::Parser;
use rocket::Config;
use rocket::data::{ByteUnit, Limits};
use std::fs;
use std::net::IpAddr;
use std::path::Path;
use std::str::FromStr;

mod app;
mod config;
mod db;
mod discovery;
mod event;
mod namespace;
mod protocol;
mod raft;

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

    builder = builder.mount("/", raft::api::routes());
    builder = builder.mount("/config", config::server::api::routes());
    builder = builder.mount("/namespace", namespace::server::api::routes());
    builder = builder.mount("/discovery", discovery::server::api::routes());

    //builder = builder.manage(App::new(&args).await);

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
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,rocket=warn,rocket::response::debug=error".into()),
        )
        .with_level(true)
        .with_ansi(true)
        .with_line_number(true)
        .with_timer(tracing_subscriber::fmt::time::ChronoLocal::new(
            "%Y-%m-%d %H:%M:%S.%.3f".to_string(),
        ))
        .compact() // 避免乱码问题
        .init();
}
