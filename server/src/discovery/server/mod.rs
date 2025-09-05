mod api;

use std::collections::HashMap;
use crate::discovery::discovery::Discovery;

pub struct Service {}
pub struct DiscoveryManager {
    discoveries: HashMap<String, Discovery>,
}

impl DiscoveryManager {
    pub fn new() -> Self {
        DiscoveryManager {
            discoveries: HashMap::new(),
        }
    }
    pub fn add_discovery(&mut self, namespace_id: &str, discovery: Discovery) {
        self.discoveries.insert(namespace_id.to_string(), discovery);
    }
    pub fn remove_discovery(&mut self, namespace_id: &str) {
        self.discoveries.remove(namespace_id);
    }
    pub fn get_discovery(&self, namespace_id: &str) -> Option<&Discovery> {
        self.discoveries.get(namespace_id)
    }

    // 注册服务实例

    // 获取服务实例

    // 获取可用服务实例

    // 删除服务实例

    // 更新心跳
}
