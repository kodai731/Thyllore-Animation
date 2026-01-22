use cgmath::{InnerSpace, Matrix4, SquareMatrix, Vector3};

use crate::debugview::gizmo::{GridGizmoData, LightGizmoData};
use crate::ecs::component::{LineMesh, MeshScale};
use crate::ecs::RenderData;
use crate::app::billboard::BillboardData;

pub fn line_mesh_render_data(mesh: &LineMesh, scale: &MeshScale) -> RenderData {
    RenderData {
        object_index: mesh.object_index,
        pipeline_id: mesh.pipeline_id,
        vertex_buffer_handle: mesh.vertex_buffer_handle,
        index_buffer_handle: mesh.index_buffer_handle,
        index_count: mesh.indices.len() as u32,
        model_matrix: Matrix4::from_scale(scale.value()),
    }
}

pub fn gizmo_mesh_render_data(gizmo: &GridGizmoData) -> RenderData {
    RenderData {
        object_index: gizmo.mesh.object_index,
        pipeline_id: gizmo.mesh.pipeline_id,
        vertex_buffer_handle: gizmo.mesh.vertex_buffer_handle,
        index_buffer_handle: gizmo.mesh.index_buffer_handle,
        index_count: gizmo.mesh.indices.len() as u32,
        model_matrix: Matrix4::identity(),
    }
}

pub fn gizmo_selectable_render_data(
    gizmo: &LightGizmoData,
    camera_position: Vector3<f32>,
) -> RenderData {
    let gizmo_pos = gizmo.position.position;
    let distance = (gizmo_pos - camera_position).magnitude();
    let scale_factor = distance * 0.03;
    let model_matrix = Matrix4::from_translation(gizmo_pos) * Matrix4::from_scale(scale_factor);

    RenderData {
        object_index: gizmo.mesh.object_index,
        pipeline_id: gizmo.mesh.pipeline_id,
        vertex_buffer_handle: gizmo.mesh.vertex_buffer_handle,
        index_buffer_handle: gizmo.mesh.index_buffer_handle,
        index_count: gizmo.mesh.indices.len() as u32,
        model_matrix,
    }
}

pub fn billboard_render_data(billboard: &BillboardData) -> RenderData {
    let model_matrix = billboard
        .info
        .transform
        .as_ref()
        .map(|t| t.model_matrix)
        .unwrap_or_else(Matrix4::identity);

    RenderData {
        object_index: billboard.render.object_index,
        pipeline_id: billboard.render.pipeline_id,
        vertex_buffer_handle: billboard.info.vertex_buffer_handle,
        index_buffer_handle: billboard.info.index_buffer_handle,
        index_count: billboard.info.indices.len() as u32,
        model_matrix,
    }
}

pub fn collect_scene_render_data(
    grid_mesh: &LineMesh,
    grid_scale: &MeshScale,
    gizmo: &GridGizmoData,
    light_gizmo: &LightGizmoData,
    billboard: &BillboardData,
    camera_position: Vector3<f32>,
) -> Vec<RenderData> {
    vec![
        line_mesh_render_data(grid_mesh, grid_scale),
        gizmo_mesh_render_data(gizmo),
        gizmo_selectable_render_data(light_gizmo, camera_position),
        billboard_render_data(billboard),
    ]
}
