use crate::app::AppData;
use crate::render::MaterialUBO;
use crate::scene::billboard::BillboardData;
use crate::scene::graphics_resource::MeshBuffer;
use crate::scene::CubeModel;
use crate::vulkanr::buffer::{RRIndexBuffer, RRVertexBuffer};
use crate::vulkanr::command::RRCommandPool;
use crate::vulkanr::data as vulkan_data;
use crate::vulkanr::data::VertexData;
use crate::vulkanr::device::*;
use crate::vulkanr::image::{
    create_image_view, create_texture_image_pixel, create_texture_sampler,
};
use crate::vulkanr::raytracing::acceleration::RRAccelerationStructure;
use crate::vulkanr::swapchain::RRSwapchain;
use crate::vulkanr::vulkan::*;

use anyhow::Result;
use std::mem::size_of;
use std::os::raw::c_void;
use std::rc::Rc;

pub unsafe fn cleanup_model_resources(rrdevice: &RRDevice, data: &mut AppData) {
    crate::log!("Cleaning up model resources...");

    rrdevice.device.device_wait_idle().ok();

    if let Some(ref mut accel) = data.raytracing.acceleration_structure {
        accel.destroy(&rrdevice.device);
        crate::log!("Destroyed acceleration structure");
    }
    data.raytracing.acceleration_structure = None;

    data.graphics_resources.clear_meshes(rrdevice);
    data.graphics_resources.mesh_material_ids.clear();
    data.graphics_resources
        .materials
        .clear_materials(&rrdevice.device);
    crate::log!("Cleared materials");

    crate::log!("Model resources cleaned up");
}

pub unsafe fn rebuild_acceleration_structures(
    instance: &Instance,
    rrdevice: &RRDevice,
    data: &mut AppData,
    rrcommand_pool: &Rc<RRCommandPool>,
) -> Result<()> {
    crate::log!("Rebuilding acceleration structures...");

    let mut acceleration_structure = RRAccelerationStructure::new();

    for mesh in &data.graphics_resources.meshes {
        let blas = RRAccelerationStructure::create_blas(
            instance,
            rrdevice,
            rrcommand_pool.as_ref(),
            &mesh.vertex_buffer.buffer,
            mesh.vertex_data.vertices.len() as u32,
            std::mem::size_of::<vulkan_data::Vertex>() as u32,
            &mesh.index_buffer.buffer,
            mesh.vertex_data.indices.len() as u32,
        )?;

        acceleration_structure.blas_list.push(blas);
        crate::log!("Created BLAS for mesh");
    }

    if !acceleration_structure.blas_list.is_empty() {
        let tlas = RRAccelerationStructure::create_tlas(
            instance,
            rrdevice,
            rrcommand_pool.as_ref(),
            &acceleration_structure.blas_list,
        )?;
        acceleration_structure.tlas = tlas;
        crate::log!(
            "Created TLAS with {} instances",
            acceleration_structure.blas_list.len()
        );
    }

    data.raytracing.acceleration_structure = Some(acceleration_structure);
    crate::log!("Acceleration structures rebuilt successfully");
    Ok(())
}

pub unsafe fn replace_model_with_cube(
    instance: &Instance,
    rrdevice: &RRDevice,
    data: &mut AppData,
    billboard: &mut BillboardData,
    rrcommand_pool: &Rc<RRCommandPool>,
    rrswapchain: &RRSwapchain,
    size: f32,
    position: [f32; 3],
) -> Result<()> {
    cleanup_model_resources(rrdevice, data);

    let cube = CubeModel::new_at_position(size, position);

    let mut mesh = MeshBuffer::default();

    (mesh.image, mesh.image_memory, mesh.mip_level) = create_texture_image_pixel(
        instance,
        rrdevice,
        rrcommand_pool,
        &vec![255u8, 255, 255, 255],
        1,
        1,
    )?;

    mesh.image_view = create_image_view(
        rrdevice,
        mesh.image,
        vk::Format::R8G8B8A8_SRGB,
        vk::ImageAspectFlags::COLOR,
        mesh.mip_level,
    )?;

    mesh.sampler = create_texture_sampler(rrdevice, mesh.mip_level)?;

    mesh.vertex_data = VertexData {
        vertices: cube.vertices.clone(),
        indices: cube.indices.clone(),
    };

    mesh.vertex_buffer = RRVertexBuffer::new(
        instance,
        rrdevice,
        rrcommand_pool.as_ref(),
        (size_of::<vulkan_data::Vertex>() * mesh.vertex_data.vertices.len()) as vk::DeviceSize,
        mesh.vertex_data.vertices.as_ptr() as *const c_void,
        mesh.vertex_data.vertices.len(),
    );

    mesh.index_buffer = RRIndexBuffer::new(
        instance,
        rrdevice,
        rrcommand_pool.as_ref(),
        (size_of::<u32>() * mesh.vertex_data.indices.len()) as u64,
        mesh.vertex_data.indices.as_ptr() as *const c_void,
        mesh.vertex_data.indices.len(),
    );

    mesh.object_index = data.graphics_resources.objects.allocate_slot();
    crate::log!("Allocated object_index {} for cube mesh", mesh.object_index);

    let material_id = data
        .graphics_resources
        .materials
        .create_material_with_texture(
            instance,
            rrdevice,
            "cube_material",
            mesh.image_view,
            mesh.sampler,
            MaterialUBO::default(),
        )?;
    data.graphics_resources.mesh_material_ids.push(material_id);
    crate::log!("Created material {} for cube", material_id);

    data.graphics_resources.meshes.push(mesh);
    crate::log!("Added cube mesh to graphics_resources.meshes");

    rebuild_acceleration_structures(instance, rrdevice, data, rrcommand_pool)?;

    if let Some(ref accel_struct) = data.raytracing.acceleration_structure {
        if let Some(tlas) = accel_struct.tlas.acceleration_structure {
            if let Some(ref mut ray_query_desc) = data.raytracing.ray_query_descriptor {
                ray_query_desc.update_tlas(rrdevice, tlas)?;
                crate::log!("Updated ray_query_descriptor with new TLAS");
            }
        }
    }

    {
        let texture_clone = billboard.texture.clone();
        if let Some(ref billboard_texture) = texture_clone {
            billboard.descriptor_set.update_descriptor_sets(
                rrdevice,
                rrswapchain,
                billboard_texture,
            )?;
            crate::log!("Re-updated billboard.descriptor_set after cube reload");
        }
    }

    data.debug_view_data.cube_model = Some(cube);

    crate::log!(
        "Model replaced with cube. Size: {}, Position: ({}, {}, {})",
        size,
        position[0],
        position[1],
        position[2]
    );
    Ok(())
}
