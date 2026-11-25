use std::path::Path;

use anyhow::{anyhow, Result};

use super::{caches, cleanup, clips, entities, gpu, nodes};
use crate::asset::AssetStorage;
use crate::ecs::systems::rebuild_scene_acceleration;
use crate::ecs::world::{Entity, World};
use crate::ecs::GpuSceneContext;
use crate::hooks::model_load::{run_model_load_hooks, LoadedModel};
use thyllore_anim_core::editable::SourceClipId;
use thyllore_file_format_core::fbx::FbxModel;
use thyllore_importer_core::ModelLoadResult;
use thyllore_vulkan_core::core::RRSwapchain;
use thyllore_vulkan_core::resource::GraphicsResources;

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
        let (result, fbx_model) =
            thyllore_importer_core::fbx::load_fbx_to_graphics_resources(path)?;
        Ok((ModelLoadResult::from_fbx(result), Some(fbx_model)))
    } else if caches::is_gltf_path(path) {
        let result = thyllore_importer_core::gltf::load_gltf_file(path)?;
        Ok((ModelLoadResult::from_gltf(result), None))
    } else {
        Err(anyhow!(
            "Unsupported file format. Only FBX and glTF/GLB are supported."
        ))
    }
}

/// Leaves the world without entities and the acceleration structures released, for the caller to restore.
pub(super) unsafe fn upload_scene_model(
    load_result: &ModelLoadResult,
    model_name: &str,
    gpu: &mut GpuSceneContext,
    swapchain: &RRSwapchain,
    world: &mut World,
    assets: &mut AssetStorage,
    fbx_model: Option<FbxModel>,
) -> Result<Option<SourceClipId>> {
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

    Ok(clips::register_loaded_clips(
        world,
        assets,
        Path::new(model_name),
        &load_result.clips,
    ))
}

pub(super) fn attach_scene_model(
    model_entity: Entity,
    first_clip_id: Option<SourceClipId>,
    load_result: &ModelLoadResult,
    graphics: &GraphicsResources,
    world: &mut World,
    assets: &mut AssetStorage,
) {
    entities::attach_model_entities(
        model_entity,
        first_clip_id,
        graphics,
        world,
        assets,
        load_result,
    );
    run_model_load_hooks(
        world,
        assets,
        &LoadedModel {
            entity: model_entity,
            load_result,
        },
    );
}
