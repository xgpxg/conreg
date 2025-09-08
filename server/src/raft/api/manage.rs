use crate::app::get_app;
use crate::handle_raft_error;
use crate::protocol::res::Res;
use crate::raft::api::{ForwardRequest, forward_request_to_leader};
use crate::raft::declare_types::{Node, RaftMetrics};
use crate::raft::{NodeId, TypeConfig};
use openraft::error::{ClientWriteError, RaftError};
use openraft::raft::ClientWriteResponse;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{get, post};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use tracing::log;

/// 初始化集群
///
/// 当请求中没有传集群信息时，默认初始化当前接节点为单实例集群，
/// 后续可通过`add_learner`添加
///
/// 示例：`curl -X POST http://127.0.0.1:8000/init -d []`
#[post("/init", data = "<req>")]
pub async fn init(req: Json<Vec<(NodeId, String)>>) -> Res<String> {
    let app = get_app();
    if app.raft.is_initialized().await.unwrap() {
        return Res::success("Cluster already initialized, no need to reinitialize".to_string());
    }
    let mut nodes = BTreeMap::new();

    // 没有传Nodes，初始化自己为Leader
    if req.0.is_empty() {
        nodes.insert(
            app.id,
            Node {
                addr: app.addr.clone(),
            },
        );
    } else {
        for (id, addr) in req.0.into_iter() {
            nodes.insert(id, Node { addr });
        }
    };
    match app.raft.initialize(nodes).await {
        Ok(_) => Res::success("Cluster initialization completed".to_string()),
        Err(e) => {
            log::error!("{}", e);
            Res::error(&e.to_string())
        }
    }
}

/// 添加一个Learner节点
///
/// Learner节点接收主节点的日志，但不参与投票。
/// 也就是说，Learner能够响应读请求，但是无法将自己转为Leader节点，
/// 要转为Follower节点，需要调用`change-membership`来改变集群成员配置。
///
/// 示例：`curl -X POST http://localhost:8000/add-learner -d '[2,"127.0.0.1:8001"]'`
#[post("/add-learner", data = "<req>")]
pub async fn add_learner(req: Json<(NodeId, String)>) -> Res<ClientWriteResponse<TypeConfig>> {
    let (node_id, api_addr) = req.0;
    let node = Node {
        addr: api_addr.clone(),
    };
    match get_app().raft.add_learner(node_id, node, true).await {
        Ok(response) => Res::success(response),
        Err(e) => handle_raft_error!(e, ForwardRequest::AddLearner(node_id, api_addr)),
    }
}

/// 添加或删除集群节点
///
/// 示例：`curl -X POST http://localhost:8000/change-membership -d '[1,2,3]'`
#[post("/change-membership", data = "<req>")]
pub async fn change_membership(
    req: Json<BTreeSet<NodeId>>,
) -> Res<ClientWriteResponse<TypeConfig>> {
    match get_app().raft.change_membership(req.0.clone(), false).await {
        Ok(res) => Res::success(res),
        Err(e) => handle_raft_error!(e, ForwardRequest::MembershipRequest(req.into_inner())),
    }
}

/// 获取集群信息
///
/// 示例：`curl -X GET http://localhost:8000/metrics`
#[get("/metrics")]
pub async fn metrics() -> Res<RaftMetrics> {
    let metrics = get_app().raft.metrics().borrow().clone();
    Res::success(metrics)
}
