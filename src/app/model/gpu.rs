use std::rc::Rc;

use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use super::texture::resolve_texture_pixels;
use crate::asset::AssetStorage;
use crate::ecs::world::World;
use crate::loader::ModelLoadResult;
use crate::vulkanr::command::RRCommandPool;
use crate::vulkanr::device::RRDevice;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;
use crate::vulkanr::resource::{MeshSource, TexturePixels};
use crate::vulkanr::swapchain::RRSwapchain;
use crate::vulkanr::vulkan::Instance;
use thyllore_vulkan_core::resource::raytracing_data::RayTracingData;

pub(super) unsafe fn upload_model_meshes(
    load_result: &ModelLoadResult,
    model_path: &str,
    instance: &Instance,
    device: &RRDevice,
    command_pool: &Rc<RRCommandPool>,
    swapchain: &RRSwapchain,
    graphics: &mut GraphicsResources,
) -> Result<()> {
    graphics.ensure_object_capacity(
        instance,
        device,
        swapchain.swapchain_images.len(),
        load_result.meshes.len(),
    )?;

    for loaded_mesh in &load_result.meshes {
        let texture = resolve_texture_pixels(loaded_mesh, model_path);
        let source = MeshSource {
            vertex_data: &loaded_mesh.vertex_data,
            base_vertices: &loaded_mesh.local_vertices,
            skin_data: loaded_mesh.skin_data.as_ref(),
            skeleton_id: loaded_mesh.skeleton_id,
            node_index: loaded_mesh.node_index,
            texture: TexturePixels {
                rgba: &texture.data,
                width: texture.width,
                height: texture.height,
            },
            base_color_factor: loaded_mesh.base_color_factor,
        };
        graphics.push_mesh(instance, device, command_pool, &source)?;
    }

    Ok(())
}

pub(super) unsafe fn upload_posed_meshes(
    instance: &Instance,
    device: &RRDevice,
    command_pool: &Rc<RRCommandPool>,
    graphics: &mut GraphicsResources,
    mesh_indices: &[usize],
) {
    for &mesh_index in mesh_indices {
        if let Err(e) = graphics.upload_mesh_vertices(instance, device, command_pool, mesh_index) {
            log!(
                "Failed to upload initial pose for mesh {}: {}",
                mesh_index,
                e
            );
        }
    }
}

pub(super) unsafe fn rebuild_scene_acceleration(
    instance: &Instance,
    device: &RRDevice,
    command_pool: &Rc<RRCommandPool>,
    graphics: &GraphicsResources,
    raytracing: &mut RayTracingData,
    world: &World,
    assets: &AssetStorage,
) -> Result<()> {
    let procedural_primitives =
        crate::app::raytracing::scene_build::collect_procedural_primitives(world);
    let mesh_transforms = crate::ecs::systems::collect_mesh_transforms(world, assets);
    crate::app::raytracing::scene_build::rebuild_acceleration_structures(
        instance,
        device,
        command_pool,
        graphics,
        raytracing,
        &procedural_primitives,
        &mesh_transforms,
    )
}

pub(super) unsafe fn release_model_gpu_resources(
    device: &RRDevice,
    graphics: &mut GraphicsResources,
    raytracing: &mut RayTracingData,
) -> Result<()> {
    device.device.device_wait_idle()?;

    if let Some(ref mut accel) = raytracing.acceleration_structure {
        accel.destroy(&device.device);
    }
    raytracing.acceleration_structure = None;

    graphics.clear_meshes(device);
    graphics.mesh_material_ids.clear();
    graphics.materials.clear_materials(&device.device);
    graphics.objects.reset_to_reserved();
    Ok(())
}
