use std::rc::Rc;

use anyhow::Result;
use cgmath::SquareMatrix;
use vulkanalia::prelude::v1_0::*;

use crate::app::AppData;
use crate::asset::AssetStorage;
use crate::ecs::resource::billboard::{BillboardData, BillboardRenderState};
use crate::ecs::systems::collect_mesh_transforms;
use crate::ecs::world::World;
use crate::vulkanr::command::RRCommandPool;
use crate::vulkanr::data as vulkan_data;
use crate::vulkanr::device::RRDevice;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;
use crate::vulkanr::swapchain::RRSwapchain;
use crate::vulkanr::vulkan::Instance;
use thyllore_math_core::AffineRows3x4;
use thyllore_vulkan_core::raytracing::{BlasGeometry, GpuPrimitive, RRAccelerationStructure};
use thyllore_vulkan_core::resource::raytracing_data::RayTracingData;

pub fn collect_procedural_primitives(world: &World) -> Vec<GpuPrimitive<'static>> {
    crate::hooks::gpu_primitive::collect_all(world)
}

pub unsafe fn rebuild_acceleration_structures(
    instance: &Instance,
    device: &RRDevice,
    command_pool: &Rc<RRCommandPool>,
    graphics: &GraphicsResources,
    raytracing: &mut RayTracingData,
    procedural_primitives: &[GpuPrimitive<'static>],
    mesh_transforms: &[cgmath::Matrix4<f32>],
) -> Result<()> {
    log!("Rebuilding acceleration structures...");

    let mut acceleration_structure = RRAccelerationStructure::new();

    // Collect vertex_buffers in the same order as BLAS creation
    let vertex_buffers: Vec<_> = graphics
        .meshes
        .iter()
        .filter(|mesh| mesh.render_to_gbuffer)
        .map(|mesh| {
            (
                &mesh.vertex_buffer.buffer,
                mesh.vertex_data.vertices.len() as u32,
                std::mem::size_of::<vulkan_data::Vertex>() as u32,
                &mesh.index_buffer.buffer,
                mesh.vertex_data.indices.len() as u32,
            )
        })
        .collect();

    for (mesh_index, mesh) in graphics.meshes.iter().enumerate() {
        if !mesh.render_to_gbuffer {
            continue;
        }

        let mut blas = RRAccelerationStructure::create_blas(
            instance,
            device,
            command_pool.as_ref(),
            &mesh.vertex_buffer.buffer,
            mesh.vertex_data.vertices.len() as u32,
            std::mem::size_of::<vulkan_data::Vertex>() as u32,
            &mesh.index_buffer.buffer,
            mesh.vertex_data.indices.len() as u32,
        )?;

        let model = mesh_transforms
            .get(mesh_index)
            .copied()
            .unwrap_or_else(cgmath::Matrix4::identity);
        blas.transform = vk::TransformMatrixKHR {
            matrix: AffineRows3x4::from_mat4(model).rows,
        };

        acceleration_structure.blas_list.push(blas);
        log!("Created BLAS for mesh");
    }

    for primitive in procedural_primitives {
        if let BlasGeometry::ProceduralAabb { aabb } = &primitive.geometry {
            let blas = RRAccelerationStructure::create_procedural_blas(
                instance,
                device,
                command_pool.as_ref(),
                &primitive.model,
                *aabb,
            )?;
            acceleration_structure.procedural_blas.push(blas);
        }
    }

    let tlas = RRAccelerationStructure::create_tlas(
        instance,
        device,
        command_pool.as_ref(),
        &acceleration_structure.blas_list,
        &acceleration_structure.procedural_blas,
    )?;
    acceleration_structure.tlas = tlas;
    log!(
        "Created TLAS with {} mesh + {} procedural instances",
        acceleration_structure.blas_list.len(),
        acceleration_structure.procedural_blas.len()
    );

    acceleration_structure.fill_hit_shading_table(
        instance,
        device,
        &vertex_buffers,
        procedural_primitives,
    )?;

    raytracing.acceleration_structure = Some(acceleration_structure);
    log!("Acceleration structures rebuilt successfully");
    Ok(())
}

pub unsafe fn rebuild_acceleration_structures_from_data(
    instance: &Instance,
    rrdevice: &RRDevice,
    data: &mut AppData,
    rrcommand_pool: &Rc<RRCommandPool>,
) -> Result<()> {
    let procedural_primitives = collect_procedural_primitives(&data.ecs_world);
    let mesh_transforms = collect_mesh_transforms(&data.ecs_world, &data.ecs_assets);
    rebuild_acceleration_structures(
        instance,
        rrdevice,
        rrcommand_pool,
        &data.graphics_resources,
        &mut data.raytracing,
        &procedural_primitives,
        &mesh_transforms,
    )
}

pub unsafe fn update_ray_query_descriptor(
    device: &RRDevice,
    raytracing: &mut RayTracingData,
) -> Result<()> {
    raytracing.bind_ray_query_tlas(device)?;
    if raytracing.has_valid_tlas() {
        log!("Updated ray_query_descriptor with new TLAS");
    }
    Ok(())
}

pub unsafe fn update_billboard_descriptor(
    device: &RRDevice,
    swapchain: &RRSwapchain,
    billboard: &mut BillboardData,
) -> Result<()> {
    let BillboardRenderState {
        descriptor_set,
        texture,
    } = &mut billboard.render_state;
    if let Some(billboard_texture) = texture.as_ref() {
        descriptor_set.update_descriptor_sets(device, swapchain, billboard_texture)?;
        log!("Re-updated billboard.render_state.descriptor_set after model reload");
    }
    Ok(())
}
