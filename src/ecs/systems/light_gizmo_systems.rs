use cgmath::{vec3, Deg, InnerSpace, Vector2, Vector3};

use crate::debugview::gizmo::light::{LightGizmoAxis, LightGizmoData};
use crate::math::{
    coordinate_system::perspective, is_point_in_rect, ray_to_line_segment_distance,
    ray_to_point_distance, screen_to_world_ray, view,
};

pub fn light_gizmo_try_select(
    gizmo: &mut LightGizmoData,
    mouse_pos: Vector2<f32>,
    camera_pos: Vector3<f32>,
    camera_direction: Vector3<f32>,
    camera_up: Vector3<f32>,
    swapchain_extent: (u32, u32),
    light_pos: Vector3<f32>,
    billboard_click_rect: Option<[f32; 4]>,
) {
    let view_mat = unsafe { view(camera_pos, camera_direction, camera_up) };
    let aspect = swapchain_extent.0 as f32 / swapchain_extent.1 as f32;
    let proj = perspective(Deg(45.0), aspect, 0.1, 10000.0);
    let screen_size = Vector2::new(swapchain_extent.0 as f32, swapchain_extent.1 as f32);

    let (ray_origin, ray_direction) = screen_to_world_ray(mouse_pos, screen_size, view_mat, proj);

    let distance = (light_pos - camera_pos).magnitude();
    let scale_factor = distance * 0.03;

    let billboard_clicked = billboard_click_rect
        .map(|rect| is_point_in_rect(mouse_pos, rect))
        .unwrap_or(false);

    let center_distance = ray_to_point_distance(ray_origin, ray_direction, light_pos);

    let x_axis_end = light_pos + vec3(1.0 * scale_factor, 0.0, 0.0);
    let y_axis_end = light_pos + vec3(0.0, 1.0 * scale_factor, 0.0);
    let z_axis_end = light_pos + vec3(0.0, 0.0, 1.0 * scale_factor);

    let x_distance = ray_to_line_segment_distance(ray_origin, ray_direction, light_pos, x_axis_end);
    let y_distance = ray_to_line_segment_distance(ray_origin, ray_direction, light_pos, y_axis_end);
    let z_distance = ray_to_line_segment_distance(ray_origin, ray_direction, light_pos, z_axis_end);

    let threshold = 0.05 * scale_factor;

    let mut min_distance = center_distance;
    let mut selected_axis = LightGizmoAxis::None;

    if billboard_clicked {
        selected_axis = LightGizmoAxis::Center;
        min_distance = 0.0;
    } else {
        if center_distance < threshold {
            selected_axis = LightGizmoAxis::Center;
        }

        if x_distance < threshold && x_distance < min_distance {
            min_distance = x_distance;
            selected_axis = LightGizmoAxis::X;
        }

        if y_distance < threshold && y_distance < min_distance {
            min_distance = y_distance;
            selected_axis = LightGizmoAxis::Y;
        }

        if z_distance < threshold && z_distance < min_distance {
            let _ = min_distance;
            selected_axis = LightGizmoAxis::Z;
        }
    }

    if selected_axis != LightGizmoAxis::None {
        gizmo.is_selected = true;
        gizmo.drag_axis = selected_axis;
        gizmo.selected_axis = selected_axis;
        gizmo.initial_position = [light_pos.x, light_pos.y, light_pos.z];

        let drag_depth = (light_pos - camera_pos).magnitude();
        crate::log!(
            "Light gizmo selected - axis: {:?}, depth: {:.2}",
            selected_axis,
            drag_depth
        );

        gizmo.just_selected = true;
    }
}

pub fn light_gizmo_update_position(gizmo: &mut LightGizmoData, position: Vector3<f32>) {
    gizmo.position = position;
}

pub fn light_gizmo_set_default(gizmo: &mut LightGizmoData) {
    gizmo.is_selected = false;
    gizmo.drag_axis = LightGizmoAxis::None;
    gizmo.selected_axis = LightGizmoAxis::None;
    gizmo.just_selected = false;
    gizmo.initial_position = [0.0, 0.0, 0.0];
}

pub fn light_gizmo_sync_from_debug_state(
    gizmo: &mut LightGizmoData,
    debug_state_position: Vector3<f32>,
) {
    if gizmo.position.x != debug_state_position.x
        || gizmo.position.y != debug_state_position.y
        || gizmo.position.z != debug_state_position.z
    {
        crate::log!("LightGizmoData: syncing from rt_debug_state");
        crate::log!(
            "  Before: ({:.2}, {:.2}, {:.2})",
            gizmo.position.x,
            gizmo.position.y,
            gizmo.position.z
        );
        crate::log!(
            "  After:  ({:.2}, {:.2}, {:.2})",
            debug_state_position.x,
            debug_state_position.y,
            debug_state_position.z
        );
        gizmo.position = debug_state_position;
    }
}

pub fn light_gizmo_update_position_with_constraint(
    gizmo: &mut LightGizmoData,
    new_position: Vector3<f32>,
    initial_position: Vector3<f32>,
    is_ctrl_pressed: bool,
) {
    if is_ctrl_pressed {
        let delta = new_position - initial_position;

        let abs_x = delta.x.abs();
        let abs_y = delta.y.abs();
        let abs_z = delta.z.abs();

        let constrained_pos = if abs_x >= abs_y && abs_x >= abs_z {
            Vector3::new(
                initial_position.x + delta.x,
                initial_position.y,
                initial_position.z,
            )
        } else if abs_y >= abs_x && abs_y >= abs_z {
            Vector3::new(
                initial_position.x,
                initial_position.y + delta.y,
                initial_position.z,
            )
        } else {
            Vector3::new(
                initial_position.x,
                initial_position.y,
                initial_position.z + delta.z,
            )
        };

        crate::log!(
            "Ctrl pressed - axis constrained: initial({:.2}, {:.2}, {:.2}) -> delta({:.2}, {:.2}, {:.2}) -> constrained({:.2}, {:.2}, {:.2})",
            initial_position.x,
            initial_position.y,
            initial_position.z,
            delta.x,
            delta.y,
            delta.z,
            constrained_pos.x,
            constrained_pos.y,
            constrained_pos.z
        );

        gizmo.position = constrained_pos;
    } else {
        gizmo.position = new_position;
    }
}

pub fn light_gizmo_update_selection_color(gizmo: &mut LightGizmoData) {
    let yellow = [1.0, 1.0, 0.0];
    let highlight = [1.0, 1.0, 0.5];

    gizmo.vertices[0].color = yellow;
    gizmo.vertices[1].color = [1.0, 0.0, 0.0];
    gizmo.vertices[2].color = [0.0, 1.0, 0.0];
    gizmo.vertices[3].color = [0.0, 0.0, 1.0];

    match gizmo.selected_axis {
        LightGizmoAxis::None => {}
        LightGizmoAxis::Center => {
            gizmo.vertices[0].color = highlight;
        }
        LightGizmoAxis::X => {
            gizmo.vertices[1].color = [1.0, 0.5, 0.0];
        }
        LightGizmoAxis::Y => {
            gizmo.vertices[2].color = [0.5, 1.0, 0.0];
        }
        LightGizmoAxis::Z => {
            gizmo.vertices[3].color = [0.0, 0.5, 1.0];
        }
    }
}
