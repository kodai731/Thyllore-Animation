use crate::animation::BoneId;
use crate::ecs::component::{SpringChainId, SpringColliderId};

pub struct SpringBoneEditorState {
    pub selected_chain_id: Option<SpringChainId>,
    pub selected_collider_id: Option<SpringColliderId>,
    pub show_collider_gizmos: bool,
    pub add_chain_root_bone: BoneId,
    pub add_chain_length: i32,
    pub add_collider_bone: BoneId,
    pub add_collider_shape_index: i32,
    pub new_group_name: String,
}

impl Default for SpringBoneEditorState {
    fn default() -> Self {
        Self {
            selected_chain_id: None,
            selected_collider_id: None,
            show_collider_gizmos: true,
            add_chain_root_bone: 0,
            add_chain_length: 3,
            add_collider_bone: 0,
            add_collider_shape_index: 0,
            new_group_name: "NewGroup".to_string(),
        }
    }
}
