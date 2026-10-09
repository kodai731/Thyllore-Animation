use std::collections::BTreeMap;

use crate::humanoid::components::role::HumanoidRole;

#[derive(Clone, Debug)]
pub struct SampledPose {
    pub rotations: BTreeMap<HumanoidRole, [f32; 3]>,
    pub hips_translation: [f32; 3],
    pub morph: BTreeMap<String, f32>,
}
