use crate::protocol::res::Res;
use crate::raft::declare_types::ClientWriteResponse;
use crate::raft::{NodeId, RaftRequest};
use rocket::http::Status;
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use tracing::log;

mod app;
mod manage;
mod raft;

pub fn routes() -> Vec<rocket::Route> {
    routes![
        raft::vote,
        raft::append,
        raft::snapshot,
        manage::init,
        manage::metrics,
        manage::change_membership,
        manage::add_learner,
        app::read,
        app::write,
    ]
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum ForwardRequest {
    RaftRequest(RaftRequest),
    AddLearner(NodeId, String),
    MembershipRequest(BTreeSet<NodeId>),
}

async fn forward_request_to_leader(
    leader_addr: &str,
    request: ForwardRequest,
) -> Result<ClientWriteResponse, Status> {
    let client = reqwest::Client::new();

    let forward_url = match &request {
        ForwardRequest::RaftRequest(_) => {
            format!("http://{}/write", leader_addr)
        }
        ForwardRequest::MembershipRequest(_) => {
            format!("http://{}/change-membership", leader_addr)
        }
        ForwardRequest::AddLearner(_, _) => {
            format!("http://{}/add-learner", leader_addr)
        }
    };
    match client.post(&forward_url).json(&request).send().await {
        Ok(response) => match response.json::<Res<ClientWriteResponse>>().await {
            Ok(result) => {
                if result.is_success() {
                    Ok(result.data.unwrap())
                } else {
                    Err(Status::InternalServerError)
                }
            }
            Err(e) => {
                log::error!("Failed to parse forwarded response: {}", e);
                Err(Status::InternalServerError)
            }
        },
        Err(e) => {
            log::error!("Failed to forward request to leader {}: {}", leader_addr, e);
            Err(Status::ServiceUnavailable)
        }
    }
}

#[macro_export]
macro_rules! handle_raft_error {
    ($e:expr, $forward_request:expr) => {{
        use crate::protocol::res::Res;
        match $e {
            RaftError::APIError(err) => match err {
                // 转发到Leader节点处理
                ClientWriteError::ForwardToLeader(fl) => match fl.leader_node {
                    Some(node) => {
                        log::debug!(
                            "forward to leader {}, leader address: {}",
                            fl.leader_id.unwrap(),
                            node.addr
                        );
                        match forward_request_to_leader(&node.addr, $forward_request).await {
                            Ok(result) => Res::success(result),
                            Err(e) => Res::error(&e.to_string()),
                        }
                    }
                    None => {
                        log::error!("forward to leader error: no leader");
                        Res::error("forward to leader error: no leader")
                    }
                },
                ClientWriteError::ChangeMembershipError(e) => {
                    log::error!("error when change membership: {:?}", e);
                    Res::error(&e.to_string())
                }
            },
            RaftError::Fatal(e) => {
                log::error!("fatal error when write: {:?}", e);
                Res::error(&e.to_string())
            }
        }
    }};
}
