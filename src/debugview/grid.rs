use crate::ecs::component::{LineMesh, MeshScale, RenderInfo};

#[derive(Clone, Debug, Default)]
pub struct GridMeshData {
    pub mesh: LineMesh,
    pub render_info: RenderInfo,
    pub scale: MeshScale,
}
