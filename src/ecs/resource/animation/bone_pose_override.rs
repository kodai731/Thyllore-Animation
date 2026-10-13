use std::collections::HashMap;

use crate::animation::{BoneId, BoneLocalPose};

#[derive(Default)]
pub struct BonePoseOverride {
    pub overrides: HashMap<BoneId, BoneLocalPose>,
}

impl BonePoseOverride {
    pub fn clear(&mut self) {
        self.overrides.clear();
    }

    pub fn has_overrides(&self) -> bool {
        !self.overrides.is_empty()
    }
}
