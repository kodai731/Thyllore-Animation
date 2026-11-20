use anyhow::{anyhow, Result};

use super::entities::{
    despawn_model_source_entities, find_model_source_entity, replace_model_source_entity,
};
use super::replace::{attach_scene_model, read_model_file, upload_scene_model};
use crate::asset::AssetStorage;
use crate::ecs::component::GlbSource;
use crate::ecs::resource::SceneState;
use crate::ecs::resource::{ModelState, TimelineState};
use crate::ecs::systems::clip_library_register_loaded;
use crate::ecs::systems::rebuild_scene_acceleration;
use crate::ecs::world::World;
use crate::ecs::GpuSceneContext;
use crate::scene::{
    apply_scene_entities, capture_scene_entities, capture_scheduled_clips, SceneEntity,
};
use thyllore_anim_core::editable::{EditableAnimationClip, SourceClipId};
use thyllore_importer_core::ModelLoadResult;
use thyllore_vulkan_core::core::RRSwapchain;

/// Replaces the model while keeping the scene's own entities and scheduled clips.
pub unsafe fn load_model_keeping_scene(
    path: &str,
    gpu: &mut GpuSceneContext,
    swapchain: &RRSwapchain,
    world: &mut World,
    assets: &mut AssetStorage,
) -> Result<()> {
    log!("Loading new model from: {}", path);
    let (load_result, fbx_model) = match read_model_file(path) {
        Ok(model_file) => model_file,
        Err(error) => return report_load_result(world, Err(error)),
    };

    replace_model_source_entity(world, path);
    let kept_scene = capture_kept_scene(world);
    let uploaded = upload_scene_model(&load_result, path, gpu, swapchain, world, assets, fbx_model);
    restore_kept_scene(kept_scene, world, assets);

    let attached = uploaded.and_then(|first_clip_id| {
        attach_file_model(path, first_clip_id, &load_result, gpu, world, assets)
    });
    if attached.is_err() {
        despawn_model_source_entities(world);
    }
    let rebuilt = rebuild_scene_acceleration(gpu, world, assets);

    report_load_result(world, attached.and(rebuilt))
}

fn attach_file_model(
    path: &str,
    first_clip_id: Option<SourceClipId>,
    load_result: &ModelLoadResult,
    gpu: &GpuSceneContext,
    world: &mut World,
    assets: &mut AssetStorage,
) -> Result<()> {
    let model_entity = find_model_source_entity(world)
        .ok_or_else(|| anyhow!("The model entity was not restored with the scene"))?;

    attach_scene_model(
        model_entity,
        first_clip_id,
        load_result,
        gpu.graphics,
        world,
        assets,
    );
    world.insert_component(model_entity, GlbSource::FilePath(path.to_string()));
    record_loaded_model(world, path, format!("Loaded: {}", path));

    Ok(())
}

/// Replaces the model by an in-memory GLB while keeping the scene's own entities and scheduled clips.
#[cfg(feature = "auto-rig")]
pub unsafe fn load_generated_mesh(
    glb_data: &[u8],
    gpu: &mut GpuSceneContext,
    swapchain: &RRSwapchain,
    world: &mut World,
    assets: &mut AssetStorage,
) -> Result<()> {
    use super::entities::spawn_model_root_entity;
    use crate::ecs::resource::GltfModelCache;
    use crate::scene::ModelReference;

    log!("Loading generated mesh from GLB ({} bytes)", glb_data.len());
    let load_result = match thyllore_importer_core::gltf::load_gltf_from_slice(glb_data) {
        Ok(gltf) => ModelLoadResult::from_gltf(gltf),
        Err(error) => return report_load_result(world, Err(error)),
    };

    despawn_model_source_entities(world);
    let kept_scene = capture_kept_scene(world);
    let uploaded = upload_scene_model(
        &load_result,
        ModelReference::GENERATED_MESH,
        gpu,
        swapchain,
        world,
        assets,
        None,
    );
    restore_kept_scene(kept_scene, world, assets);

    let attached = uploaded.map(|first_clip_id| {
        let model_entity = spawn_model_root_entity(ModelReference::GENERATED_MESH, world);
        attach_scene_model(
            model_entity,
            first_clip_id,
            &load_result,
            gpu.graphics,
            world,
            assets,
        );
        record_loaded_model(
            world,
            ModelReference::GENERATED_MESH,
            "Loaded: Generated Mesh".to_string(),
        );
        world.insert_resource(GltfModelCache::from_glb_data(glb_data.to_vec()));
        world.insert_component(model_entity, GlbSource::InMemory(glb_data.to_vec()));
    });
    let rebuilt = rebuild_scene_acceleration(gpu, world, assets);

    report_load_result(world, attached.and(rebuilt))
}

struct KeptScene {
    entities: Vec<SceneEntity>,
    scheduled_clips: Vec<EditableAnimationClip>,
}

fn capture_kept_scene(world: &World) -> KeptScene {
    KeptScene {
        entities: capture_scene_entities(world),
        scheduled_clips: capture_scheduled_clips(world),
    }
}

/// Runs whether or not the upload succeeded, so a failed load still leaves the scene's entities.
fn restore_kept_scene(kept_scene: KeptScene, world: &mut World, assets: &mut AssetStorage) {
    clip_library_register_loaded(world, assets, kept_scene.scheduled_clips);
    apply_scene_entities(world, assets, &kept_scene.entities);
}

fn record_loaded_model(world: &mut World, model_path: &str, load_status: String) {
    {
        let mut model_state = world.resource_mut::<ModelState>();
        model_state.model_path = model_path.to_string();
        model_state.load_status = load_status;
    }
    world.resource_mut::<TimelineState>().current_time = 0.0;
    world.resource_mut::<SceneState>().clear();
}

fn report_load_result(world: &mut World, loaded: Result<()>) -> Result<()> {
    if let Err(error) = &loaded {
        world.resource_mut::<ModelState>().load_status = format!("Error: {}", error);
    }
    loaded
}
