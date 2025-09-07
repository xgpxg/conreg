use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HeartbeatResult {
    /// Ok
    Ok,
    /// 找不到实例，需要重新注册服务实例
    NoInstanceFound,
    /// 未知结果，可能出现在客户端和服务端版本不兼容时
    Unknown,
}

impl Default for HeartbeatResult {
    fn default() -> Self {
        HeartbeatResult::Unknown
    }
}

impl From<String> for HeartbeatResult {
    fn from(s: String) -> Self {
        match s.as_str() {
            "Ok" => HeartbeatResult::Ok,
            "NoInstanceFound" => HeartbeatResult::NoInstanceFound,
            _ => HeartbeatResult::Unknown,
        }
    }
}
