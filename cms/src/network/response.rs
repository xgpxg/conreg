use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct RaftMetrics {
    pub running_state: RunningState,
    pub id: u64,
    pub current_term: u64,
    pub vote: Vote,
    pub last_log_index: Option<u64>,
    pub last_applied: Option<LogIndex>,
    pub snapshot: Option<Snapshot>,
    pub purged: Option<Purged>,
    pub state: String,
    pub current_leader: Option<u64>,
    pub millis_since_quorum_ack: Option<u64>,
    pub membership_config: MembershipConfig,
    pub replication: Option<BTreeMap<String, Option<Replication>>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RunningState {
    #[serde(rename = "Ok")]
    pub ok: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Vote {
    pub leader_id: LeaderId,
    pub committed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderId {
    pub term: u64,
    pub node_id: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogIndex {
    pub leader_id: LeaderId,
    pub index: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub leader_id: LeaderId,
    pub index: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Purged {}

#[derive(Debug, Serialize, Deserialize)]
pub struct MembershipConfig {
    pub log_id: Option<LogId>,
    pub membership: Membership,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogId {
    pub leader_id: LeaderId,
    pub index: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Membership {
    pub configs: Vec<Vec<u64>>,
    pub nodes: BTreeMap<String, Node>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Node {
    pub addr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Replication {
    pub leader_id: LeaderId,
    pub index: u64,
}
