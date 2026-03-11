use crate::ecs::component::{LineMesh, MeshScale, RenderInfo};

#[derive(Clone, Debug, Default)]
pub struct GridMeshData {
    pub mesh: LineMesh,
    pub render_info: RenderInfo,
    pub scale: MeshScale,
    pub show_y_axis_grid: bool,
    pub xz_only_index_count: u32,
}
