# Conreg

使用Rust实现的配置和注册中心，参考了Nacos设计，集配置与服务发现于一体，简单易用，支持集群部署，CP模式，使用Raft保证数据一致性。

配置中心：

- [x] 命名空间隔离
- [x] 配置增删改查
- [x] 一致性同步（Raft）
- [x] 配置历史记录
- [x] 配置恢复
- [ ] 导入/导出配置
- [ ] Web UI

注册中心：

- [x] 命名空间隔离
- [x] 服务注册
- [x] 心跳检测
- [ ] 服务发现
- [ ] Web UI

安全：

- [ ] 登录校验
- [ ] OpenAPI鉴权

客户端SDK（[conreg-client](https://docs.rs/conreg-client)）：

- [x] 配置获取
- [x] 服务注册
- [x] 服务发现
- [ ] 负载均衡

集群管理工具：

- [x] 集群初始化
- [x] 集群扩容
- [x] 集群缩容
- [x] Raft状态监控
- [ ] 集群升级
- [ ] 集群备份

# 整体架构

<img alt="architecture" src="docs/architecture.png" width="500px"/>

# Conreg Server

## 单机部署

- 启动

```shell
conreg-server -p 8000
```

## 集群部署

单节点

- 手动部署

启动多个节点：

```shell
conreg-server -p 8001 -d ./data1 -m cluster -n 1
conreg-server -p 8002 -d ./data2 -m cluster -n 2
conreg-server -p 8003 -d ./data3 -m cluster -n 3
```

初始化：

```shell
curl -X POST http://127.0.0.1:8001/init -d [[1,"127.0.0.1:8000"],[2,"127.0.0.1:8001"],[3,"127.0.0.1:8002"]]
```

- 使用集群管理工具

提供了一个集群管理的CLI工具，用于集群创建、扩容和缩容：[conreg-cmt](https://crates.io/crates/conreg-cmt)

```shell
Usage: conreg-cmt --server <SERVER> <COMMAND>

Commands:
  init         Initialize the cluster
  add-learner  Add a learner node to the cluster
  promote      Promote some learner node to a full member, must call "add-learner" first
  remove-node  Remove a node from the cluster
  status       Get cluster status
  monitor      Monitor cluster status
  help         Print this message or the help of the given subcommand(s)

Options:
  -s, --server <SERVER>  Address of any node in the cluster [default: 127.0.0.1:8000]
  -h, --help             Print help
  -V, --version          Print version
```