use std::mem::size_of;

use anyhow::Result;
use cgmath::{InnerSpace, Matrix4, Vector3, Vector4};
use vulkanalia::prelude::v1_0::*;

use crate::ecs::component::CameraState;
use crate::ecs::world::World;
use crate::scene::billboard::{BillboardData, BillboardTransform, BillboardVertex};
use crate::vulkanr::buffer::create_buffer;
use crate::vulkanr::command::RRCommandPool;
use crate::vulkanr::descriptor::RRBillboardDescriptorSet;
use crate::vulkanr::device::RRDevice;
use crate::vulkanr::image::RRImage;
use crate::vulkanr::vulkan::Instance;

pub fn create_billboard() -> BillboardData {
    let billboard_size = 0.5;
    let vertices = vec![
        BillboardVertex {
            pos: [-billboard_size, -billboard_size, 0.0],
            tex_coord: [0.0, 1.0],
        },
        BillboardVertex {
            pos: [billboard_size, -billboard_size, 0.0],
            tex_coord: [1.0, 1.0],
        },
        BillboardVertex {
            pos: [billboard_size, billboard_size, 0.0],
            tex_coord: [1.0, 0.0],
        },
        BillboardVertex {
            pos: [-billboard_size, billboard_size, 0.0],
            tex_coord: [0.0, 0.0],
        },
    ];

    let indices = vec![0, 1, 2, 0, 2, 3];

    BillboardData {
        pipeline_id: None,
        descriptor_set: RRBillboardDescriptorSet::default(),
        transform: None,
        object_index: 0,
        vertices,
        indices,
        vertex_buffer: None,
        vertex_buffer_memory: None,
        index_buffer: None,
        index_buffer_memory: None,
        texture: None,
    }
}

pub fn create_billboard_transform(position: Vector3<f32>) -> BillboardTransform {
    BillboardTransform {
        position,
        model_matrix: Matrix4::from_translation(position),
    }
}

pub fn billboard_system(world: &mut World, camera: &CameraState) {
    let billboards = world.query_billboards();

    for entity in billboards {
        let Some(behavior) = world.billboard_behaviors.get(&entity) else {
            continue;
        };

        if !behavior.always_face_camera {
            continue;
        }

        let Some(transform) = world.transforms.get_mut(&entity) else {
            continue;
        };

        let position = transform.translation;
        let to_camera = camera.position - position;

        if to_camera.magnitude() > 0.001 {
            let forward = to_camera.normalize();
            let up = Vector3::new(0.0, 1.0, 0.0);
            let right = up.cross(forward).normalize();
            let adjusted_up = forward.cross(right);

            transform.rotation =
                cgmath::Quaternion::from(cgmath::Matrix3::from_cols(right, adjusted_up, forward));
        }
    }
}

pub unsafe fn billboard_create_buffers(
    billboard: &mut BillboardData,
    instance: &Instance,
    rrdevice: &RRDevice,
    rrcommand_pool: &RRCommandPool,
) -> Result<()> {
    let vertex_buffer_size = (size_of::<BillboardVertex>() * billboard.vertices.len()) as u64;
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
    std::ptr::copy_nonoverlapping(billboard.vertices.as_ptr(), data.cast(), billboard.vertices.len());
    rrdevice.device.unmap_memory(vertex_buffer_memory);

    billboard.vertex_buffer = Some(vertex_buffer);
    billboard.vertex_buffer_memory = Some(vertex_buffer_memory);

    let index_buffer_size = (size_of::<u32>() * billboard.indices.len()) as u64;
    let (index_buffer, index_buffer_memory) = create_buffer(
        instance,
        rrdevice,
        index_buffer_size,
        vk::BufferUsageFlags::INDEX_BUFFER,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;

    let data = rrdevice.device.map_memory(
        index_buffer_memory,
        0,
        index_buffer_size,
        vk::MemoryMapFlags::empty(),
    )?;
    std::ptr::copy_nonoverlapping(billboard.indices.as_ptr(), data.cast(), billboard.indices.len());
    rrdevice.device.unmap_memory(index_buffer_memory);

    billboard.index_buffer = Some(index_buffer);
    billboard.index_buffer_memory = Some(index_buffer_memory);

    let texture_path = std::path::Path::new("assets/textures/lightIcon.png");
    billboard.texture = Some(
        RRImage::new_from_file(instance, rrdevice, rrcommand_pool, texture_path)
            .map_err(|e| anyhow::anyhow!("Failed to load billboard texture: {}", e))?,
    );

    Ok(())
}

pub fn billboard_transform_update_look_at(
    transform: &mut BillboardTransform,
    camera_position: Vector3<f32>,
    world_up: Vector3<f32>,
) {
    let forward = (camera_position - transform.position).normalize();
    let right = world_up.cross(forward).normalize();
    let up = forward.cross(right);

    let rotation = Matrix4::from_cols(
        right.extend(0.0),
        up.extend(0.0),
        forward.extend(0.0),
        Vector4::new(0.0, 0.0, 0.0, 1.0),
    );

    let translation = Matrix4::from_translation(transform.position);
    transform.model_matrix = translation * rotation;
}

pub fn billboard_transform_set_position(transform: &mut BillboardTransform, position: Vector3<f32>) {
    transform.position = position;
}
