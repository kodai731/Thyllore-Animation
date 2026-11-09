use anyhow::{anyhow, Result};

use super::{caches, cleanup, clips, entities, gpu, initial_pose, nodes};
use crate::asset::AssetStorage;
use crate::ecs::component::GlbSource;
use crate::ecs::systems::rebuild_scene_acceleration;
use crate::ecs::world::{Entity, World};
use crate::ecs::GpuSceneContext;
use crate::hooks::model_load::{run_model_load_hooks, LoadedModel};
use crate::loader::fbx::FbxModel;
use crate::loader::ModelLoadResult;
use crate::vulkanr::swapchain::RRSwapchain;

/// Who registers the clips a model brings: the load itself, or the scene restore that follows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipRegistration {
    FromModel,
    DeferredToScene,
}

pub unsafe fn load_model_from_file_system(
    path: &str,
    gpu: &mut GpuSceneContext,
    swapchain: &RRSwapchain,
    world: &mut World,
    assets: &mut AssetStorage,
    clip_registration: ClipRegistration,
) -> Result<()> {
    log!("=== Loading model from path: {} ===", path);

    let (load_result, fbx_model) = read_model_file(path)?;
    replace_scene_model(
        &load_result,
        path,
        gpu,
        swapchain,
        world,
        assets,
        clip_registration,
        fbx_model,
    )?;

    log!("=== Model loaded successfully ===");
    Ok(())
}

pub(crate) unsafe fn append_model_to_scene(
    load_result: &ModelLoadResult,
    part_name: &str,
    gpu: &mut GpuSceneContext,
    swapchain: &RRSwapchain,
    world: &mut World,
    assets: &mut AssetStorage,
) -> Result<Entity> {
    let mesh_index_offset = gpu.graphics.meshes.len();
    gpu::upload_model_meshes(
        load_result,
        part_name,
        gpu.instance,
        gpu.device,
        gpu.command_pool,
        swapchain,
        gpu.graphics,
    )?;
    rebuild_scene_acceleration(gpu, world, assets)?;

    let parent_entity = entities::spawn_model_part_entity(part_name, world);
    entities::spawn_mesh_entities(
        &entities::mesh_entity_names(part_name, &load_result.meshes, mesh_index_offset),
        gpu.graphics,
        world,
        assets,
        parent_entity,
        mesh_index_offset..gpu.graphics.meshes.len(),
    );

    log!(
        "Additively loaded '{}': {} meshes, total entities={}",
        part_name,
        load_result.meshes.len(),
        world.entity_count()
    );

    Ok(parent_entity)
}

pub(super) unsafe fn read_model_file(path: &str) -> Result<(ModelLoadResult, Option<FbxModel>)> {
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

pub(super) unsafe fn replace_scene_model(
    load_result: &ModelLoadResult,
    model_name: &str,
    gpu: &mut GpuSceneContext,
    swapchain: &RRSwapchain,
    world: &mut World,
    assets: &mut AssetStorage,
    clip_registration: ClipRegistration,
    fbx_model: Option<FbxModel>,
) -> Result<Entity> {
    cleanup::reset_scene_model(gpu, world, assets)?;
    caches::insert_model_caches(world, model_name, fbx_model, gpu.graphics.meshes.len());
    clips::register_imported_animation(world, load_result, assets);
    nodes::replace_node_assets(world, assets, load_result);

    gpu::upload_model_meshes(
        load_result,
        model_name,
        gpu.instance,
        gpu.device,
        gpu.command_pool,
        swapchain,
        gpu.graphics,
    )?;
    let posed_meshes = initial_pose::apply_initial_pose(world, assets, gpu.graphics, load_result);
    gpu::upload_posed_meshes(
        gpu.instance,
        gpu.device,
        gpu.command_pool,
        gpu.graphics,
        &posed_meshes,
    );
    rebuild_scene_acceleration(gpu, world, assets)?;

    let parent_entity = entities::spawn_model_entities(
        model_name,
        gpu.graphics,
        world,
        assets,
        load_result,
        clip_registration,
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
