use anyhow::Result;
use cgmath::{vec3, Deg, InnerSpace, Matrix3, Vector2, Vector3};

use crate::debugview::gizmo::grid::GridGizmoData;
use crate::debugview::gizmo::light::LightGizmoData;
use crate::ecs::component::{
    GizmoAxis, GizmoDraggable, GizmoMesh, GizmoPosition, GizmoSelectable, GizmoVertex, LineMesh,
};
use crate::math::{
    coordinate_system::perspective, is_point_in_rect, ray_to_line_segment_distance,
    ray_to_point_distance, screen_to_world_ray, view,
};
use crate::render::{IndexBufferHandle, RenderBackend, VertexBufferHandle};

pub fn create_light_gizmo(position: Vector3<f32>) -> LightGizmoData {
    let axis_length = 1.0;
    let yellow = [1.0, 1.0, 0.0];

    let vertices = vec![
        GizmoVertex {
            pos: [0.0, 0.0, 0.0],
            color: yellow,
        },
        GizmoVertex {
            pos: [axis_length, 0.0, 0.0],
            color: [1.0, 0.0, 0.0],
        },
        GizmoVertex {
            pos: [0.0, axis_length, 0.0],
            color: [0.0, 1.0, 0.0],
        },
        GizmoVertex {
            pos: [0.0, 0.0, axis_length],
            color: [0.0, 0.0, 1.0],
        },
    ];

    let indices = vec![0, 1, 0, 2, 0, 3];

    LightGizmoData {
        mesh: GizmoMesh {
            pipeline_id: None,
            object_index: 0,
            vertices,
            indices,
            vertex_buffer_handle: VertexBufferHandle::INVALID,
            index_buffer_handle: IndexBufferHandle::INVALID,
        },
        position: GizmoPosition { position },
        selectable: GizmoSelectable::default(),
        draggable: GizmoDraggable::default(),
        ray_to_model: LineMesh::default(),
        vertical_lines: LineMesh::default(),
    }
}

pub fn create_grid_gizmo() -> GridGizmoData {
    let axis_length = 0.15;

    let vertices = vec![
        GizmoVertex {
            pos: [0.0, 0.0, 0.0],
            color: [1.0, 1.0, 1.0],
        },
        GizmoVertex {
            pos: [axis_length, 0.0, 0.0],
            color: [1.0, 0.0, 0.0],
        },
        GizmoVertex {
            pos: [0.0, axis_length, 0.0],
            color: [0.0, 1.0, 0.0],
        },
        GizmoVertex {
            pos: [0.0, 0.0, axis_length],
            color: [0.0, 0.0, 1.0],
        },
    ];

    let indices = vec![0, 1, 0, 2, 0, 3];

    GridGizmoData {
        mesh: GizmoMesh {
            pipeline_id: None,
            object_index: 0,
            vertices,
            indices,
            vertex_buffer_handle: VertexBufferHandle::INVALID,
            index_buffer_handle: IndexBufferHandle::INVALID,
        },
    }
}

pub fn gizmo_try_select(
    position: &GizmoPosition,
    selectable: &mut GizmoSelectable,
    draggable: &mut GizmoDraggable,
    mouse_pos: Vector2<f32>,
    camera_pos: Vector3<f32>,
    camera_direction: Vector3<f32>,
    camera_up: Vector3<f32>,
    swapchain_extent: (u32, u32),
    billboard_click_rect: Option<[f32; 4]>,
) {
    let light_pos = position.position;
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
    let mut selected_axis = GizmoAxis::None;

    if billboard_clicked {
        selected_axis = GizmoAxis::Center;
        min_distance = 0.0;
    } else {
        if center_distance < threshold {
            selected_axis = GizmoAxis::Center;
        }

        if x_distance < threshold && x_distance < min_distance {
            min_distance = x_distance;
            selected_axis = GizmoAxis::X;
        }

        if y_distance < threshold && y_distance < min_distance {
            min_distance = y_distance;
            selected_axis = GizmoAxis::Y;
        }

        if z_distance < threshold && z_distance < min_distance {
            let _ = min_distance;
            selected_axis = GizmoAxis::Z;
        }
    }

    if selected_axis != GizmoAxis::None {
        selectable.is_selected = true;
        selectable.selected_axis = selected_axis;
        draggable.drag_axis = selected_axis;
        draggable.initial_position = light_pos;

        let drag_depth = (light_pos - camera_pos).magnitude();
        crate::log!(
            "Gizmo selected - axis: {:?}, depth: {:.2}",
            selected_axis,
            drag_depth
        );

        draggable.just_selected = true;
    }
}

pub fn gizmo_update_position(position: &mut GizmoPosition, new_position: Vector3<f32>) {
    position.position = new_position;
}

pub fn gizmo_reset_selection(selectable: &mut GizmoSelectable, draggable: &mut GizmoDraggable) {
    selectable.is_selected = false;
    selectable.selected_axis = GizmoAxis::None;
    draggable.drag_axis = GizmoAxis::None;
    draggable.just_selected = false;
    draggable.initial_position = Vector3::new(0.0, 0.0, 0.0);
}

pub fn gizmo_sync_position(position: &mut GizmoPosition, source_position: Vector3<f32>) {
    if position.position.x != source_position.x
        || position.position.y != source_position.y
        || position.position.z != source_position.z
    {
        crate::log!("GizmoPosition: syncing from source");
        crate::log!(
            "  Before: ({:.2}, {:.2}, {:.2})",
            position.position.x,
            position.position.y,
            position.position.z
        );
        crate::log!(
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
    new_position: Vector3<f32>,
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
            Vector3::new(initial.x + delta.x, initial.y, initial.z)
        } else if abs_y >= abs_x && abs_y >= abs_z {
            Vector3::new(initial.x, initial.y + delta.y, initial.z)
        } else {
            Vector3::new(initial.x, initial.y, initial.z + delta.z)
        };

        crate::log!(
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

pub fn gizmo_update_selection_color(mesh: &mut GizmoMesh, selectable: &GizmoSelectable) {
    let yellow = [1.0, 1.0, 0.0];
    let highlight = [1.0, 1.0, 0.5];

    mesh.vertices[0].color = yellow;
    mesh.vertices[1].color = [1.0, 0.0, 0.0];
    mesh.vertices[2].color = [0.0, 1.0, 0.0];
    mesh.vertices[3].color = [0.0, 0.0, 1.0];

    match selectable.selected_axis {
        GizmoAxis::None => {}
        GizmoAxis::Center => {
            mesh.vertices[0].color = highlight;
        }
        GizmoAxis::X => {
            mesh.vertices[1].color = [1.0, 0.5, 0.0];
        }
        GizmoAxis::Y => {
            mesh.vertices[2].color = [0.5, 1.0, 0.0];
        }
        GizmoAxis::Z => {
            mesh.vertices[3].color = [0.0, 0.5, 1.0];
        }
    }
}

pub fn gizmo_update_rotation(mesh: &mut GizmoMesh, rotation_matrix: &Matrix3<f32>) {
    let axis_length = 0.15;

    let x_axis = rotation_matrix * vec3(axis_length, 0.0, 0.0);
    let y_axis = rotation_matrix * vec3(0.0, axis_length, 0.0);
    let z_axis = rotation_matrix * vec3(0.0, 0.0, axis_length);

    mesh.vertices[1].pos = [x_axis.x, x_axis.y, x_axis.z];
    mesh.vertices[2].pos = [y_axis.x, y_axis.y, y_axis.z];
    mesh.vertices[3].pos = [z_axis.x, z_axis.y, z_axis.z];
}

pub unsafe fn gizmo_create_buffers(
    mesh: &mut GizmoMesh,
    backend: &mut dyn RenderBackend,
    use_staging: bool,
) -> Result<()> {
    backend.create_gizmo_buffers(mesh, use_staging)
}

pub unsafe fn gizmo_update_vertex_buffer(
    mesh: &GizmoMesh,
    backend: &dyn RenderBackend,
) -> Result<()> {
    backend.update_gizmo_vertex_buffer(mesh)
}

pub unsafe fn gizmo_destroy_buffers(mesh: &mut GizmoMesh, backend: &mut dyn RenderBackend) {
    backend.destroy_gizmo_buffers(mesh);
}

pub fn gizmo_update_ray_to_model(
    ray: &mut LineMesh,
    position: &GizmoPosition,
    model_positions: &[Vector3<f32>],
) {
    if model_positions.is_empty() {
        ray.vertices.clear();
        ray.indices.clear();
        return;
    }

    let gizmo_pos = position.position;

    let mut closest_point = model_positions[0];
    let mut min_distance = (closest_point - gizmo_pos).magnitude();

    for pos in model_positions.iter() {
        let distance = (*pos - gizmo_pos).magnitude();
        if distance < min_distance {
            min_distance = distance;
            closest_point = *pos;
        }
    }

    let bright_yellow = [1.0, 1.0, 0.0];

    let vertex_0 = GizmoVertex {
        pos: [gizmo_pos.x, gizmo_pos.y, gizmo_pos.z],
        color: bright_yellow,
    };
    let vertex_1 = GizmoVertex {
        pos: [closest_point.x, closest_point.y, closest_point.z],
        color: bright_yellow,
    };

    ray.vertices = vec![vertex_0, vertex_1];
    ray.indices = vec![0, 1];
}

pub unsafe fn gizmo_update_or_create_ray_buffers(
    ray: &mut LineMesh,
    backend: &mut dyn RenderBackend,
) -> Result<()> {
    backend.update_or_create_line_buffers(ray)
}

pub unsafe fn gizmo_destroy_ray_buffers(ray: &mut LineMesh, backend: &mut dyn RenderBackend) {
    backend.destroy_line_buffers(ray);
}

pub fn gizmo_update_vertical_lines(
    lines: &mut LineMesh,
    position: &GizmoPosition,
    model_positions: &[Vector3<f32>],
) {
    let orange = [1.0, 0.5, 0.0];

    lines.vertices.clear();
    lines.indices.clear();

    let gizmo_pos = position.position;

    lines.vertices.push(GizmoVertex {
        pos: [gizmo_pos.x, gizmo_pos.y, gizmo_pos.z],
        color: orange,
    });
    lines.vertices.push(GizmoVertex {
        pos: [gizmo_pos.x, 0.0, gizmo_pos.z],
        color: orange,
    });
    lines.indices.push(0);
    lines.indices.push(1);

    for (i, pos) in model_positions.iter().enumerate() {
        let base_index = (2 + i * 2) as u32;
        lines.vertices.push(GizmoVertex {
            pos: [pos.x, pos.y, pos.z],
            color: orange,
        });
        lines.vertices.push(GizmoVertex {
            pos: [pos.x, 0.0, pos.z],
            color: orange,
        });
        lines.indices.push(base_index);
        lines.indices.push(base_index + 1);
    }
}

pub unsafe fn gizmo_update_or_create_vertical_line_buffers(
    lines: &mut LineMesh,
    backend: &mut dyn RenderBackend,
) -> Result<()> {
    backend.update_or_create_line_buffers(lines)
}

pub unsafe fn gizmo_destroy_vertical_line_buffers(
    lines: &mut LineMesh,
    backend: &mut dyn RenderBackend,
) {
    backend.destroy_line_buffers(lines);
}

