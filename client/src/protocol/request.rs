use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterReq {
    pub namespace_id: String,
    pub service_id: String,
    pub ip: String,
    pub port: u16,
    pub meta: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetInstancesReq {
    pub namespace_id: String,
    pub service_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatReq {
    pub namespace_id: String,
    pub service_id: String,
    pub instance_id: String,
}
