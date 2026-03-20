use anyhow::Result;
use cgmath::{Matrix4, Vector2, Vector3};

use super::{
    billboard_transform_update_look_at, compute_camera_direction, compute_camera_position,
    compute_camera_up, create_billboard_transform,
};
use crate::app::data::LightMoveTarget;
use crate::ecs::resource::billboard::BillboardData;
use crate::ecs::resource::{Camera, LightState, ProjectionData};
use crate::math::coordinate_system::perspective_infinite_reverse;
use crate::render::RenderBackend;

pub fn calculate_projection(camera: &Camera, swapchain_extent: (u32, u32)) -> ProjectionData {
    let position = compute_camera_position(camera);
    let direction = compute_camera_direction(camera);
    let up = compute_camera_up(camera);

    let view = unsafe { crate::math::view(position, direction, up) };
    let aspect = swapchain_extent.0 as f32 / swapchain_extent.1 as f32;
    let proj = perspective_infinite_reverse(camera.fov_y, aspect, camera.near_plane);
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
    backend.update_frame_ubo(
        proj_data,
        camera_position,
        light_position,
        light_color,
        image_index,
    )
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
    if billboard.transform.is_none() {
        billboard.transform = Some(create_billboard_transform(light_position));
    }

    if let Some(ref mut transform) = billboard.transform {
        transform.position = light_position;
        billboard_transform_update_look_at(transform, camera_position, camera_up);
    }
}

pub fn update_light_auto_target(
    light_state: &mut LightState,
    all_positions: &[Vector3<f32>],
    _camera_position: Vector3<f32>,
    move_light_to: LightMoveTarget,
) {
    if matches!(move_light_to, LightMoveTarget::None) {
        return;
    }

    log!("LIGHT MOVE BUTTON PRESSED: {:?}", move_light_to);

    if all_positions.is_empty() {
        log_warn!("No model positions found!");
        return;
    }

    let (min, max) = compute_model_bounds(all_positions);

    let model_size = ((max.x - min.x).abs() + (max.y - min.y).abs() + (max.z - min.z).abs()) / 3.0;

    let offset = 2.0;
    let current = light_state.light_position;
    let new_pos = match move_light_to {
        LightMoveTarget::XMin => Vector3::new(min.x - offset, current.y, current.z),
        LightMoveTarget::XMax => Vector3::new(max.x + offset, current.y, current.z),
        LightMoveTarget::YMin => Vector3::new(current.x, min.y - offset, current.z),
        LightMoveTarget::YMax => Vector3::new(current.x, max.y + offset, current.z),
        LightMoveTarget::ZMin => Vector3::new(current.x, current.y, min.z - offset),
        LightMoveTarget::ZMax => Vector3::new(current.x, current.y, max.z + offset),
        LightMoveTarget::None => current,
    };

    light_state.shadow_normal_offset = (model_size * 0.005).max(0.5);

    log!(
        "Model bounds: X[{:.2}, {:.2}], Y[{:.2}, {:.2}], Z[{:.2}, {:.2}]",
        min.x,
        max.x,
        min.y,
        max.y,
        min.z,
        max.z
    );
    log!(
        "Light position: ({:.2}, {:.2}, {:.2}) -> ({:.2}, {:.2}, {:.2})",
        current.x,
        current.y,
        current.z,
        new_pos.x,
        new_pos.y,
        new_pos.z,
    );

    light_state.light_position = new_pos;
}

fn compute_model_bounds(positions: &[Vector3<f32>]) -> (Vector3<f32>, Vector3<f32>) {
    let mut min = Vector3::new(f32::MAX, f32::MAX, f32::MAX);
    let mut max = Vector3::new(f32::MIN, f32::MIN, f32::MIN);

    for pos in positions {
        min.x = min.x.min(pos.x);
        max.x = max.x.max(pos.x);
        min.y = min.y.min(pos.y);
        max.y = max.y.max(pos.y);
        min.z = min.z.min(pos.z);
        max.z = max.z.max(pos.z);
    }

    (min, max)
}
