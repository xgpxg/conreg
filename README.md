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

# 整体架构

<img alt="architecture" src="docs/architecture.png" width="500px"/>

# Conreg Server

## 单机部署

- 启动
```shell
conreg-server -p 8000 -d ./data -m standalone
```

## 集群部署

- 在同一台机器上的不同端口启动
```shell
conreg-server -p 8001 -d ./data1 -m cluster -n 1
conreg-server -p 8002 -d ./data2 -m cluster -n 2
conreg-server -p 8003 -d ./data3 -m cluster -n 3
```

- 使用集群管理工具 
TODO