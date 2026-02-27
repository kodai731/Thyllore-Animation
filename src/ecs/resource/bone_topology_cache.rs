use std::collections::HashMap;

use crate::animation::BoneId;
use crate::ml::BoneTopologyFeatures;

pub struct BoneTopologyCache {
    pub features: HashMap<BoneId, BoneTopologyFeatures>,
}

impl Default for BoneTopologyCache {
    fn default() -> Self {
        Self {
            features: HashMap::new(),
        }
    }
}

impl BoneTopologyCache {
    pub fn clear(&mut self) {
        self.features.clear();
    }

    pub fn get(&self, bone_id: BoneId) -> BoneTopologyFeatures {
        self.features.get(&bone_id).cloned().unwrap_or_default()
    }
}
