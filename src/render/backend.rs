use anyhow::Result;
use cgmath::{Matrix4, Vector3};

use crate::ecs::component::{GizmoMesh, GizmoRayToModel, GizmoVerticalLines};
use crate::ecs::systems::ProjectionData;
use crate::scene::billboard::BillboardData;

pub type MeshId = usize;

pub trait RenderBackend {
    unsafe fn upload_mesh_vertices(&mut self, mesh_id: MeshId) -> Result<()>;

    unsafe fn update_acceleration_structure(&mut self, mesh_ids: &[MeshId]) -> Result<()>;

    unsafe fn rebuild_tlas(&mut self) -> Result<()>;

    unsafe fn create_gizmo_buffers(&mut self, mesh: &mut GizmoMesh, use_staging: bool)
        -> Result<()>;

    unsafe fn update_gizmo_vertex_buffer(&self, mesh: &GizmoMesh) -> Result<()>;

    unsafe fn destroy_gizmo_buffers(&mut self, mesh: &mut GizmoMesh);

    unsafe fn update_or_create_ray_buffers(&mut self, ray: &mut GizmoRayToModel) -> Result<()>;

    unsafe fn destroy_ray_buffers(&mut self, ray: &mut GizmoRayToModel);

    unsafe fn update_or_create_vertical_line_buffers(
        &mut self,
        lines: &mut GizmoVerticalLines,
    ) -> Result<()>;

    unsafe fn destroy_vertical_line_buffers(&mut self, lines: &mut GizmoVerticalLines);

    unsafe fn create_billboard_buffers(&mut self, billboard: &mut BillboardData) -> Result<()>;

    unsafe fn update_frame_ubo(
        &mut self,
        proj_data: &ProjectionData,
        camera_pos: Vector3<f32>,
        light_pos: Vector3<f32>,
        light_color: Vector3<f32>,
        image_index: usize,
    ) -> Result<()>;

    unsafe fn update_object_ubo(
        &mut self,
        model_matrix: Matrix4<f32>,
        object_index: usize,
        image_index: usize,
    ) -> Result<()>;
}
