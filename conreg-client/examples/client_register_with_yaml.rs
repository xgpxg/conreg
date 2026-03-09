use conreg_client::AppConfig;
use conreg_client::conf::{
    ClientConfigBuilder, ConRegConfigBuilder, ConfigConfigBuilder, DiscoveryConfigBuilder,
};
use rocket::data::{ByteUnit, Limits};
use rocket::{Config, get};
use std::net::IpAddr;

/// 通过yaml配置文件注册客户端。
///
/// 这里使用 `Rocket` 框架作为示例，你可以使用任何你喜欢的框架，例如 `Actix` 等。
///
/// # Run example
/// ```
/// cargo run --example client_register_with_yaml -F tracing -- --nocapture
/// ```
#[tokio::main]
async fn main() {
    // 初始化
    conreg_client::init_from_file("./conreg-client/examples/bootstrap.yaml").await;

    // 从配置中心获取配置
    tokio::spawn(async move {
        loop {
            // 获取配置
            let name = AppConfig::get::<String>("name");

            println!("name = {:?}", name);

            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    });

    // 启动 Rocket 服务器
    rocket::build()
        .configure(Config {
            port: 8080,
            ..Config::debug_default()
        })
        .mount("/", rocket::routes![test])
        .launch()
        .await
        .unwrap();
}

#[get("/hello")]
fn test() -> &'static str {
    "world"
}
