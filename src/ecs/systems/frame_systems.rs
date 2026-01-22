use anyhow::Result;
use cgmath::{Deg, Matrix3, Matrix4, Vector2, Vector3};

use super::{
    billboard_transform_update_look_at, create_billboard_transform, gizmo_update_rotation,
};
use crate::app::data::LightMoveTarget;
use crate::debugview::view_mode::RayTracingDebugState;
use crate::math::coordinate_system::perspective;
use crate::render::RenderBackend;
use crate::app::billboard::BillboardData;
use crate::scene::camera::Camera;

pub struct ProjectionData {
    pub view: Matrix4<f32>,
    pub proj: Matrix4<f32>,
    pub screen_size: Vector2<f32>,
    pub aspect: f32,
}

pub fn calculate_projection(camera: &Camera, swapchain_extent: (u32, u32)) -> ProjectionData {
    let view = unsafe { crate::math::view(camera.position, camera.direction, camera.up) };
    let aspect = swapchain_extent.0 as f32 / swapchain_extent.1 as f32;
    let proj = perspective(Deg(45.0), aspect, camera.near_plane, camera.far_plane);
    let screen_size = Vector2::new(swapchain_extent.0 as f32, swapchain_extent.1 as f32);

    ProjectionData {
        view,
        proj,
        screen_size,
        aspect,
    }
}

pub unsafe fn update_frame_ubo(
    backend: &mut dyn RenderBackend,
    proj_data: &ProjectionData,
    camera_position: Vector3<f32>,
    light_position: Vector3<f32>,
    light_color: Vector3<f32>,
    image_index: usize,
) -> Result<()> {
    backend.update_frame_ubo(proj_data, camera_position, light_position, light_color, image_index)
}

pub unsafe fn update_object_ubos(
    backend: &mut dyn RenderBackend,
    model_matrix: Matrix4<f32>,
    object_index: usize,
    image_index: usize,
) -> Result<()> {
    backend.update_object_ubo(model_matrix, object_index, image_index)
}

pub fn update_billboard_transform(
    billboard: &mut BillboardData,
    light_position: Vector3<f32>,
    camera_position: Vector3<f32>,
    camera_up: Vector3<f32>,
) {
    if billboard.info.transform.is_none() {
        billboard.info.transform = Some(create_billboard_transform(light_position));
    }

    if let Some(ref mut transform) = billboard.info.transform {
        transform.position = light_position;
        billboard_transform_update_look_at(transform, camera_position, camera_up);
    }
}

pub fn update_grid_gizmo_rotation_from_view(
    gizmo: &mut crate::debugview::gizmo::GridGizmoData,
    view: Matrix4<f32>,
) {
    let (camera_right, camera_up, camera_forward) = get_camera_axes_from_view(view);

    let rotation_matrix = Matrix3::from_cols(
        Vector3::new(camera_right.x, camera_up.x, camera_forward.x),
        Vector3::new(camera_right.y, camera_up.y, camera_forward.y),
        Vector3::new(camera_right.z, camera_up.z, camera_forward.z),
    );

    gizmo_update_rotation(&mut gizmo.mesh, &rotation_matrix);
}

fn get_camera_axes_from_view(view: Matrix4<f32>) -> (Vector3<f32>, Vector3<f32>, Vector3<f32>) {
    let camera_right = Vector3::new(view[0][0], view[1][0], view[2][0]);
    let camera_up = Vector3::new(view[0][1], view[1][1], view[2][1]);
    let camera_forward = Vector3::new(view[0][2], view[1][2], view[2][2]);
    (camera_right, camera_up, camera_forward)
}

pub fn update_light_auto_target(
    rt_debug_state: &mut RayTracingDebugState,
    all_positions: &[Vector3<f32>],
    camera_position: Vector3<f32>,
    move_light_to: LightMoveTarget,
) {
    match move_light_to {
        LightMoveTarget::None => {}
        _ => {
            rt_debug_state.update_light_position(all_positions, camera_position, move_light_to);
        }
    }
}
