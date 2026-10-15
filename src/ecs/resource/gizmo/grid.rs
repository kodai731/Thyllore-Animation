use crate::ecs::component::{RenderInfo, StaticLineMesh};

#[derive(Clone, Debug, Default)]
pub struct GridGizmoData {
    pub mesh: StaticLineMesh,
    pub render_info: RenderInfo,
}
