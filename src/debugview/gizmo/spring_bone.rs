use crate::ecs::component::{LineMesh, RenderInfo};

#[derive(Clone, Debug)]
pub struct SpringBoneGizmoData {
    pub visible: bool,
    pub wire_mesh: LineMesh,
    pub wire_render_info: RenderInfo,
}

impl Default for SpringBoneGizmoData {
    fn default() -> Self {
        Self {
            visible: true,
            wire_mesh: LineMesh::default(),
            wire_render_info: RenderInfo::default(),
        }
    }
}
