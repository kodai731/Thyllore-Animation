use std::path::Path;
use std::rc::Rc;

use anyhow::Result;
use thyllore_avatar_core::material::systems::texture_remap_io::resolve_remapped_texture;
use vulkanalia::prelude::v1_0::*;

use crate::ecs::systems::load_model_texture_remap;
use crate::loader::{resolve_mesh_texture, DecodedTextureFiles, ModelLoadResult};
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

    let texture_remap = load_model_texture_remap(model_path);
    let mut decoded_files = DecodedTextureFiles::new();

    for loaded_mesh in &load_result.meshes {
        let remapped_file = resolve_remapped_texture(
            &texture_remap,
            &loaded_mesh.material_name,
            Path::new(model_path),
        );
        let texture = resolve_mesh_texture(
            loaded_mesh,
            Path::new(model_path),
            remapped_file,
            &mut decoded_files,
        );
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
            material_name: &loaded_mesh.material_name,
            base_color_factor: loaded_mesh.base_color_factor,
            morph: &loaded_mesh.morph,
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
