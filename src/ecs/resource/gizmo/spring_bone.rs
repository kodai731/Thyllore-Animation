pub use thyllore_anim_core::spring_bone::SpringChainGizmoMode;

use crate::ecs::component::{LineMesh, RenderInfo};

#[derive(Clone, Debug)]
pub struct SpringBoneGizmoData {
    pub colliders_visible: bool,
    pub chain_mode: SpringChainGizmoMode,
    pub wire_mesh: LineMesh,
    pub wire_render_info: RenderInfo,
}

impl SpringBoneGizmoData {
    pub fn draws_anything(&self) -> bool {
        self.colliders_visible || self.chain_mode != SpringChainGizmoMode::None
    }
}

impl Default for SpringBoneGizmoData {
    fn default() -> Self {
        Self {
            colliders_visible: true,
            chain_mode: SpringChainGizmoMode::default(),
            wire_mesh: LineMesh::default(),
            wire_render_info: RenderInfo::default(),
        }
    }
}
