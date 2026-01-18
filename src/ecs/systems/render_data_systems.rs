use cgmath::{InnerSpace, Matrix4, SquareMatrix, Vector3};
use vulkanalia::prelude::v1_0::*;

use crate::debugview::gizmo::{GridGizmoData, LightGizmoData};
use crate::ecs::RenderData;
use crate::scene::billboard::BillboardData;
use crate::scene::grid::GridData;

pub fn grid_render_data(grid: &GridData) -> RenderData {
    RenderData {
        object_index: grid.object_index,
        pipeline_id: grid.pipeline_id,
        vertex_buffer: grid.vertex_buffer.buffer,
        index_buffer: grid.index_buffer.buffer,
        index_count: grid.index_buffer.indices,
        model_matrix: Matrix4::from_scale(grid.scale),
    }
}

pub fn gizmo_mesh_render_data(gizmo: &GridGizmoData) -> RenderData {
    RenderData {
        object_index: gizmo.mesh.object_index,
        pipeline_id: gizmo.mesh.pipeline_id,
        vertex_buffer: gizmo.mesh.vertex_buffer.unwrap_or(vk::Buffer::null()),
        index_buffer: gizmo.mesh.index_buffer.unwrap_or(vk::Buffer::null()),
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
        vertex_buffer: gizmo.mesh.vertex_buffer.unwrap_or(vk::Buffer::null()),
        index_buffer: gizmo.mesh.index_buffer.unwrap_or(vk::Buffer::null()),
        index_count: gizmo.mesh.indices.len() as u32,
        model_matrix,
    }
}

pub fn billboard_render_data(billboard: &BillboardData) -> RenderData {
    let model_matrix = billboard
        .transform
        .as_ref()
        .map(|t| t.model_matrix)
        .unwrap_or_else(Matrix4::identity);

    RenderData {
        object_index: billboard.object_index,
        pipeline_id: billboard.pipeline_id,
        vertex_buffer: billboard.vertex_buffer.unwrap_or(vk::Buffer::null()),
        index_buffer: billboard.index_buffer.unwrap_or(vk::Buffer::null()),
        index_count: billboard.indices.len() as u32,
        model_matrix,
    }
}

pub fn collect_scene_render_data(
    grid: &GridData,
    gizmo: &GridGizmoData,
    light_gizmo: &LightGizmoData,
    billboard: &BillboardData,
    camera_position: Vector3<f32>,
) -> Vec<RenderData> {
    vec![
        grid_render_data(grid),
        gizmo_mesh_render_data(gizmo),
        gizmo_selectable_render_data(light_gizmo, camera_position),
        billboard_render_data(billboard),
    ]
}
