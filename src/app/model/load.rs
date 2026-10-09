use std::rc::Rc;

use anyhow::{anyhow, Result};

use super::{caches, cleanup, clips, entities, gpu, initial_pose, nodes};
use crate::asset::AssetStorage;
use crate::ecs::component::GlbSource;
use crate::ecs::world::{Entity, World};
use crate::hooks::model_load::{run_model_load_hooks, LoadedModel};
use crate::loader::fbx::FbxModel;
use crate::loader::ModelLoadResult;
use crate::vulkanr::command::RRCommandPool;
use crate::vulkanr::device::RRDevice;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;
use crate::vulkanr::swapchain::RRSwapchain;
use crate::vulkanr::vulkan::Instance;
use thyllore_vulkan_core::resource::raytracing_data::RayTracingData;

pub unsafe fn load_model_from_file_system(
    path: &str,
    instance: &Instance,
    device: &RRDevice,
    command_pool: &Rc<RRCommandPool>,
    swapchain: &RRSwapchain,
    graphics: &mut GraphicsResources,
    raytracing: &mut RayTracingData,
    world: &mut World,
    assets: &mut AssetStorage,
    scene_will_provide_clips: bool,
) -> Result<()> {
    log!("=== Loading model from path: {} ===", path);

    let (load_result, fbx_model) = read_model_file(path)?;
    replace_scene_model(
        &load_result,
        path,
        instance,
        device,
        command_pool,
        swapchain,
        graphics,
        raytracing,
        world,
        assets,
        scene_will_provide_clips,
        fbx_model,
    )?;

    log!("=== Model loaded successfully ===");
    Ok(())
}

#[cfg(feature = "auto-rig")]
pub unsafe fn load_model_from_file_system_with_result(
    load_result: &ModelLoadResult,
    model_name: &str,
    instance: &Instance,
    device: &RRDevice,
    command_pool: &Rc<RRCommandPool>,
    swapchain: &RRSwapchain,
    graphics: &mut GraphicsResources,
    raytracing: &mut RayTracingData,
    world: &mut World,
    assets: &mut AssetStorage,
    scene_will_provide_clips: bool,
    fbx_model: Option<FbxModel>,
) -> Result<Entity> {
    let parent_entity = replace_scene_model(
        load_result,
        model_name,
        instance,
        device,
        command_pool,
        swapchain,
        graphics,
        raytracing,
        world,
        assets,
        scene_will_provide_clips,
        fbx_model,
    )?;

    log!("=== Model loaded successfully ===");
    Ok(parent_entity)
}

pub unsafe fn load_model_additive(
    path: &str,
    instance: &Instance,
    device: &RRDevice,
    command_pool: &Rc<RRCommandPool>,
    swapchain: &RRSwapchain,
    graphics: &mut GraphicsResources,
    raytracing: &mut RayTracingData,
    world: &mut World,
    assets: &mut AssetStorage,
) -> Result<()> {
    let (load_result, _fbx_model) = read_model_file(path)?;
    let part_name = entities::model_display_name(path, "part");

    let parent_entity = append_model_to_scene(
        &load_result,
        &part_name,
        instance,
        device,
        command_pool,
        swapchain,
        graphics,
        raytracing,
        world,
        assets,
    )?;
    world.insert_component(parent_entity, GlbSource::FilePath(path.to_string()));

    Ok(())
}

pub(crate) unsafe fn append_model_to_scene(
    load_result: &ModelLoadResult,
    part_name: &str,
    instance: &Instance,
    device: &RRDevice,
    command_pool: &Rc<RRCommandPool>,
    swapchain: &RRSwapchain,
    graphics: &mut GraphicsResources,
    raytracing: &mut RayTracingData,
    world: &mut World,
    assets: &mut AssetStorage,
) -> Result<Entity> {
    let mesh_index_offset = graphics.meshes.len();
    gpu::upload_model_meshes(
        load_result,
        part_name,
        instance,
        device,
        command_pool,
        swapchain,
        graphics,
    )?;
    gpu::rebuild_scene_acceleration(
        instance,
        device,
        command_pool,
        graphics,
        raytracing,
        world,
        assets,
    )?;

    let parent_entity = entities::spawn_model_part_entity(part_name, world);
    entities::spawn_mesh_entities(
        part_name,
        graphics,
        world,
        assets,
        parent_entity,
        mesh_index_offset..graphics.meshes.len(),
    );

    log!(
        "Additively loaded '{}': {} meshes, total entities={}",
        part_name,
        load_result.meshes.len(),
        world.entity_count()
    );

    Ok(parent_entity)
}

unsafe fn read_model_file(path: &str) -> Result<(ModelLoadResult, Option<FbxModel>)> {
    let path_lower = path.to_lowercase();

    if path_lower.ends_with(".fbx") {
        let (result, fbx_model) = crate::loader::fbx::load_fbx_to_graphics_resources(path)?;
        Ok((ModelLoadResult::from_fbx(result), Some(fbx_model)))
    } else if caches::is_gltf_path(path) {
        let result = crate::loader::gltf::load_gltf_file(path)?;
        Ok((ModelLoadResult::from_gltf(result), None))
    } else {
        Err(anyhow!(
            "Unsupported file format. Only FBX and glTF/GLB are supported."
        ))
    }
}

unsafe fn replace_scene_model(
    load_result: &ModelLoadResult,
    model_name: &str,
    instance: &Instance,
    device: &RRDevice,
    command_pool: &Rc<RRCommandPool>,
    swapchain: &RRSwapchain,
    graphics: &mut GraphicsResources,
    raytracing: &mut RayTracingData,
    world: &mut World,
    assets: &mut AssetStorage,
    scene_will_provide_clips: bool,
    fbx_model: Option<FbxModel>,
) -> Result<Entity> {
    cleanup::reset_scene_model(device, graphics, raytracing, world, assets)?;
    caches::insert_model_caches(world, model_name, fbx_model);
    clips::register_imported_animation(world, load_result, assets);
    nodes::replace_node_assets(world, assets, load_result);

    gpu::upload_model_meshes(
        load_result,
        model_name,
        instance,
        device,
        command_pool,
        swapchain,
        graphics,
    )?;
    let posed_meshes = initial_pose::apply_initial_pose(world, assets, graphics, load_result);
    gpu::upload_posed_meshes(instance, device, command_pool, graphics, &posed_meshes);
    gpu::rebuild_scene_acceleration(
        instance,
        device,
        command_pool,
        graphics,
        raytracing,
        world,
        assets,
    )?;

    let parent_entity = entities::spawn_model_entities(
        model_name,
        graphics,
        world,
        assets,
        load_result,
        scene_will_provide_clips,
    );
    if is_model_file_path(model_name) {
        world.insert_component(parent_entity, GlbSource::FilePath(model_name.to_string()));
    }
    run_model_load_hooks(
        world,
        assets,
        &LoadedModel {
            entity: parent_entity,
            load_result,
        },
    );

    Ok(parent_entity)
}

fn is_model_file_path(model_name: &str) -> bool {
    caches::is_gltf_path(model_name) || model_name.to_lowercase().ends_with(".fbx")
}
