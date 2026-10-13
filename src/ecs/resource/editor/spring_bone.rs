use crate::ecs::component::{SpringChainId, SpringColliderId};

pub struct SpringBoneEditorState {
    pub selected_chain_id: Option<SpringChainId>,
    pub selected_collider_id: Option<SpringColliderId>,
    pub show_collider_gizmos: bool,
}

impl Default for SpringBoneEditorState {
    fn default() -> Self {
        Self {
            selected_chain_id: None,
            selected_collider_id: None,
            show_collider_gizmos: true,
        }
    }
}
