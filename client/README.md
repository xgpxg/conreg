# Conreg Client

conreg是一个参考了Nacos设计的分布式服务注册和配置中心。详情请看：[conreg](https://github.com/xgpxg/conreg)

conreg-client是conreg的客户端SDK，用于集成到您的服务中和conreg-server通信。

ℹ️ 注意：当前conreg的0.1.x版本仍处于快速迭代中，API在未来可能会发生变化

# 快速开始

## 基本使用

在项目的根目录下添加`bootstrap.yaml`配置文件：

 ```yaml
 conreg:
   # 服务ID
   # 服务ID是服务的唯一标识，同一命名空间下的服务ID不能重复
   service-id: test
   # 客户端配置，这些信息将会作为服务实例的基本信息提交到注册中心
   client:
     # 监听地址
     address: 127.0.0.1
     # 端口
     port: 8000
   # 配置中心配置
   config:
     # 配置中心地址
     server-addr: 127.0.0.1:8000
     # 配置ID
     # 如果多个配置中存在同名配置key，则靠后的配置将会覆盖之前的配置
     config-ids:
       - test.yaml
   # 注册中心配置
   discovery:
     # 注册中心地址
     server-addr:
       - 127.0.0.1:8000
       - 127.0.0.1:8001
       - 127.0.0.1:8002
 ```

然后，在`main`函数中初始化：

 ```rust
 #[tokio::main]
async fn main() {
    // 初始化
    init().await;
    // 获取配置项
    println!("{:?}", AppConfig::get::<String>("name"));
    // 获取服务实例
    let instances = AppDiscovery::get_instances("your_service_id").await.unwrap();
    println!("service instances: {:?}", instances);
}
 ```

## 命名空间

conreg使用命名空间（Namespace）来对配置和服务进行隔离，默认命名空间为`public`。

## 配置中心

从配置中心中加载，并使用这些配置。目前仅支持`yaml`格式的配置。

### 初始化并加载配置

 ```rust
 #[tokio::main]
async fn main() {
    init_with(
        ConRegConfigBuilder::default()
            .config(
                ConfigConfigBuilder::default()
                    .server_addr("127.0.0.1:8000")
                    .namespace("public")
                    .config_ids(vec!["test.yaml".into()])
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap(),
    )
        .await;
    println!("{:?}", AppConfig::get::<String>("name"));
    println!("{:?}", AppConfig::get::<u32>("age"));
}
 ```

### 从配置文件初始化

conreg-client默认从项目根目录下的bootstrap.yaml加载配置初始化配置，就像SpringCloud一样。
以下是`bootstrap.yaml`配置示例

 ```yaml
 conreg:
   config:
     server-addr: 127.0.0.1:8000
     config-ids:
       - your_config.yaml
 ```

然后调用`init`方法即可初始化并获取配置内容。

 ```rust
 #[tokio::main]
async fn main() {
    init().await;
    // 或者指定配置文件路径
    // init_from_file("config.yaml").await;
    println!("{:?}", AppConfig::get::<String>("name"));
    println!("{:?}", AppConfig::get::<u32>("age"));
}
 ```

## 注册中心

用于服务注册和发现。

### 初始化并加载配置

 ```rust
 #[tokio::main]
async fn main() {
    let config = ConRegConfigBuilder::default()
        .service_id("your_service_id")
        .client(
            ClientConfigBuilder::default()
                .address("127.0.0.1")
                .port(8080)
                .build()
                .unwrap(),
        )
        .discovery(
            DiscoveryConfigBuilder::default()
                .server_addr("127.0.0.1:8000")
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let service_id = config.service_id.clone();
    init_with(config).await;
    let instances = AppDiscovery::get_instances(&service_id).await.unwrap();
    println!("service instances: {:?}", instances);
}
 ```

### 从配置文件初始化

默认从`bootstrap.yaml`中加载配置。
以下是示例配置：

 ```yaml
 conreg:
   service-id: your_service_id
   client:
     address: 127.0.0.1
     port: 8000
   discovery:
     server-addr:
       - 127.0.0.1:8000
       - 127.0.0.1:8001
       - 127.0.0.1:8002
 ```

 ```rust
 #[tokio::main]
async fn main() {
    init().await;
    // 或者指定配置文件路径
    // init_from_file("config.yaml").await;
    init_with(config).await;
    let service_id = "your_service_id";
    let instances = AppDiscovery::get_instances(service_id).await.unwrap();
    println!("service instances: {:?}", instances);
}
 ```