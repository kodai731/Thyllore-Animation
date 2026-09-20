use std::collections::HashMap;

use crate::animation::editable::PropertyType;
use crate::animation::BoneId;

#[derive(Clone, Debug, Default)]
pub struct CurveEditorBuffer {
    pub snapshots: HashMap<(BoneId, PropertyType), Vec<(f32, f32)>>,
}

impl CurveEditorBuffer {
    pub fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }

    pub fn clear(&mut self) {
        self.snapshots.clear();
    }

    pub fn has_snapshot(&self, bone_id: BoneId, property_type: PropertyType) -> bool {
        self.snapshots.contains_key(&(bone_id, property_type))
    }

    pub fn get_snapshot(
        &self,
        bone_id: BoneId,
        property_type: PropertyType,
    ) -> Option<&Vec<(f32, f32)>> {
        self.snapshots.get(&(bone_id, property_type))
    }
}
