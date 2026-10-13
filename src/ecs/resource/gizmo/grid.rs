use crate::ecs::component::{LineMesh, RenderInfo};

#[derive(Clone, Debug, Default)]
pub struct GridGizmoData {
    pub mesh: LineMesh,
    pub render_info: RenderInfo,
}
