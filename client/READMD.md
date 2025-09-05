# Conreg配置和注册中心客户端

配置中心示例：

```rust
 #[tokio::main]
async fn test_init() {
    // 从默认的bootstrap.yaml初始化
    //init().await;
    // 从指定的文件初始化
    //init_from_file("your_config_file.yml").await;
    // 从指定的配置初始化
    init_with(ConRegConfig {
        service_id: "test".to_string(),
        config: Config {
            server_addr: "127.0.0.1:8000".to_string(),
            namespace: "public".to_string(),
            config_ids: vec!["app.yaml".to_string()],
        },
    })
        .await;
    // 获取配置
    println!("{:?}", AppConfig::get::<String>("name"));
    println!("{:?}", AppConfig::get::<u32>("age"));
    // 绑定配置内容到一个struct
    #[derive(Deserialize)]
    struct MyConfig {
        name: String,
    }
    let my_config = AppConfig::bind::<MyConfig>().unwrap();
    println!("my config, name: {:?}", my_config.name);
}
```
