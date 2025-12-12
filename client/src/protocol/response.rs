use serde::{Deserialize, Serialize};

/// 响应结果
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Res<T> {
    pub code: i32,
    pub msg: String,
    pub data: Option<T>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) enum HeartbeatResult {
    /// Ok
    Ok,
    /// 找不到实例，需要重新注册服务实例
    NoInstanceFound,
    /// 未知结果，可能出现在客户端和服务端版本不兼容时
    #[default]
    Unknown,
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
