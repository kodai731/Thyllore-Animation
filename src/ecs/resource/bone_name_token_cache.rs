use std::collections::HashMap;

use crate::animation::BoneId;
use crate::ml::{BONE_NAME_TOKEN_LENGTH, PAD_TOKEN};

pub struct BoneNameTokenCache {
    pub tokens: HashMap<BoneId, [i64; BONE_NAME_TOKEN_LENGTH]>,
}

impl Default for BoneNameTokenCache {
    fn default() -> Self {
        Self {
            tokens: HashMap::new(),
        }
    }
}

impl BoneNameTokenCache {
    pub fn clear(&mut self) {
        self.tokens.clear();
    }

    pub fn get(&self, bone_id: BoneId) -> [i64; BONE_NAME_TOKEN_LENGTH] {
        self.tokens
            .get(&bone_id)
            .copied()
            .unwrap_or([PAD_TOKEN; BONE_NAME_TOKEN_LENGTH])
    }
}
