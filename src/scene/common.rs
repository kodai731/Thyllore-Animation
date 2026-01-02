use crate::app::AppData;
use rust_rendering::vulkanr::buffer::*;
use rust_rendering::vulkanr::data as vulkan_data;
use rust_rendering::vulkanr::data::*;
use rust_rendering::vulkanr::descriptor::{RRDescriptorSet, RRRayQueryDescriptorSet};
use rust_rendering::vulkanr::device::*;
use rust_rendering::vulkanr::image::*;
use rust_rendering::vulkanr::raytracing::acceleration::RRAccelerationStructure;
use rust_rendering::vulkanr::vulkan::*;
use rust_rendering::math::*;
use rust_rendering::logger::logger::*;

use anyhow::Result;
use std::borrow::BorrowMut;
use std::mem::size_of;
use std::os::raw::c_void;

pub struct CubeModel {
    pub vertices: Vec<vulkan_data::Vertex>,
    pub indices: Vec<u32>,
}

impl CubeModel {
    pub fn new(size: f32) -> Self {
        let half = size / 2.0;

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let faces = [
            ([0.0, 0.0, 1.0], [
                [-half, -half,  half],
                [ half, -half,  half],
                [ half,  half,  half],
                [-half,  half,  half],
            ]),
            ([0.0, 0.0, -1.0], [
                [ half, -half, -half],
                [-half, -half, -half],
                [-half,  half, -half],
                [ half,  half, -half],
            ]),
            ([1.0, 0.0, 0.0], [
                [ half, -half,  half],
                [ half, -half, -half],
                [ half,  half, -half],
                [ half,  half,  half],
            ]),
            ([-1.0, 0.0, 0.0], [
                [-half, -half, -half],
                [-half, -half,  half],
                [-half,  half,  half],
                [-half,  half, -half],
            ]),
            ([0.0, 1.0, 0.0], [
                [-half,  half,  half],
                [ half,  half,  half],
                [ half,  half, -half],
                [-half,  half, -half],
            ]),
            ([0.0, -1.0, 0.0], [
                [-half, -half, -half],
                [ half, -half, -half],
                [ half, -half,  half],
                [-half, -half,  half],
            ]),
        ];

        let tex_coords = [
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0],
            [0.0, 0.0],
        ];

        let face_colors = [
            Vec4::new(1.0, 0.3, 0.3, 1.0), // +Z: 赤
            Vec4::new(0.3, 1.0, 0.3, 1.0), // -Z: 緑
            Vec4::new(0.3, 0.3, 1.0, 1.0), // +X: 青
            Vec4::new(1.0, 1.0, 0.3, 1.0), // -X: 黄
            Vec4::new(0.3, 1.0, 1.0, 1.0), // +Y: シアン
            Vec4::new(1.0, 0.3, 1.0, 1.0), // -Y: マゼンタ
        ];

        for (face_idx, (normal, positions)) in faces.iter().enumerate() {
            let base_index = vertices.len() as u32;
            let color = face_colors[face_idx];

            for (i, pos) in positions.iter().enumerate() {
                vertices.push(vulkan_data::Vertex {
                    pos: Vec3::new(pos[0], pos[1], pos[2]),
                    color,
                    tex_coord: Vec2::new(tex_coords[i][0], tex_coords[i][1]),
                    normal: Vec3::new(normal[0], normal[1], normal[2]),
                });
            }

            indices.push(base_index);
            indices.push(base_index + 1);
            indices.push(base_index + 2);
            indices.push(base_index);
            indices.push(base_index + 2);
            indices.push(base_index + 3);
        }

        Self { vertices, indices }
    }

    pub fn new_at_position(size: f32, position: [f32; 3]) -> Self {
        let mut cube = Self::new(size);
        for vertex in &mut cube.vertices {
            vertex.pos.x += position[0];
            vertex.pos.y += position[1];
            vertex.pos.z += position[2];
        }
        cube
    }
}

pub unsafe fn create_cube_rrdata(
    instance: &Instance,
    rrdevice: &RRDevice,
    data: &mut AppData,
    size: f32,
    position: [f32; 3],
) -> Result<RRData> {
    let cube = CubeModel::new_at_position(size, position);

    log!("Creating cube model: {} vertices, {} indices at ({}, {}, {})",
        cube.vertices.len(), cube.indices.len(), position[0], position[1], position[2]);

    let mut rrdata = RRData::new(instance, rrdevice, &data.rrswapchain);

    (rrdata.image, rrdata.image_memory, rrdata.mip_level) = create_texture_image_pixel(
        instance,
        rrdevice,
        data.rrcommand_pool.borrow_mut(),
        &vec![255u8, 255, 255, 255],
        1,
        1,
    )?;

    rrdata.image_view = create_image_view(
        rrdevice,
        rrdata.image,
        vk::Format::R8G8B8A8_SRGB,
        vk::ImageAspectFlags::COLOR,
        rrdata.mip_level,
    )?;

    rrdata.sampler = create_texture_sampler(rrdevice, rrdata.mip_level)?;

    rrdata.vertex_data.vertices = cube.vertices.clone();
    rrdata.vertex_data.indices = cube.indices.clone();

    rrdata.vertex_buffer = RRVertexBuffer::new(
        instance,
        rrdevice,
        &data.rrcommand_pool,
        (size_of::<vulkan_data::Vertex>() * rrdata.vertex_data.vertices.len()) as vk::DeviceSize,
        rrdata.vertex_data.vertices.as_ptr() as *const c_void,
        rrdata.vertex_data.vertices.len(),
    );

    rrdata.index_buffer = RRIndexBuffer::new(
        instance,
        rrdevice,
        &data.rrcommand_pool,
        (size_of::<u32>() * rrdata.vertex_data.indices.len()) as vk::DeviceSize,
        rrdata.vertex_data.indices.as_ptr() as *const c_void,
        rrdata.vertex_data.indices.len(),
    );

    Ok(rrdata)
}

pub unsafe fn load_cube_model(
    instance: &Instance,
    rrdevice: &RRDevice,
    data: &mut AppData,
    size: f32,
    position: [f32; 3],
) -> Result<()> {
    let rrdata = create_cube_rrdata(instance, rrdevice, data, size, position)?;
    data.model_descriptor_set.rrdata.push(rrdata);

    log!("Cube model loaded successfully. Total meshes: {}", data.model_descriptor_set.rrdata.len());
    Ok(())
}

pub unsafe fn cleanup_model_resources(
    rrdevice: &RRDevice,
    data: &mut AppData,
) {
    log!("Cleaning up model resources...");

    rrdevice.device.device_wait_idle().ok();

    if let Some(ref mut accel) = data.acceleration_structure {
        accel.destroy(&rrdevice.device);
        log!("Destroyed acceleration structure");
    }
    data.acceleration_structure = None;

    if let Some(ref mut gbuffer_desc) = data.gbuffer_descriptor_set {
        gbuffer_desc.rrdata.clear();
        log!("Cleared gbuffer_descriptor_set.rrdata (shared handles, no delete)");
    }

    for rrdata in &mut data.model_descriptor_set.rrdata {
        rrdata.delete(rrdevice);
    }
    data.model_descriptor_set.rrdata.clear();

    data.fbx_model.clear();
    data.animation_playing = false;
    data.animation_time = 0.0;

    log!("Model resources cleaned up");
}

pub unsafe fn rebuild_acceleration_structures(
    instance: &Instance,
    rrdevice: &RRDevice,
    data: &mut AppData,
) -> Result<()> {
    log!("Rebuilding acceleration structures...");

    let mut acceleration_structure = RRAccelerationStructure::new();

    for rrdata in &data.model_descriptor_set.rrdata {
        let blas = RRAccelerationStructure::create_blas(
            instance,
            rrdevice,
            &data.rrcommand_pool,
            &rrdata.vertex_buffer.buffer,
            rrdata.vertex_data.vertices.len() as u32,
            std::mem::size_of::<vulkan_data::Vertex>() as u32,
            &rrdata.index_buffer.buffer,
            rrdata.vertex_data.indices.len() as u32,
        )?;

        acceleration_structure.blas_list.push(blas);
        log!("Created BLAS for mesh");
    }

    if !acceleration_structure.blas_list.is_empty() {
        let tlas = RRAccelerationStructure::create_tlas(
            instance,
            rrdevice,
            &data.rrcommand_pool,
            &acceleration_structure.blas_list,
        )?;
        acceleration_structure.tlas = tlas;
        log!("Created TLAS with {} instances", acceleration_structure.blas_list.len());
    }

    data.acceleration_structure = Some(acceleration_structure);
    log!("Acceleration structures rebuilt successfully");
    Ok(())
}

pub unsafe fn replace_model_with_cube(
    instance: &Instance,
    rrdevice: &RRDevice,
    data: &mut AppData,
    size: f32,
    position: [f32; 3],
) -> Result<()> {
    cleanup_model_resources(rrdevice, data);

    let rrdata = create_cube_rrdata(instance, rrdevice, data, size, position)?;
    data.model_descriptor_set.rrdata.push(rrdata);

    if let Some(ref mut gbuffer_desc) = data.gbuffer_descriptor_set {
        for rrdata in &data.model_descriptor_set.rrdata {
            gbuffer_desc.rrdata.push(rrdata.clone());
        }
        RRDescriptorSet::create_descriptor_set(rrdevice, &data.rrswapchain, gbuffer_desc)?;
        log!("Updated gbuffer_descriptor_set with new model data");
    }

    rebuild_acceleration_structures(instance, rrdevice, data)?;

    if let Some(ref accel_struct) = data.acceleration_structure {
        if let Some(tlas) = accel_struct.tlas.acceleration_structure {
            if let Some(ref mut ray_query_desc) = data.ray_query_descriptor {
                ray_query_desc.update_tlas(rrdevice, tlas)?;
                log!("Updated ray_query_descriptor with new TLAS");
            }
        }
    }

    if let Some(ref billboard_texture) = data.light_gizmo_data.billboard_texture {
        data.billboard_descriptor_set
            .update_descriptor_sets(rrdevice, &data.rrswapchain, billboard_texture)?;
        log!("Re-updated billboard_descriptor_set after cube reload");
    }

    log!("Model replaced with cube. Size: {}, Position: ({}, {}, {})", size, position[0], position[1], position[2]);
    Ok(())
}
