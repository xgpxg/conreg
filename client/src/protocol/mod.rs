use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub(crate) mod request;
pub(crate) mod response;

/// 服务示例
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Instance {
    /// 实例ID，由conreg自动生成
    pub id: String,
    /// 服务ID
    pub service_id: String,
    /// 实例IP
    pub ip: String,
    /// 端口
    pub port: u16,
    /// 元数据
    pub meta: HashMap<String, String>,
}
