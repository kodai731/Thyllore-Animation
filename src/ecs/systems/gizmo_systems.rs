use std::mem::size_of;

use anyhow::Result;
use cgmath::{vec3, Deg, InnerSpace, Matrix3, Vector2, Vector3};
use vulkanalia::prelude::v1_0::*;

use crate::debugview::gizmo::grid::GridGizmoData;
use crate::debugview::gizmo::light::LightGizmoData;
use crate::ecs::component::{
    GizmoAxis, GizmoDraggable, GizmoMesh, GizmoPosition, GizmoRayToModel, GizmoSelectable,
    GizmoVertex, GizmoVerticalLines,
};
use crate::math::{
    coordinate_system::perspective, is_point_in_rect, ray_to_line_segment_distance,
    ray_to_point_distance, screen_to_world_ray, view, Vec2, Vec3, Vec4,
};
use crate::vulkanr::buffer::{copy_buffer, create_buffer};
use crate::scene::graphics_resource::GraphicsResources;
use crate::vulkanr::command::RRCommandPool;
use crate::vulkanr::core::Device;
use crate::vulkanr::data::Vertex;
use crate::vulkanr::device::RRDevice;
use crate::vulkanr::pipeline::RRPipeline;
use crate::vulkanr::vulkan::Instance;

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
            vertex_buffer: None,
            vertex_buffer_memory: None,
            index_buffer: None,
            index_buffer_memory: None,
        },
        position: GizmoPosition { position },
        selectable: GizmoSelectable::default(),
        draggable: GizmoDraggable::default(),
        ray_to_model: GizmoRayToModel::default(),
        vertical_lines: GizmoVerticalLines::default(),
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
            vertex_buffer: None,
            vertex_buffer_memory: None,
            index_buffer: None,
            index_buffer_memory: None,
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
    instance: &Instance,
    rrdevice: &RRDevice,
    rrcommand_pool: &RRCommandPool,
    use_staging: bool,
) -> Result<()> {
    let vertex_buffer_size = (size_of::<GizmoVertex>() * mesh.vertices.len()) as u64;

    if use_staging {
        let (staging_buffer, staging_buffer_memory) = create_buffer(
            instance,
            rrdevice,
            vertex_buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let data = rrdevice.device.map_memory(
            staging_buffer_memory,
            0,
            vertex_buffer_size,
            vk::MemoryMapFlags::empty(),
        )?;
        std::ptr::copy_nonoverlapping(mesh.vertices.as_ptr(), data.cast(), mesh.vertices.len());
        rrdevice.device.unmap_memory(staging_buffer_memory);

        let (vertex_buffer, vertex_buffer_memory) = create_buffer(
            instance,
            rrdevice,
            vertex_buffer_size,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        copy_buffer(
            rrdevice,
            rrcommand_pool,
            staging_buffer,
            vertex_buffer,
            vertex_buffer_size,
        )?;

        rrdevice.device.destroy_buffer(staging_buffer, None);
        rrdevice.device.free_memory(staging_buffer_memory, None);

        mesh.vertex_buffer = Some(vertex_buffer);
        mesh.vertex_buffer_memory = Some(vertex_buffer_memory);
    } else {
        let (vertex_buffer, vertex_buffer_memory) = create_buffer(
            instance,
            rrdevice,
            vertex_buffer_size,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let data = rrdevice.device.map_memory(
            vertex_buffer_memory,
            0,
            vertex_buffer_size,
            vk::MemoryMapFlags::empty(),
        )?;
        std::ptr::copy_nonoverlapping(mesh.vertices.as_ptr(), data.cast(), mesh.vertices.len());
        rrdevice.device.unmap_memory(vertex_buffer_memory);

        mesh.vertex_buffer = Some(vertex_buffer);
        mesh.vertex_buffer_memory = Some(vertex_buffer_memory);
    }

    let index_buffer_size = (size_of::<u32>() * mesh.indices.len()) as u64;
    let (staging_buffer, staging_buffer_memory) = create_buffer(
        instance,
        rrdevice,
        index_buffer_size,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;

    let data = rrdevice.device.map_memory(
        staging_buffer_memory,
        0,
        index_buffer_size,
        vk::MemoryMapFlags::empty(),
    )?;
    std::ptr::copy_nonoverlapping(mesh.indices.as_ptr(), data.cast(), mesh.indices.len());
    rrdevice.device.unmap_memory(staging_buffer_memory);

    let (index_buffer, index_buffer_memory) = create_buffer(
        instance,
        rrdevice,
        index_buffer_size,
        vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;

    copy_buffer(
        rrdevice,
        rrcommand_pool,
        staging_buffer,
        index_buffer,
        index_buffer_size,
    )?;

    rrdevice.device.destroy_buffer(staging_buffer, None);
    rrdevice.device.free_memory(staging_buffer_memory, None);

    mesh.index_buffer = Some(index_buffer);
    mesh.index_buffer_memory = Some(index_buffer_memory);

    Ok(())
}

pub unsafe fn gizmo_update_vertex_buffer(mesh: &GizmoMesh, rrdevice: &RRDevice) -> Result<()> {
    if let Some(vertex_buffer_memory) = mesh.vertex_buffer_memory {
        let vertex_buffer_size = (size_of::<GizmoVertex>() * mesh.vertices.len()) as u64;
        let data = rrdevice.device.map_memory(
            vertex_buffer_memory,
            0,
            vertex_buffer_size,
            vk::MemoryMapFlags::empty(),
        )?;
        std::ptr::copy_nonoverlapping(mesh.vertices.as_ptr(), data.cast(), mesh.vertices.len());
        rrdevice.device.unmap_memory(vertex_buffer_memory);
    }
    Ok(())
}

pub unsafe fn gizmo_destroy_buffers(mesh: &mut GizmoMesh, rrdevice: &RRDevice) {
    if let Some(vertex_buffer) = mesh.vertex_buffer {
        rrdevice.device.destroy_buffer(vertex_buffer, None);
    }
    if let Some(vertex_buffer_memory) = mesh.vertex_buffer_memory {
        rrdevice.device.free_memory(vertex_buffer_memory, None);
    }
    if let Some(index_buffer) = mesh.index_buffer {
        rrdevice.device.destroy_buffer(index_buffer, None);
    }
    if let Some(index_buffer_memory) = mesh.index_buffer_memory {
        rrdevice.device.free_memory(index_buffer_memory, None);
    }

    mesh.vertex_buffer = None;
    mesh.vertex_buffer_memory = None;
    mesh.index_buffer = None;
    mesh.index_buffer_memory = None;
}

pub fn gizmo_update_ray_to_model(
    ray: &mut GizmoRayToModel,
    position: &GizmoPosition,
    model_positions: &[Vector3<f32>],
) {
    if model_positions.is_empty() {
        ray.vertices.clear();
        ray.indices.clear();
        return;
    }

    let gizmo_pos = position.position;

    let mut min_x = f32::MAX;
    let mut max_x = f32::MIN;
    let mut min_y = f32::MAX;
    let mut max_y = f32::MIN;
    let mut min_z = f32::MAX;
    let mut max_z = f32::MIN;

    let mut closest_point = model_positions[0];
    let mut closest_index: usize = 0;
    let mut min_distance = (closest_point - gizmo_pos).magnitude();

    for (i, pos) in model_positions.iter().enumerate() {
        min_x = min_x.min(pos.x);
        max_x = max_x.max(pos.x);
        min_y = min_y.min(pos.y);
        max_y = max_y.max(pos.y);
        min_z = min_z.min(pos.z);
        max_z = max_z.max(pos.z);

        let distance = (*pos - gizmo_pos).magnitude();
        if distance < min_distance {
            min_distance = distance;
            closest_point = *pos;
            closest_index = i;
        }
    }

    let bright_yellow = Vec4::new(1.0, 1.0, 0.0, 1.0);
    let tex_coord = Vec2::new(0.0, 0.0);

    let light_pos = Vec3::new(gizmo_pos.x, gizmo_pos.y, gizmo_pos.z);
    let closest = Vec3::new(closest_point.x, closest_point.y, closest_point.z);

    let vertex_0 = Vertex::new(light_pos, bright_yellow, tex_coord);
    let vertex_1 = Vertex::new(closest, bright_yellow, tex_coord);

    ray.vertices = vec![vertex_0, vertex_1];
    ray.indices = vec![0, 1];

    static mut VERTEX_LOG_COUNTER: u32 = 0;
    unsafe {
        VERTEX_LOG_COUNTER += 1;
        if VERTEX_LOG_COUNTER % 120 == 1 {
            crate::log!("=== Ray to Model Debug ===");
            crate::log!("Model vertex count: {}", model_positions.len());
            crate::log!(
                "Model bounds: X[{:.2}, {:.2}], Y[{:.2}, {:.2}], Z[{:.2}, {:.2}]",
                min_x,
                max_x,
                min_y,
                max_y,
                min_z,
                max_z
            );
            crate::log!(
                "Gizmo position: ({:.2}, {:.2}, {:.2})",
                gizmo_pos.x,
                gizmo_pos.y,
                gizmo_pos.z
            );
            crate::log!("Closest vertex index: {}", closest_index);
            crate::log!(
                "Closest vertex position: ({:.2}, {:.2}, {:.2})",
                closest_point.x,
                closest_point.y,
                closest_point.z
            );
            crate::log!("Distance to closest: {:.2}", min_distance);
            crate::log!(
                "Ray line: [0]=Gizmo({:.2}, {:.2}, {:.2}) -> [1]=Model({:.2}, {:.2}, {:.2})",
                vertex_0.pos.x,
                vertex_0.pos.y,
                vertex_0.pos.z,
                vertex_1.pos.x,
                vertex_1.pos.y,
                vertex_1.pos.z
            );
            crate::log!("==========================");
        }
    }
}

pub unsafe fn gizmo_update_or_create_ray_buffers(
    ray: &mut GizmoRayToModel,
    instance: &Instance,
    rrdevice: &RRDevice,
) -> Result<()> {
    if ray.vertices.is_empty() {
        return Ok(());
    }

    let vertex_buffer_size = (size_of::<Vertex>() * ray.vertices.len()) as u64;

    if ray.vertex_buffer.is_none() {
        let (vertex_buffer, vertex_buffer_memory) = create_buffer(
            instance,
            rrdevice,
            vertex_buffer_size,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        ray.vertex_buffer = Some(vertex_buffer);
        ray.vertex_buffer_memory = Some(vertex_buffer_memory);
    }

    if let (Some(vertex_buffer_memory), Some(_vertex_buffer)) =
        (ray.vertex_buffer_memory, ray.vertex_buffer)
    {
        let data = rrdevice.device.map_memory(
            vertex_buffer_memory,
            0,
            vertex_buffer_size,
            vk::MemoryMapFlags::empty(),
        )?;
        std::ptr::copy_nonoverlapping(ray.vertices.as_ptr(), data.cast(), ray.vertices.len());
        rrdevice.device.unmap_memory(vertex_buffer_memory);
    }

    let index_buffer_size = (size_of::<u32>() * ray.indices.len()) as u64;

    if ray.index_buffer.is_none() {
        let (index_buffer, index_buffer_memory) = create_buffer(
            instance,
            rrdevice,
            index_buffer_size,
            vk::BufferUsageFlags::INDEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        ray.index_buffer = Some(index_buffer);
        ray.index_buffer_memory = Some(index_buffer_memory);
    }

    if let (Some(index_buffer_memory), Some(_index_buffer)) =
        (ray.index_buffer_memory, ray.index_buffer)
    {
        let data = rrdevice.device.map_memory(
            index_buffer_memory,
            0,
            index_buffer_size,
            vk::MemoryMapFlags::empty(),
        )?;
        std::ptr::copy_nonoverlapping(ray.indices.as_ptr(), data.cast(), ray.indices.len());
        rrdevice.device.unmap_memory(index_buffer_memory);
    }

    Ok(())
}

pub unsafe fn gizmo_destroy_ray_buffers(ray: &mut GizmoRayToModel, rrdevice: &RRDevice) {
    if let Some(vertex_buffer) = ray.vertex_buffer {
        rrdevice.device.destroy_buffer(vertex_buffer, None);
    }
    if let Some(vertex_buffer_memory) = ray.vertex_buffer_memory {
        rrdevice.device.free_memory(vertex_buffer_memory, None);
    }
    if let Some(index_buffer) = ray.index_buffer {
        rrdevice.device.destroy_buffer(index_buffer, None);
    }
    if let Some(index_buffer_memory) = ray.index_buffer_memory {
        rrdevice.device.free_memory(index_buffer_memory, None);
    }

    ray.vertex_buffer = None;
    ray.vertex_buffer_memory = None;
    ray.index_buffer = None;
    ray.index_buffer_memory = None;
}

pub fn gizmo_update_vertical_lines(
    lines: &mut GizmoVerticalLines,
    position: &GizmoPosition,
    model_positions: &[Vector3<f32>],
) {
    let orange = Vec4::new(1.0, 0.5, 0.0, 1.0);
    let tex_coord = Vec2::new(0.0, 0.0);

    lines.vertices.clear();
    lines.indices.clear();

    let gizmo_pos = position.position;
    let light_pos = Vec3::new(gizmo_pos.x, gizmo_pos.y, gizmo_pos.z);
    let light_ground = Vec3::new(gizmo_pos.x, 0.0, gizmo_pos.z);

    lines
        .vertices
        .push(Vertex::new(light_pos, orange, tex_coord));
    lines
        .vertices
        .push(Vertex::new(light_ground, orange, tex_coord));
    lines.indices.push(0);
    lines.indices.push(1);

    for (i, pos) in model_positions.iter().enumerate() {
        let top = Vec3::new(pos.x, pos.y, pos.z);
        let bottom = Vec3::new(pos.x, 0.0, pos.z);

        let base_index = (2 + i * 2) as u32;
        lines.vertices.push(Vertex::new(top, orange, tex_coord));
        lines.vertices.push(Vertex::new(bottom, orange, tex_coord));
        lines.indices.push(base_index);
        lines.indices.push(base_index + 1);
    }

    static mut LOG_COUNTER: u32 = 0;
    unsafe {
        LOG_COUNTER += 1;
        if LOG_COUNTER % 60 == 1 {
            crate::log!(
                "Vertical lines: gizmo=({:.1},{:.1},{:.1}), models={}, vertices={}, indices={}",
                light_pos.x,
                light_pos.y,
                light_pos.z,
                model_positions.len(),
                lines.vertices.len(),
                lines.indices.len()
            );
            for (i, pos) in model_positions.iter().enumerate() {
                crate::log!(
                    "  Model[{}] top: ({:.1},{:.1},{:.1})",
                    i,
                    pos.x,
                    pos.y,
                    pos.z
                );
            }
        }
    }
}

pub unsafe fn gizmo_update_or_create_vertical_line_buffers(
    lines: &mut GizmoVerticalLines,
    instance: &Instance,
    rrdevice: &RRDevice,
) -> Result<()> {
    if lines.vertices.is_empty() {
        return Ok(());
    }

    let vertex_buffer_size = (size_of::<Vertex>() * lines.vertices.len()) as u64;

    if lines.vertex_buffer.is_none() {
        let (vertex_buffer, vertex_buffer_memory) = create_buffer(
            instance,
            rrdevice,
            vertex_buffer_size.max(1024),
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        lines.vertex_buffer = Some(vertex_buffer);
        lines.vertex_buffer_memory = Some(vertex_buffer_memory);
    }

    if let Some(vertex_buffer_memory) = lines.vertex_buffer_memory {
        let data = rrdevice.device.map_memory(
            vertex_buffer_memory,
            0,
            vertex_buffer_size,
            vk::MemoryMapFlags::empty(),
        )?;
        std::ptr::copy_nonoverlapping(lines.vertices.as_ptr(), data.cast(), lines.vertices.len());
        rrdevice.device.unmap_memory(vertex_buffer_memory);
    }

    let index_buffer_size = (size_of::<u32>() * lines.indices.len()) as u64;

    if lines.index_buffer.is_none() {
        let (index_buffer, index_buffer_memory) = create_buffer(
            instance,
            rrdevice,
            index_buffer_size.max(256),
            vk::BufferUsageFlags::INDEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        lines.index_buffer = Some(index_buffer);
        lines.index_buffer_memory = Some(index_buffer_memory);
    }

    if let Some(index_buffer_memory) = lines.index_buffer_memory {
        let data = rrdevice.device.map_memory(
            index_buffer_memory,
            0,
            index_buffer_size,
            vk::MemoryMapFlags::empty(),
        )?;
        std::ptr::copy_nonoverlapping(lines.indices.as_ptr(), data.cast(), lines.indices.len());
        rrdevice.device.unmap_memory(index_buffer_memory);
    }

    Ok(())
}

pub unsafe fn gizmo_destroy_vertical_line_buffers(
    lines: &mut GizmoVerticalLines,
    rrdevice: &RRDevice,
) {
    if let Some(buffer) = lines.vertex_buffer {
        rrdevice.device.destroy_buffer(buffer, None);
    }
    if let Some(memory) = lines.vertex_buffer_memory {
        rrdevice.device.free_memory(memory, None);
    }
    if let Some(buffer) = lines.index_buffer {
        rrdevice.device.destroy_buffer(buffer, None);
    }
    if let Some(memory) = lines.index_buffer_memory {
        rrdevice.device.free_memory(memory, None);
    }

    lines.vertex_buffer = None;
    lines.vertex_buffer_memory = None;
    lines.index_buffer = None;
    lines.index_buffer_memory = None;
}

pub unsafe fn gizmo_draw_ray_with_pipeline(
    ray: &GizmoRayToModel,
    device: &Device,
    command_buffer: vk::CommandBuffer,
    pipeline: &RRPipeline,
    graphics_resources: &GraphicsResources,
    object_index: usize,
    image_index: usize,
) {
    let (vertex_buffer, index_buffer) = match (ray.vertex_buffer, ray.index_buffer) {
        (Some(vb), Some(ib)) => (vb, ib),
        _ => return,
    };

    device.cmd_bind_pipeline(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        pipeline.pipeline,
    );

    device.cmd_set_line_width(command_buffer, 1.0);
    device.cmd_bind_vertex_buffers(command_buffer, 0, &[vertex_buffer], &[0]);
    device.cmd_bind_index_buffer(command_buffer, index_buffer, 0, vk::IndexType::UINT32);

    let frame_set = graphics_resources.frame_set.sets[image_index];
    device.cmd_bind_descriptor_sets(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        pipeline.pipeline_layout,
        0,
        &[frame_set],
        &[],
    );

    let object_set_idx = graphics_resources
        .objects
        .get_set_index(image_index, object_index);
    let object_set = graphics_resources.objects.sets[object_set_idx];
    device.cmd_bind_descriptor_sets(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        pipeline.pipeline_layout,
        2,
        &[object_set],
        &[],
    );

    device.cmd_draw_indexed(command_buffer, ray.indices.len() as u32, 1, 0, 0, 0);
}

pub unsafe fn gizmo_draw_vertical_lines_with_pipeline(
    lines: &GizmoVerticalLines,
    device: &Device,
    command_buffer: vk::CommandBuffer,
    pipeline: &RRPipeline,
    graphics_resources: &GraphicsResources,
    object_index: usize,
    image_index: usize,
) {
    if lines.indices.is_empty() {
        return;
    }

    let (vertex_buffer, index_buffer) = match (lines.vertex_buffer, lines.index_buffer) {
        (Some(vb), Some(ib)) => (vb, ib),
        _ => return,
    };

    device.cmd_bind_pipeline(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        pipeline.pipeline,
    );

    device.cmd_set_line_width(command_buffer, 1.0);
    device.cmd_bind_vertex_buffers(command_buffer, 0, &[vertex_buffer], &[0]);
    device.cmd_bind_index_buffer(command_buffer, index_buffer, 0, vk::IndexType::UINT32);

    let frame_set = graphics_resources.frame_set.sets[image_index];
    device.cmd_bind_descriptor_sets(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        pipeline.pipeline_layout,
        0,
        &[frame_set],
        &[],
    );

    let object_set_idx = graphics_resources
        .objects
        .get_set_index(image_index, object_index);
    let object_set = graphics_resources.objects.sets[object_set_idx];
    device.cmd_bind_descriptor_sets(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        pipeline.pipeline_layout,
        2,
        &[object_set],
        &[],
    );

    device.cmd_draw_indexed(command_buffer, lines.indices.len() as u32, 1, 0, 0, 0);
}
