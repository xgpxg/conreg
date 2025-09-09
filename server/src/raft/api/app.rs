use crate::app::get_app;
use crate::handle_raft_error;
use crate::protocol::res::Res;
use crate::raft::RaftRequest;
use crate::raft::api::{ForwardRequest, forward_request_to_leader};
use crate::raft::declare_types::ClientWriteResponse;
use openraft::error::{ClientWriteError, RaftError};
use rocket::post;
use rocket::serde::json::Json;
use tracing::log;

/// 写入数据
///
/// 仅当集群中超过半数节点存活时，才会写入成功，否则会阻塞，直到有超过半数的可用节点。
#[post("/write", data = "<req>")]
pub async fn write(req: Json<RaftRequest>) -> Res<ClientWriteResponse> {
    match get_app().raft.client_write(req.0.clone()).await {
        Ok(response) => Res::success(response),
        Err(err) => {
            let res: Res<ClientWriteResponse> =
                handle_raft_error!(err, ForwardRequest::RaftRequest(req.0));
            res
        }
    }
}

/// 读取数据
///
/// TODO 考虑提供一个`linearizable`参数，由客户端控制读请求的一致性。
/// 当要求实时一致性时，调用`app.raft.ensure_linearizable()`检查集群是否处于一致状态，
/// 该方法会阻塞，直到集群处于一致状态。
/// 如果不是Leader节点，该方法会返回Err，需要转发到Leader节点。
/// 这样读写都在Leader节点上，可能性能会有损失。
#[get("/read?<key>")]
pub async fn read(key: &str) -> Res<Option<String>> {
    let state_machine = &get_app().state_machine;
    match state_machine.read().await.data.get(key).cloned() {
        Some(value) => Res::success(Some(value)),
        None => Res::success(None),
    }
}
