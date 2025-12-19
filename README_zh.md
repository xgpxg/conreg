<div align="center">
  <img src="docs/logo.png" style="width:150px" alt="Conreg Logo">

![Release](https://github.com/xgpxg/conreg/actions/workflows/publish.yml/badge.svg)
![GitHub release](https://img.shields.io/github/v/release/xgpxg/conreg?label=Version)
![Crates.io](https://img.shields.io/crates/d/conreg-client?label=conreg-client)
![License](https://img.shields.io/github/license/xgpxg/conreg)

[中文](README_zh.md) | [English](README.md)
</div>

# 简介

Conreg 是一个使用 Rust 实现的配置中心和注册中心，参考了 Nacos 的设计，简单易用，并使用 Raft 算法确保集群节点间的数据一致性。

如果您正在使用 Rust 构建分布式或者微服务应用，Conreg 也许是个不错的选择。

支持平台：

- Ubuntu
- CentOS
- 其他常见 Linux 发行版（我们使用 `musl` 编译，理论上支持所有主流 Linux 发行版）

# 功能特性

配置中心：

- [x] 命名空间隔离
- [x] 配置增删改查操作
- [x] 数据一致性同步（Raft）
- [x] 配置历史记录
- [x] 配置恢复
- [x] 配置导入/导出

注册中心：

- [x] 命名空间隔离
- [x] 服务注册
- [x] 心跳检测
- [x] 实例元数据

安全机制：

- [x] 登录验证
- [x] OpenAPI 认证
- [ ] 配置内容加密

客户端 SDK（[conreg-client](https://docs.rs/conreg-client)）：

- [x] 配置获取
- [x] 服务注册
- [x] 服务发现
- [x] 负载均衡

集群管理工具：

- [x] 集群初始化
- [x] 集群扩容
- [x] 集群缩容
- [x] Raft 状态监控
- [ ] 集群升级
- [ ] 集群备份

Web UI：

- [x] 基础界面
- [ ] 嵌入与集成

# 使用方法

## Conreg 服务端

### 单机部署

```shell
# 下载安装包
curl -L https://github.com/xgpxg/conreg/releases/latest/download/conreg-server.tar.gz | tar -zxvf - -C .

# 启动服务
conreg-server -p 8000
```

浏览器访问地址：http://127.0.0.1:8000

默认用户名和密码：conreg/conreg

### 集群部署

在生产环境中，通常推荐使用集群部署方式。以下示例使用三节点集群：

```shell
# 下载安装包
curl -L https://github.com/xgpxg/conreg/releases/latest/download/conreg-server.tar.gz 

# 解压安装包
tar -zxvf conreg-server.tar.gz -C ./conreg1
tar -zxvf conreg-server.tar.gz -C ./conreg2
tar -zxvf conreg-server.tar.gz -C ./conreg3

# 启动服务
conreg1/conreg-server -p 8001 -d ./conreg1/data1 -m cluster -n 1
conreg2/conreg-server -p 8002 -d ./conreg2/data2 -m cluster -n 2
conreg3/conreg-server -p 8003 -d ./conreg3/data3 -m cluster -n 3

# 初始化集群
curl -X POST http://127.0.0.1:8001/api/cluster/init -d [[1,"127.0.0.1:8001"],[2,"127.0.0.1:8002"],[3,"127.0.0.1:8003"]]
```

您可以使用代理组件（例如 Nginx）来代理集群节点，以便通过浏览器访问后端页面，或者您也可以直接访问集群中的任意节点。

对于集群管理（如初始化、扩容、缩容、监控等），我们提供了一个命令行工具用于集群管理：[conreg-cmt](https://crates.io/crates/conreg-cmt)
，可以方便地使用：

```shell
用法: conreg-cmt --server <SERVER> <COMMAND>

命令:
  init         初始化集群
  add-learner  添加学习者节点到集群
  promote      将某个学习者节点提升为正式成员，必须先调用 "add-learner"
  remove-node  从集群中移除节点
  status       获取集群状态
  monitor      监控集群状态
  help         打印此消息或给定子命令的帮助信息

选项:
  -s, --server <SERVER>  集群中任意节点的地址 [默认: 127.0.0.1:8000]
  -h, --help             打印帮助信息
  -V, --version          打印版本信息
```

## Conreg 客户端

conreg-client 是 Conreg 的客户端 SDK，用于集成到您的 Rust 应用程序中。

例如，可以使用`AppConfig::get('key')`来轻松获取配置内容，并且不需要重启应用。

您可以从 [conreg-client](https://docs.rs/conreg-client) 查看详细文档

# 用户界面

详情请见：[conreg-ui](https://github.com/xgpxg/conreg-ui)

![img.png](docs/ui.png)

# 性能表现

测试机器（Windows WSL）：Intel i7-8750H，6 核 12 线程，16G 内存。

使用单机模式测试了 100 万次请求。

| 操作     | 性能     | 备注    |
|--------|--------|-------|
| 配置写入   | 1.3k/s | -     |
| 配置读取   | 11k/s  | 未启用缓存 |
| 配置读取   | 52k/s  | 启用缓存  |
| 服务实例注册 | 1.1k/s | -     |
| 服务实例查询 | 55k/s  | -     |
| 服务实例心跳 | 1.4k/s | -     |

内存稳定使用量为 55.7M