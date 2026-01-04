use crate::app::AppData;
use crate::vulkanr::data as vulkan_data;
use crate::vulkanr::data::*;
use crate::vulkanr::descriptor::RRDescriptorSet;
use crate::vulkanr::device::*;
use crate::vulkanr::raytracing::acceleration::RRAccelerationStructure;
use crate::vulkanr::vulkan::*;
use crate::logger::logger::*;
use crate::scene::CubeModel;

use anyhow::Result;

pub unsafe fn cleanup_model_resources(
    rrdevice: &RRDevice,
    data: &mut AppData,
) {
    crate::log!("Cleaning up model resources...");

    rrdevice.device.device_wait_idle().ok();

    if let Some(ref mut accel) = data.raytracing.acceleration_structure {
        accel.destroy(&rrdevice.device);
        crate::log!("Destroyed acceleration structure");
    }
    data.raytracing.acceleration_structure = None;

    if let Some(ref mut gbuffer_desc) = data.raytracing.gbuffer_descriptor_set {
        gbuffer_desc.rrdata.clear();
        crate::log!("Cleared gbuffer_descriptor_set.rrdata (shared handles, no delete)");
    }

    for rrdata in &mut data.model_descriptor_set.rrdata {
        rrdata.delete(rrdevice);
    }
    data.model_descriptor_set.rrdata.clear();

    data.fbx_model.clear();
    data.animation_playing = false;
    data.animation_time = 0.0;

    crate::log!("Model resources cleaned up");
}

pub unsafe fn rebuild_acceleration_structures(
    instance: &Instance,
    rrdevice: &RRDevice,
    data: &mut AppData,
) -> Result<()> {
    crate::log!("Rebuilding acceleration structures...");

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
        crate::log!("Created BLAS for mesh");
    }

    if !acceleration_structure.blas_list.is_empty() {
        let tlas = RRAccelerationStructure::create_tlas(
            instance,
            rrdevice,
            &data.rrcommand_pool,
            &acceleration_structure.blas_list,
        )?;
        acceleration_structure.tlas = tlas;
        crate::log!("Created TLAS with {} instances", acceleration_structure.blas_list.len());
    }

    data.raytracing.acceleration_structure = Some(acceleration_structure);
    crate::log!("Acceleration structures rebuilt successfully");
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

    let mut cube = CubeModel::new_at_position(size, position);
    cube.initialize_gpu_resources(instance, rrdevice, &data.rrswapchain, &data.rrcommand_pool)?;

    if let Some(ref rrdata) = cube.rrdata {
        data.model_descriptor_set.rrdata.push(rrdata.clone());
    }

    RRDescriptorSet::create_descriptor_set(rrdevice, &data.rrswapchain, &mut data.model_descriptor_set)?;
    crate::log!("Updated model_descriptor_set with new cube data");

    if let Some(ref mut gbuffer_desc) = data.raytracing.gbuffer_descriptor_set {
        for rrdata in &data.model_descriptor_set.rrdata {
            gbuffer_desc.rrdata.push(rrdata.clone());
        }
        RRDescriptorSet::create_descriptor_set(rrdevice, &data.rrswapchain, gbuffer_desc)?;
        crate::log!("Updated gbuffer_descriptor_set with new model data");
    }

    rebuild_acceleration_structures(instance, rrdevice, data)?;

    if let Some(ref accel_struct) = data.raytracing.acceleration_structure {
        if let Some(tlas) = accel_struct.tlas.acceleration_structure {
            if let Some(ref mut ray_query_desc) = data.raytracing.ray_query_descriptor {
                ray_query_desc.update_tlas(rrdevice, tlas)?;
                crate::log!("Updated ray_query_descriptor with new TLAS");
            }
        }
    }

    if let Some(ref billboard_texture) = data.light_gizmo_data.billboard_texture {
        data.billboard.descriptor_set
            .update_descriptor_sets(rrdevice, &data.rrswapchain, billboard_texture)?;
        crate::log!("Re-updated billboard.descriptor_set after cube reload");
    }

    data.debug_view_data.cube_model = Some(cube);

    crate::log!("Model replaced with cube. Size: {}, Position: ({}, {}, {})", size, position[0], position[1], position[2]);
    Ok(())
}
