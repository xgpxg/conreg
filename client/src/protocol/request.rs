use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GetConfigReq {
    pub(crate) namespace_id: String,
    pub(crate) id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct WatchConfigChangeReq {
    pub(crate) namespace_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RegisterReq {
    pub(crate) namespace_id: String,
    pub(crate) service_id: String,
    pub(crate) ip: String,
    pub(crate) port: u16,
    pub(crate) meta: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GetInstancesReq {
    pub(crate) namespace_id: String,
    pub(crate) service_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct HeartbeatReq {
    pub(crate) namespace_id: String,
    pub(crate) service_id: String,
    pub(crate) instance_id: String,
}
