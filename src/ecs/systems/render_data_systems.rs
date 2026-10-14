use cgmath::{InnerSpace, Matrix4, Vector3};

use crate::ecs::component::{DynamicMesh, GpuMeshRef, MeshBuffers};
use crate::ecs::resource::gizmo::{
    BoneDisplayStyle, BoneGizmoData, ConstraintGizmoData, GridGizmoData, LightGizmoData,
    SpringBoneGizmoData, TransformGizmoData,
};
use crate::ecs::resource::GridMeshData;
use crate::ecs::RenderData;

fn frame_slot_mesh_ref<V>(mesh: &DynamicMesh<V>, frame_slot: usize) -> GpuMeshRef {
    GpuMeshRef::new(
        mesh.vertex_buffer_handle(frame_slot),
        mesh.index_buffer_handle(frame_slot),
        mesh.index_count(),
    )
}

pub fn grid_mesh_render_data(grid: &GridMeshData) -> RenderData {
    let index_count = if grid.show_y_axis_grid {
        grid.mesh.indices.len() as u32
    } else {
        grid.xz_only_index_count
    };

    let mesh_ref = GpuMeshRef::new(
        grid.mesh.vertex_buffer_handle,
        grid.mesh.index_buffer_handle,
        index_count,
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
    frame_slot: usize,
) -> RenderData {
    let gizmo_pos = gizmo.position.position;
    let distance = (gizmo_pos - camera_position).magnitude();
    let scale_factor = distance * 0.03;
    let model_matrix = Matrix4::from_translation(gizmo_pos) * Matrix4::from_scale(scale_factor);

    let mesh_ref = frame_slot_mesh_ref(&gizmo.mesh, frame_slot);
    RenderData::new(mesh_ref, gizmo.render_info).with_model_matrix(model_matrix)
}

pub fn bone_gizmo_render_data(bone_gizmo: &BoneGizmoData, frame_slot: usize) -> Vec<RenderData> {
    match bone_gizmo.display_style {
        BoneDisplayStyle::Stick => {
            let mesh_ref = frame_slot_mesh_ref(&bone_gizmo.stick_mesh, frame_slot);
            vec![RenderData::new(mesh_ref, bone_gizmo.stick_render_info)]
        }
        BoneDisplayStyle::Octahedral | BoneDisplayStyle::Box | BoneDisplayStyle::Sphere => {
            let solid_ref = frame_slot_mesh_ref(&bone_gizmo.solid_mesh, frame_slot);
            let wire_ref = frame_slot_mesh_ref(&bone_gizmo.wire_mesh, frame_slot);
            vec![
                RenderData::new(solid_ref, bone_gizmo.solid_render_info),
                RenderData::new(wire_ref, bone_gizmo.wire_render_info),
            ]
        }
    }
}

pub fn constraint_gizmo_render_data(
    constraint_gizmo: &ConstraintGizmoData,
    frame_slot: usize,
) -> Vec<RenderData> {
    if constraint_gizmo.wire_mesh.indices.is_empty() {
        return Vec::new();
    }

    let mesh_ref = frame_slot_mesh_ref(&constraint_gizmo.wire_mesh, frame_slot);
    vec![RenderData::new(mesh_ref, constraint_gizmo.wire_render_info)]
}

pub fn spring_bone_gizmo_render_data(
    gizmo: &SpringBoneGizmoData,
    frame_slot: usize,
) -> Vec<RenderData> {
    if gizmo.wire_mesh.indices.is_empty() {
        return Vec::new();
    }

    let mesh_ref = frame_slot_mesh_ref(&gizmo.wire_mesh, frame_slot);
    vec![RenderData::new(mesh_ref, gizmo.wire_render_info)]
}

pub fn transform_gizmo_render_data(
    gizmo: &TransformGizmoData,
    camera_position: Vector3<f32>,
    gizmo_scale: f32,
    frame_slot: usize,
) -> Vec<RenderData> {
    if !gizmo.visible {
        return Vec::new();
    }

    let gizmo_pos = gizmo.position.position;
    let distance = (gizmo_pos - camera_position).magnitude();
    let scale_factor = distance * gizmo_scale;
    let model_matrix = Matrix4::from_translation(gizmo_pos) * Matrix4::from_scale(scale_factor);

    let mut result = Vec::new();

    if !gizmo.line_mesh.indices.is_empty() {
        let line_ref = frame_slot_mesh_ref(&gizmo.line_mesh, frame_slot);
        result.push(
            RenderData::new(line_ref, gizmo.line_render_info).with_model_matrix(model_matrix),
        );
    }

    if !gizmo.solid_mesh.indices.is_empty() {
        let solid_ref = frame_slot_mesh_ref(&gizmo.solid_mesh, frame_slot);
        result.push(
            RenderData::new(solid_ref, gizmo.solid_render_info).with_model_matrix(model_matrix),
        );
    }

    result
}
