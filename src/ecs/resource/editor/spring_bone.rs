use crate::animation::BoneId;
use crate::ecs::component::{ChainRamp, RampScope, SpringColliderGroupId};

#[derive(Clone, Debug, PartialEq, thyllore_scene_core::SceneFields)]
#[params(tag = thyllore_scene_core::SingleOwner, owner = Component)]
pub struct AddChainForm {
    pub root_bone: BoneId,
    #[persist(ui(min = 1.0, max = 16.0, format = "%.0f"))]
    pub chain_length: u32,
}

impl Default for AddChainForm {
    fn default() -> Self {
        Self {
            root_bone: 0,
            chain_length: 3,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AddColliderForm {
    pub bone: BoneId,
    pub capsule: bool,
}

#[derive(Clone, Debug)]
pub struct SpringBoneInspectorState {
    pub add_chain: AddChainForm,
    pub add_collider: AddColliderForm,
    pub add_collider_group_name: String,
    pub ramp: ChainRamp,
    pub ramp_scope: RampScope,
    pub chain_collider_groups: Vec<SpringColliderGroupId>,
    pub chain_collider_scope: RampScope,
}

impl Default for SpringBoneInspectorState {
    fn default() -> Self {
        Self {
            add_chain: AddChainForm::default(),
            add_collider: AddColliderForm::default(),
            add_collider_group_name: "NewGroup".to_string(),
            ramp: ChainRamp::default(),
            ramp_scope: RampScope::default(),
            chain_collider_groups: Vec::new(),
            chain_collider_scope: RampScope::default(),
        }
    }
}
