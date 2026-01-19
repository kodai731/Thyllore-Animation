use anyhow::Result;

pub type MeshId = usize;

pub trait RenderBackend {
    unsafe fn upload_mesh_vertices(&mut self, mesh_id: MeshId) -> Result<()>;

    unsafe fn update_acceleration_structure(&mut self, mesh_ids: &[MeshId]) -> Result<()>;

    unsafe fn rebuild_tlas(&mut self) -> Result<()>;
}
