use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub(crate) mod request;
pub(crate) mod response;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Instance {
    pub id: String,
    pub service_id: String,
    pub ip: String,
    pub port: u16,
    pub meta: HashMap<String, String>,
}
