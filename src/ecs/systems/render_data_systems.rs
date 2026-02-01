use cgmath::{InnerSpace, Matrix4, SquareMatrix, Vector3};

use crate::app::billboard::BillboardData;
use crate::debugview::gizmo::{BoneGizmoData, GridGizmoData, LightGizmoData};
use crate::debugview::GridMeshData;
use crate::ecs::component::GpuMeshRef;
use crate::ecs::RenderData;

pub fn grid_mesh_render_data(grid: &GridMeshData) -> RenderData {
    let mesh_ref = GpuMeshRef::new(
        grid.mesh.vertex_buffer_handle,
        grid.mesh.index_buffer_handle,
        grid.mesh.indices.len() as u32,
    );
    RenderData::new(mesh_ref, grid.render_info)
        .with_model_matrix(Matrix4::from_scale(grid.scale.value()))
}

pub fn gizmo_mesh_render_data(gizmo: &GridGizmoData) -> RenderData {
    let mesh_ref = GpuMeshRef::new(
        gizmo.mesh.vertex_buffer_handle,
        gizmo.mesh.index_buffer_handle,
        gizmo.mesh.indices.len() as u32,
    );
    RenderData::new(mesh_ref, gizmo.render_info)
}

pub fn gizmo_selectable_render_data(
    gizmo: &LightGizmoData,
    camera_position: Vector3<f32>,
) -> RenderData {
    let gizmo_pos = gizmo.position.position;
    let distance = (gizmo_pos - camera_position).magnitude();
    let scale_factor = distance * 0.03;
    let model_matrix = Matrix4::from_translation(gizmo_pos) * Matrix4::from_scale(scale_factor);

    let mesh_ref = GpuMeshRef::new(
        gizmo.mesh.vertex_buffer_handle,
        gizmo.mesh.index_buffer_handle,
        gizmo.mesh.indices.len() as u32,
    );
    RenderData::new(mesh_ref, gizmo.render_info).with_model_matrix(model_matrix)
}

pub fn billboard_render_data(billboard: &BillboardData) -> RenderData {
    let model_matrix = billboard
        .transform
        .as_ref()
        .map(|t| t.model_matrix)
        .unwrap_or_else(Matrix4::identity);

    let mesh_ref = GpuMeshRef::new(
        billboard.mesh.vertex_buffer_handle,
        billboard.mesh.index_buffer_handle,
        billboard.mesh.indices.len() as u32,
    );
    RenderData::new(mesh_ref, billboard.render_info).with_model_matrix(model_matrix)
}

pub fn bone_gizmo_render_data(bone_gizmo: &BoneGizmoData) -> RenderData {
    let mesh_ref = GpuMeshRef::new(
        bone_gizmo.mesh.vertex_buffer_handle,
        bone_gizmo.mesh.index_buffer_handle,
        bone_gizmo.mesh.indices.len() as u32,
    );
    RenderData::new(mesh_ref, bone_gizmo.render_info)
}

pub fn collect_scene_render_data(
    grid: &GridMeshData,
    gizmo: &GridGizmoData,
    light_gizmo: &LightGizmoData,
    billboard: &BillboardData,
    camera_position: Vector3<f32>,
) -> Vec<RenderData> {
    vec![
        grid_mesh_render_data(grid),
        gizmo_mesh_render_data(gizmo),
        gizmo_selectable_render_data(light_gizmo, camera_position),
        billboard_render_data(billboard),
    ]
}
