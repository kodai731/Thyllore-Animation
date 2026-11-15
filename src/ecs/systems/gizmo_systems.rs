use anyhow::Result;
use glam::{Vec2, Vec3};

use crate::ecs::component::{
    ColorVertex, GizmoAxis, GizmoDraggable, GizmoPosition, GizmoSelectable, LineMesh, RenderInfo,
    StaticLineMesh,
};
use crate::ecs::resource::gizmo::grid::GridGizmoData;
use crate::ecs::resource::gizmo::light::LightGizmoData;
use thyllore_math_core::{
    coordinate_system::perspective_infinite_reverse, is_point_in_rect,
    ray_to_line_segment_distance, ray_to_point_distance, screen_to_world_ray, view,
};
use thyllore_render_core::FRAMES_IN_FLIGHT;
use thyllore_render_core::{BufferMemoryType, IndexBufferHandle, VertexBufferHandle};

const AXIS_LENGTH_PER_CAMERA_DISTANCE: f32 = 0.03;
const PICK_THRESHOLD_PER_AXIS_LENGTH: f32 = 0.05;

pub fn create_light_gizmo(position: Vec3) -> LightGizmoData {
    let axis_length = 1.0;
    let yellow = [1.0, 1.0, 0.0];

    let vertices = vec![
        ColorVertex {
            pos: [0.0, 0.0, 0.0],
            color: yellow,
        },
        ColorVertex {
            pos: [axis_length, 0.0, 0.0],
            color: [1.0, 0.0, 0.0],
        },
        ColorVertex {
            pos: [0.0, axis_length, 0.0],
            color: [0.0, 1.0, 0.0],
        },
        ColorVertex {
            pos: [0.0, 0.0, axis_length],
            color: [0.0, 0.0, 1.0],
        },
    ];

    let indices = vec![0, 1, 0, 2, 0, 3];

    LightGizmoData {
        mesh: LineMesh {
            vertices,
            indices,
            vertex_buffer_handles: [VertexBufferHandle::INVALID; FRAMES_IN_FLIGHT],
            index_buffer_handles: [IndexBufferHandle::INVALID; FRAMES_IN_FLIGHT],
        },
        render_info: RenderInfo::default(),
        position: GizmoPosition { position },
        selectable: GizmoSelectable::default(),
        draggable: GizmoDraggable::default(),
        drag_active: false,
        vertical_lines: LineMesh::default(),
        pending_uploads: 0,
    }
}

pub fn create_grid_gizmo() -> GridGizmoData {
    let axis_length = 0.15;

    let vertices = vec![
        ColorVertex {
            pos: [0.0, 0.0, 0.0],
            color: [1.0, 1.0, 1.0],
        },
        ColorVertex {
            pos: [axis_length, 0.0, 0.0],
            color: [1.0, 0.0, 0.0],
        },
        ColorVertex {
            pos: [0.0, axis_length, 0.0],
            color: [0.0, 1.0, 0.0],
        },
        ColorVertex {
            pos: [0.0, 0.0, axis_length],
            color: [0.0, 0.0, 1.0],
        },
    ];

    let indices = vec![0, 1, 0, 2, 0, 3];

    GridGizmoData {
        mesh: StaticLineMesh {
            vertices,
            indices,
            vertex_buffer_handle: VertexBufferHandle::INVALID,
            index_buffer_handle: IndexBufferHandle::INVALID,
        },
        render_info: RenderInfo::default(),
    }
}

pub fn gizmo_try_select(
    position: &GizmoPosition,
    selectable: &mut GizmoSelectable,
    draggable: &mut GizmoDraggable,
    mouse_pos: Vec2,
    camera_pos: Vec3,
    camera_direction: Vec3,
    camera_up: Vec3,
    swapchain_extent: (u32, u32),
    billboard_click_rect: Option<[f32; 4]>,
    fov_y: f32,
    near_plane: f32,
) {
    let light_pos = position.position;
    let view_mat = view(camera_pos, camera_direction, camera_up);
    let aspect = swapchain_extent.0 as f32 / swapchain_extent.1 as f32;
    let proj = perspective_infinite_reverse(fov_y, aspect, near_plane);
    let screen_size = Vec2::new(swapchain_extent.0 as f32, swapchain_extent.1 as f32);

    let (ray_origin, ray_direction) = screen_to_world_ray(mouse_pos, screen_size, view_mat, proj);

    let distance = (light_pos - camera_pos).length();
    let axis_length = distance * AXIS_LENGTH_PER_CAMERA_DISTANCE;

    let billboard_clicked = billboard_click_rect
        .map(|rect| is_point_in_rect(mouse_pos, rect))
        .unwrap_or(false);

    let center_distance = ray_to_point_distance(ray_origin, ray_direction, light_pos);

    let x_axis_end = light_pos + Vec3::X * axis_length;
    let y_axis_end = light_pos + Vec3::Y * axis_length;
    let z_axis_end = light_pos + Vec3::Z * axis_length;

    let x_distance = ray_to_line_segment_distance(ray_origin, ray_direction, light_pos, x_axis_end);
    let y_distance = ray_to_line_segment_distance(ray_origin, ray_direction, light_pos, y_axis_end);
    let z_distance = ray_to_line_segment_distance(ray_origin, ray_direction, light_pos, z_axis_end);

    let threshold = PICK_THRESHOLD_PER_AXIS_LENGTH * axis_length;

    let nearest_axis = [
        (x_distance, GizmoAxis::X),
        (y_distance, GizmoAxis::Y),
        (z_distance, GizmoAxis::Z),
    ]
    .into_iter()
    .filter(|(axis_distance, _)| *axis_distance < threshold && *axis_distance < center_distance)
    .min_by(|a, b| a.0.total_cmp(&b.0))
    .map(|(_, axis)| axis);

    let center_or_none = if center_distance < threshold {
        GizmoAxis::Center
    } else {
        GizmoAxis::None
    };
    let selected_axis = if billboard_clicked {
        GizmoAxis::Center
    } else {
        nearest_axis.unwrap_or(center_or_none)
    };

    if selected_axis != GizmoAxis::None {
        selectable.is_selected = true;
        selectable.selected_axis = selected_axis;
        draggable.drag_axis = selected_axis;
        draggable.initial_position = light_pos;

        let drag_depth = (light_pos - camera_pos).length();
        log!(
            "Gizmo selected - axis: {:?}, depth: {:.2}",
            selected_axis,
            drag_depth
        );
    }
}

pub fn gizmo_update_position(position: &mut GizmoPosition, new_position: Vec3) {
    position.position = new_position;
}

pub fn gizmo_reset_selection(selectable: &mut GizmoSelectable, draggable: &mut GizmoDraggable) {
    selectable.is_selected = false;
    selectable.selected_axis = GizmoAxis::None;
    draggable.drag_axis = GizmoAxis::None;
    draggable.initial_position = Vec3::new(0.0, 0.0, 0.0);
}

pub fn gizmo_sync_position(position: &mut GizmoPosition, source_position: Vec3) {
    if position.position.x != source_position.x
        || position.position.y != source_position.y
        || position.position.z != source_position.z
    {
        log!("GizmoPosition: syncing from source");
        log!(
            "  Before: ({:.2}, {:.2}, {:.2})",
            position.position.x,
            position.position.y,
            position.position.z
        );
        log!(
            "  After:  ({:.2}, {:.2}, {:.2})",
            source_position.x,
            source_position.y,
            source_position.z
        );
        position.position = source_position;
    }
}

pub fn gizmo_update_position_with_constraint(
    position: &mut GizmoPosition,
    new_position: Vec3,
    draggable: &GizmoDraggable,
    is_ctrl_pressed: bool,
) {
    if is_ctrl_pressed {
        let initial = draggable.initial_position;
        let delta = new_position - initial;

        let abs_x = delta.x.abs();
        let abs_y = delta.y.abs();
        let abs_z = delta.z.abs();

        let constrained_pos = if abs_x >= abs_y && abs_x >= abs_z {
            Vec3::new(initial.x + delta.x, initial.y, initial.z)
        } else if abs_y >= abs_x && abs_y >= abs_z {
            Vec3::new(initial.x, initial.y + delta.y, initial.z)
        } else {
            Vec3::new(initial.x, initial.y, initial.z + delta.z)
        };

        log!(
            "Ctrl pressed - axis constrained: initial({:.2}, {:.2}, {:.2}) -> delta({:.2}, {:.2}, {:.2}) -> constrained({:.2}, {:.2}, {:.2})",
            initial.x,
            initial.y,
            initial.z,
            delta.x,
            delta.y,
            delta.z,
            constrained_pos.x,
            constrained_pos.y,
            constrained_pos.z
        );

        position.position = constrained_pos;
    } else {
        position.position = new_position;
    }
}

pub fn gizmo_update_selection_color(mesh: &mut LineMesh, selectable: &GizmoSelectable) -> bool {
    let yellow = [1.0, 1.0, 0.0];
    let highlight = [1.0, 1.0, 0.5];

    let mut new_colors: [[f32; 3]; 4] = [yellow, [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

    match selectable.selected_axis {
        GizmoAxis::None => {}
        GizmoAxis::Center => {
            new_colors[0] = highlight;
        }
        GizmoAxis::X => {
            new_colors[1] = [1.0, 0.5, 0.0];
        }
        GizmoAxis::Y => {
            new_colors[2] = [0.5, 1.0, 0.0];
        }
        GizmoAxis::Z => {
            new_colors[3] = [0.0, 0.5, 1.0];
        }
    }

    let mut changed = false;
    for (vertex, color) in mesh.vertices.iter_mut().zip(new_colors) {
        changed |= vertex.color != color;
        vertex.color = color;
    }
    changed
}

pub unsafe fn gizmo_create_static_buffers(
    mesh: &mut StaticLineMesh,
    backend: &mut thyllore_vulkan_core::VulkanBackend<'_>,
    memory_type: BufferMemoryType,
) -> Result<()> {
    backend.create_static_line_buffers(mesh, memory_type)
}

pub unsafe fn gizmo_create_buffers_for_every_slot(
    mesh: &mut LineMesh,
    backend: &mut thyllore_vulkan_core::VulkanBackend<'_>,
) -> Result<()> {
    for frame_slot in 0..FRAMES_IN_FLIGHT {
        backend.update_or_create_line_buffers(mesh, frame_slot)?;
    }
    Ok(())
}

pub unsafe fn gizmo_destroy_static_buffers(
    mesh: &mut StaticLineMesh,
    backend: &mut thyllore_vulkan_core::VulkanBackend<'_>,
) {
    backend.destroy_static_line_buffers(mesh);
}

pub unsafe fn gizmo_destroy_ray_buffers(
    ray: &mut LineMesh,
    backend: &mut thyllore_vulkan_core::VulkanBackend<'_>,
) {
    backend.destroy_line_buffers(ray);
}

pub unsafe fn run_vertical_lines_update(ctx: &mut crate::ecs::FrameContext) -> Result<()> {
    let model_tops: Vec<Vec3> = Vec::new();

    {
        let mut gizmo = ctx.light_gizmo_mut();
        let position = gizmo.position.clone();
        gizmo_update_vertical_lines(&mut gizmo.vertical_lines, &position, &model_tops);
    }

    let mut mesh_clone = {
        let gizmo = ctx.light_gizmo();
        gizmo.vertical_lines.clone()
    };

    {
        let frame_slot = ctx.frame_slot;
        let mut backend = ctx.create_backend();
        backend.update_or_create_line_buffers(&mut mesh_clone, frame_slot)?;
    }

    {
        let mut gizmo = ctx.light_gizmo_mut();
        gizmo.vertical_lines.vertex_buffer_handles = mesh_clone.vertex_buffer_handles;
        gizmo.vertical_lines.index_buffer_handles = mesh_clone.index_buffer_handles;
    }

    Ok(())
}

pub fn gizmo_update_vertical_lines(
    lines: &mut LineMesh,
    position: &GizmoPosition,
    model_positions: &[Vec3],
) {
    let orange = [1.0, 0.5, 0.0];

    lines.vertices.clear();
    lines.indices.clear();

    let gizmo_pos = position.position;

    lines.vertices.push(ColorVertex {
        pos: [gizmo_pos.x, gizmo_pos.y, gizmo_pos.z],
        color: orange,
    });
    lines.vertices.push(ColorVertex {
        pos: [gizmo_pos.x, 0.0, gizmo_pos.z],
        color: orange,
    });
    lines.indices.push(0);
    lines.indices.push(1);

    for (i, pos) in model_positions.iter().enumerate() {
        let base_index = (2 + i * 2) as u32;
        lines.vertices.push(ColorVertex {
            pos: [pos.x, pos.y, pos.z],
            color: orange,
        });
        lines.vertices.push(ColorVertex {
            pos: [pos.x, 0.0, pos.z],
            color: orange,
        });
        lines.indices.push(base_index);
        lines.indices.push(base_index + 1);
    }
}

pub unsafe fn gizmo_update_or_create_vertical_line_buffers(
    lines: &mut LineMesh,
    backend: &mut thyllore_vulkan_core::VulkanBackend<'_>,
    frame_slot: usize,
) -> Result<()> {
    backend.update_or_create_line_buffers(lines, frame_slot)
}

pub unsafe fn gizmo_destroy_vertical_line_buffers(
    lines: &mut LineMesh,
    backend: &mut thyllore_vulkan_core::VulkanBackend<'_>,
) {
    backend.destroy_line_buffers(lines);
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::vec3;
    use thyllore_math_core::coordinate_system::world_to_screen;

    const GIZMO_POSITION: Vec3 = vec3(1.0, 2.0, 3.0);
    const CAMERA_DISTANCE: f32 = 10.0;
    const AXIS_LENGTH: f32 = CAMERA_DISTANCE * AXIS_LENGTH_PER_CAMERA_DISTANCE;
    const PICK_THRESHOLD: f32 = PICK_THRESHOLD_PER_AXIS_LENGTH * AXIS_LENGTH;
    const SWAPCHAIN_EXTENT: (u32, u32) = (1600, 900);
    const FOV_Y: f32 = std::f32::consts::FRAC_PI_4;
    const NEAR_PLANE: f32 = 0.1;

    struct Click {
        direction_to_camera: Vec3,
        offset_from_gizmo: Vec3,
        billboard_click_rect: Option<[f32; 4]>,
    }

    fn project_to_screen(world_position: Vec3, camera_pos: Vec3, camera_direction: Vec3) -> Vec2 {
        let screen_size = Vec2::new(SWAPCHAIN_EXTENT.0 as f32, SWAPCHAIN_EXTENT.1 as f32);
        let view_mat = view(camera_pos, camera_direction, Vec3::Y);
        let proj = perspective_infinite_reverse(FOV_Y, screen_size.x / screen_size.y, NEAR_PLANE);

        world_to_screen(world_position, screen_size, view_mat, proj)
            .expect("clicked point is in front of the camera")
    }

    fn select_with_click(click: Click) -> (GizmoSelectable, GizmoDraggable) {
        let camera_pos = GIZMO_POSITION + click.direction_to_camera * CAMERA_DISTANCE;
        let camera_direction = -click.direction_to_camera;
        let mouse_pos = project_to_screen(
            GIZMO_POSITION + click.offset_from_gizmo,
            camera_pos,
            camera_direction,
        );

        let mut selectable = GizmoSelectable::default();
        let mut draggable = GizmoDraggable::default();
        gizmo_try_select(
            &GizmoPosition {
                position: GIZMO_POSITION,
            },
            &mut selectable,
            &mut draggable,
            mouse_pos,
            camera_pos,
            camera_direction,
            Vec3::Y,
            SWAPCHAIN_EXTENT,
            click.billboard_click_rect,
            FOV_Y,
            NEAR_PLANE,
        );

        (selectable, draggable)
    }

    fn select_axis_from_front(offset_from_gizmo: Vec3) -> GizmoAxis {
        let (selectable, _) = select_with_click(Click {
            direction_to_camera: Vec3::Z,
            offset_from_gizmo,
            billboard_click_rect: None,
        });
        selectable.selected_axis
    }

    #[test]
    fn test_try_select_picks_the_axis_under_the_cursor() {
        let half_axis = AXIS_LENGTH * 0.5;

        assert_eq!(select_axis_from_front(Vec3::X * half_axis), GizmoAxis::X);
        assert_eq!(select_axis_from_front(Vec3::Y * half_axis), GizmoAxis::Y);

        let (selectable, _) = select_with_click(Click {
            direction_to_camera: Vec3::X,
            offset_from_gizmo: Vec3::Z * half_axis,
            billboard_click_rect: None,
        });
        assert_eq!(selectable.selected_axis, GizmoAxis::Z);
    }

    #[test]
    fn test_try_select_prefers_the_nearest_axis_when_two_are_within_threshold() {
        let near = PICK_THRESHOLD * 0.2;
        let far = PICK_THRESHOLD * 0.6;

        assert_eq!(select_axis_from_front(vec3(far, near, 0.0)), GizmoAxis::X);
        assert_eq!(select_axis_from_front(vec3(near, far, 0.0)), GizmoAxis::Y);
    }

    #[test]
    fn test_try_select_picks_center_on_the_far_side_of_every_axis() {
        let direction_seeing_axes_in_a_half_plane = vec3(-1.0, -1.0, 2.0).normalize();
        let away_from_every_axis = vec3(-1.0, -1.0, -1.0).normalize();

        for threshold_fraction in [0.2, 0.5, 0.9] {
            let (selectable, _) = select_with_click(Click {
                direction_to_camera: direction_seeing_axes_in_a_half_plane,
                offset_from_gizmo: away_from_every_axis * PICK_THRESHOLD * threshold_fraction,
                billboard_click_rect: None,
            });

            assert_eq!(selectable.selected_axis, GizmoAxis::Center);
        }
    }

    #[test]
    fn test_try_select_billboard_click_overrides_the_axis() {
        let on_x_axis = Vec3::X * AXIS_LENGTH * 0.5;
        let whole_screen = [
            0.0,
            0.0,
            SWAPCHAIN_EXTENT.0 as f32,
            SWAPCHAIN_EXTENT.1 as f32,
        ];

        let (selectable, _) = select_with_click(Click {
            direction_to_camera: Vec3::Z,
            offset_from_gizmo: on_x_axis,
            billboard_click_rect: Some(whole_screen),
        });

        assert_eq!(selectable.selected_axis, GizmoAxis::Center);
    }

    #[test]
    fn test_try_select_rejects_a_click_past_the_axis_tip_or_beside_the_axis() {
        let past_the_tip = Vec3::X * (AXIS_LENGTH + PICK_THRESHOLD * 2.0);
        let beside_the_axis = vec3(AXIS_LENGTH * 0.5, PICK_THRESHOLD * 2.0, 0.0);

        for offset_from_gizmo in [past_the_tip, beside_the_axis] {
            let (selectable, draggable) = select_with_click(Click {
                direction_to_camera: Vec3::Z,
                offset_from_gizmo,
                billboard_click_rect: None,
            });

            assert!(!selectable.is_selected);
            assert_eq!(selectable.selected_axis, GizmoAxis::None);
            assert_eq!(draggable.drag_axis, GizmoAxis::None);
        }
    }

    #[test]
    fn test_try_select_starts_the_drag_from_the_gizmo_position() {
        let (selectable, draggable) = select_with_click(Click {
            direction_to_camera: Vec3::Z,
            offset_from_gizmo: Vec3::Y * AXIS_LENGTH * 0.5,
            billboard_click_rect: None,
        });

        assert!(selectable.is_selected);
        assert_eq!(draggable.drag_axis, GizmoAxis::Y);
        assert_eq!(draggable.initial_position, GIZMO_POSITION);
    }

    #[test]
    fn test_selection_color_reports_change_only_once_per_axis() {
        let mut mesh = LineMesh {
            vertices: vec![ColorVertex::default(); 4],
            ..Default::default()
        };
        let selectable = GizmoSelectable {
            is_selected: true,
            selected_axis: GizmoAxis::X,
        };

        assert!(gizmo_update_selection_color(&mut mesh, &selectable));
        assert!(!gizmo_update_selection_color(&mut mesh, &selectable));
    }
}
