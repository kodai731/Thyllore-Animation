use anyhow::{anyhow, ensure, Result};

use super::entities::{
    despawn_model_source_entities, despawn_other_model_source_entities, find_model_source_entity,
};
use super::replace::{attach_scene_model, read_model_file, upload_scene_model};
use super::source::resolve_model_file;
use crate::asset::AssetStorage;
use crate::ecs::resource::{ClipLibrary, TimelineState};
use crate::ecs::systems::clip_library_register_loaded;
use crate::ecs::systems::rebuild_scene_acceleration;
use crate::ecs::world::{Entity, World};
use crate::ecs::GpuSceneContext;
use crate::scene::{
    apply_scene_entities, capture_scene_entities, capture_scheduled_clips, SceneEntity,
};
use thyllore_anim_core::editable::{EditableAnimationClip, SourceClipId};
use thyllore_file_format_core::fbx::FbxModel;
use thyllore_importer_core::ModelLoadResult;
use thyllore_vulkan_core::core::RRSwapchain;

/// Replaces the model while keeping the scene's own entities and scheduled clips.
pub unsafe fn load_model_keeping_scene(
    model_entity: Entity,
    gpu: &mut GpuSceneContext,
    swapchain: &RRSwapchain,
    world: &mut World,
    assets: &mut AssetStorage,
) -> Result<()> {
    let Some(model_file) = resolve_model_file(world, model_entity) else {
        return Ok(());
    };
    let model_path = model_file.to_string_lossy().into_owned();
    let path = model_path.as_str();

    log!("Loading new model from: {}", path);
    let (load_result, fbx_model) = match read_model_with_meshes(path) {
        Ok(model_file) => model_file,
        Err(error) => {
            world.despawn(model_entity);
            return Err(error);
        }
    };

    despawn_other_model_source_entities(world, model_entity);
    let kept_scene = capture_kept_scene(world);
    let uploaded = upload_scene_model(&load_result, path, gpu, swapchain, world, assets, fbx_model);
    restore_kept_scene(kept_scene, world, assets);

    let attached = uploaded.and_then(|first_clip_id| {
        attach_file_model(first_clip_id, &load_result, gpu, world, assets)
    });
    if attached.is_err() {
        despawn_model_source_entities(world);
    }
    let rebuilt = rebuild_scene_acceleration(gpu, world, assets);

    let loaded = attached.and(rebuilt);
    if loaded.is_ok() {
        msg_info!("Model loaded: {}", path);
    }
    loaded
}

/// A model entity without mesh children counts as not loaded, so a file without meshes is refused.
unsafe fn read_model_with_meshes(path: &str) -> Result<(ModelLoadResult, Option<FbxModel>)> {
    let (load_result, fbx_model) = read_model_file(path)?;
    ensure!(
        !load_result.meshes.is_empty(),
        "The model file has no meshes: {}",
        path
    );

    Ok((load_result, fbx_model))
}

fn attach_file_model(
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
    reset_timeline_for_loaded_model(world);

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
    use super::entities::{spawn_model_root_entity, GENERATED_MODEL_NAME};
    use crate::ecs::component::GeneratedGlb;
    use crate::ecs::resource::GltfModelCache;

    log!("Loading generated mesh from GLB ({} bytes)", glb_data.len());
    let load_result = match thyllore_importer_core::gltf::load_gltf_from_slice(glb_data) {
        Ok(gltf) => ModelLoadResult::from_gltf(gltf),
        Err(error) => return Err(error),
    };

    despawn_model_source_entities(world);
    let kept_scene = capture_kept_scene(world);
    let uploaded = upload_scene_model(
        &load_result,
        GENERATED_MODEL_NAME,
        gpu,
        swapchain,
        world,
        assets,
        None,
    );
    restore_kept_scene(kept_scene, world, assets);

    let attached = uploaded.map(|first_clip_id| {
        let model_entity = spawn_model_root_entity(GENERATED_MODEL_NAME, world);
        attach_scene_model(
            model_entity,
            first_clip_id,
            &load_result,
            gpu.graphics,
            world,
            assets,
        );
        reset_timeline_for_loaded_model(world);
        world.insert_resource(GltfModelCache::from_glb_data(glb_data.to_vec()));
        world.insert_component(
            model_entity,
            GeneratedGlb {
                bytes: glb_data.to_vec(),
            },
        );
    });
    let rebuilt = rebuild_scene_acceleration(gpu, world, assets);

    attached.and(rebuilt)
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
    let removed_clips = select_clips_removed_from_library(world, kept_scene.scheduled_clips);
    clip_library_register_loaded(world, assets, removed_clips);
    apply_scene_entities(world, assets, &kept_scene.entities);
}

/// The model reset drops only the clips the old model brought; a clip that is still in the
/// library must not be registered a second time.
fn select_clips_removed_from_library(
    world: &World,
    clips: Vec<EditableAnimationClip>,
) -> Vec<EditableAnimationClip> {
    let library = world.resource::<ClipLibrary>();
    clips
        .into_iter()
        .filter(|clip| library.get(clip.id).is_none())
        .collect()
}

fn reset_timeline_for_loaded_model(world: &mut World) {
    world.resource_mut::<TimelineState>().current_time = 0.0;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::systems::clip_library_register_and_activate;

    #[test]
    fn restore_registers_only_clips_the_reset_removed() {
        let mut world = World::new();
        let mut assets = AssetStorage::default();
        let mut library = ClipLibrary::new();
        let user_clip_id = clip_library_register_and_activate(
            &mut library,
            &mut assets,
            EditableAnimationClip::new(0, "user".to_string()),
        );
        let model_clip_id = clip_library_register_and_activate(
            &mut library,
            &mut assets,
            EditableAnimationClip::new(0, "model".to_string()),
        );
        library.model_clip_ids.insert(model_clip_id);
        let scheduled_clips = vec![
            library.get(user_clip_id).cloned().expect("user clip"),
            library.get(model_clip_id).cloned().expect("model clip"),
        ];
        library.clear_model_clips();
        world.insert_resource(library);

        let removed_clips = select_clips_removed_from_library(&world, scheduled_clips);

        let removed_names: Vec<&str> = removed_clips.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(removed_names, vec!["model"]);
    }
}
