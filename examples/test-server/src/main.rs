use conreg_client::conf::{
    ClientConfigBuilder, ConRegConfigBuilder, ConfigConfigBuilder, DiscoveryConfigBuilder,
};
use conreg_client::{AppConfig, init_with};

#[tokio::main]
async fn main() {
    let config = ConRegConfigBuilder::default()
        // 服务ID，任意名称，在同一Namespace下唯一
        .service_id("test-server")
        // 客户端信息：将本服务注册到Conreg
        .client(
            ClientConfigBuilder::default()
                .address("127.0.0.1")
                .port(8080)
                .build()
                .unwrap(),
        )
        // 配置中心
        .config(
            ConfigConfigBuilder::default()
                // 配置中心地址，也就是Conreg服务端地址
                .server_addr("127.0.0.1:8000")
                // 命名空间，可选，默认为 public
                .namespace("public")
                // 配置ID，如果有多个，后配置的会覆盖前面的同名配置项
                .config_ids(vec!["test.yaml".into()])
                // 命名空间的认证Token，可选
                .auth_token(Some("token".to_string()))
                .build()
                .unwrap(),
        )
        // 服务发现
        .discovery(
            DiscoveryConfigBuilder::default()
                // 服务发现地址，也就是Conreg服务端地址
                .server_addr(vec!["127.0.0.1:8000"])
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    // 初始化
    init_with(config).await;

    // 从配置中心获取配置

    let h = tokio::spawn(async move {
        loop {
            // 获取配置
            let name = AppConfig::get::<String>("name");

            println!("name = {:?}", name);

            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    });
    let _ = tokio::join!(h);
}
