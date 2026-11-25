use std::collections::HashMap;

use anyhow::{anyhow, ensure, Result};

use super::clips::{release_selection_to_replacing_model, request_played_clip_of_reloading_model};
use super::entities::{despawn_other_model_source_entities, find_model_source_entity};
use super::replace::{attach_scene_model, read_model_file, upload_scene_model};
use super::source::{begin_model_load, fail_model_load, resolve_model_file, ModelLoadOrigin};
use crate::asset::AssetStorage;
use crate::ecs::component::ModelSource;
use crate::ecs::resource::{ClipLibrary, EditHistory, TimelineState};
use crate::ecs::systems::clip_library_register_loaded;
use crate::ecs::systems::rebuild_scene_acceleration;
use crate::ecs::world::{Entity, World};
use crate::ecs::GpuSceneContext;
use crate::scene::{
    add_scene_entities, apply_scene_entities, capture_scene_entities, capture_scheduled_clips,
    find_scene_owner_entities, SceneEntity,
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
    let load_origin = begin_model_load(world, model_entity);
    let (load_result, fbx_model) = match read_model_with_meshes(path) {
        Ok(model_file) => model_file,
        Err(error) => {
            give_up_model_load(world, model_entity, load_origin);
            return Err(error);
        }
    };

    request_played_clip_of_reloading_model(world, model_entity);
    let withheld_clip_request = withhold_requested_clip(world, load_origin);
    let replaced_models = capture_model_sources_beside(world, Some(model_entity));
    let replacement = despawn_other_model_source_entities(world, model_entity);
    release_selection_to_replacing_model(world, replacement, &load_result.clips);
    let kept_scene = capture_kept_scene(world);
    let uploaded = upload_scene_model(&load_result, path, gpu, swapchain, world, assets, fbx_model);
    restore_kept_scene(kept_scene, world, assets);

    let attached = uploaded.and_then(|first_clip_id| {
        attach_file_model(first_clip_id, &load_result, gpu, world, assets)
    });
    if attached.is_err() {
        let failed_model_entity = find_model_source_entity(world);
        bring_back_replaced_models(world, assets, &replaced_models, withheld_clip_request);
        if let Some(failed_model_entity) = failed_model_entity {
            give_up_model_load(world, failed_model_entity, load_origin);
        }
    }
    let rebuilt = rebuild_scene_acceleration(gpu, world, assets);

    let loaded = attached.and(rebuilt);
    if loaded.is_ok() {
        msg_info!("Model loaded: {}", path);
    }
    loaded
}

/// The clip a scene asked of its model is that model's alone: only its own failure discards the request.
fn give_up_model_load(world: &mut World, model_entity: Entity, load_origin: ModelLoadOrigin) {
    fail_model_load(world, model_entity, load_origin);

    match load_origin {
        ModelLoadOrigin::InScene => discard_requested_clip(world),
        ModelLoadOrigin::Added => {}
    }
}

fn discard_requested_clip(world: &mut World) {
    world.resource_mut::<TimelineState>().requested_clip_name = None;
}

/// A model the user added loads without the clip the scene asked of its own model.
fn withhold_requested_clip(world: &mut World, load_origin: ModelLoadOrigin) -> Option<String> {
    match load_origin {
        ModelLoadOrigin::InScene => None,
        ModelLoadOrigin::Added => world
            .resource_mut::<TimelineState>()
            .requested_clip_name
            .take(),
    }
}

/// The models a load replaces: every model of the scene but the one being loaded.
fn capture_model_sources_beside(world: &World, loading_entity: Option<Entity>) -> Vec<SceneEntity> {
    find_scene_owner_entities(world)
        .into_iter()
        .zip(capture_scene_entities(world))
        .filter(|(entity, _)| Some(*entity) != loading_entity)
        .filter(|(entity, _)| world.get_component::<ModelSource>(*entity).is_some())
        .map(|(_, scene_entity)| scene_entity)
        .collect()
}

/// The models a failed load replaced return without meshes, so the next dispatch loads them again.
fn bring_back_replaced_models(
    world: &mut World,
    assets: &mut AssetStorage,
    replaced_models: &[SceneEntity],
    withheld_clip_request: Option<String>,
) {
    add_scene_entities(world, assets, replaced_models);
    world.resource_mut::<TimelineState>().requested_clip_name = withheld_clip_request;
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
    use super::entities::{
        despawn_model_source_entities, spawn_model_root_entity, GENERATED_MODEL_NAME,
    };
    use crate::ecs::component::GeneratedGlb;
    use crate::ecs::resource::GltfModelCache;

    log!("Loading generated mesh from GLB ({} bytes)", glb_data.len());
    let load_result = match thyllore_importer_core::gltf::load_gltf_from_slice(glb_data) {
        Ok(gltf) => ModelLoadResult::from_gltf(gltf),
        Err(error) => return Err(error),
    };

    let withheld_clip_request = withhold_requested_clip(world, ModelLoadOrigin::Added);
    let replaced_models = capture_model_sources_beside(world, None);
    let replacement = despawn_model_source_entities(world);
    release_selection_to_replacing_model(world, replacement, &load_result.clips);
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
        world.insert_resource(GltfModelCache::from_glb_data(glb_data.to_vec()));
        world.insert_component(
            model_entity,
            GeneratedGlb {
                bytes: glb_data.to_vec(),
            },
        );
    });
    if attached.is_err() {
        bring_back_replaced_models(world, assets, &replaced_models, withheld_clip_request);
    }
    let rebuilt = rebuild_scene_acceleration(gpu, world, assets);

    attached.and(rebuilt)
}

struct KeptScene {
    owners: Vec<Entity>,
    entities: Vec<SceneEntity>,
    scheduled_clips: Vec<EditableAnimationClip>,
}

fn capture_kept_scene(world: &World) -> KeptScene {
    KeptScene {
        owners: find_scene_owner_entities(world),
        entities: capture_scene_entities(world),
        scheduled_clips: capture_scheduled_clips(world),
    }
}

/// Runs whether or not the upload succeeded, so a failed load still leaves the scene's entities.
fn restore_kept_scene(kept_scene: KeptScene, world: &mut World, assets: &mut AssetStorage) {
    let removed_clips = select_clips_removed_from_library(world, kept_scene.scheduled_clips);
    clip_library_register_loaded(world, assets, removed_clips);
    let respawned = apply_scene_entities(world, assets, &kept_scene.entities);
    follow_respawned_entities(world, &kept_scene.owners, &respawned);
}

fn follow_respawned_entities(world: &World, owners: &[Entity], respawned: &[Option<Entity>]) {
    let Some(mut edit_history) = world.get_resource_mut::<EditHistory>() else {
        return;
    };
    let respawned_by_owner: HashMap<Entity, Entity> = owners
        .iter()
        .zip(respawned)
        .filter_map(|(owner, respawned)| respawned.map(|respawned| (*owner, respawned)))
        .collect();

    edit_history.follow_respawned_entities(&respawned_by_owner);
}

/// The model reset removes only the clips the old model brought; the others are still registered.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::component::ModelLoadMark;
    use crate::ecs::systems::clip_library_register_and_activate;

    fn world_requesting_clip_of_the_scene_model() -> World {
        let mut world = crate::scene::world_with_scene_hooks();
        world.insert_resource(TimelineState {
            requested_clip_name: Some("run".to_string()),
            ..TimelineState::default()
        });
        world
    }

    fn requested_clip_name(world: &World) -> Option<String> {
        world
            .resource::<TimelineState>()
            .requested_clip_name
            .clone()
    }

    #[test]
    fn failed_added_model_keeps_the_clip_requested_of_the_scene_model() {
        let mut world = world_requesting_clip_of_the_scene_model();
        let added_entity = crate::ecs::systems::model_load_systems::spawn_model_source(
            &mut world,
            "models/avatar/missing.fbx",
        );

        let load_origin = begin_model_load(&mut world, added_entity);
        give_up_model_load(&mut world, added_entity, load_origin);

        assert_eq!(requested_clip_name(&world).as_deref(), Some("run"));
    }

    #[test]
    fn failed_scene_model_discards_the_clip_requested_of_it() {
        let mut world = world_requesting_clip_of_the_scene_model();
        let scene_model = crate::hooks::scene::spawn_scene_owner(
            &mut world,
            "model",
            ModelSource::new("models/avatar/missing.fbx"),
        );

        let load_origin = begin_model_load(&mut world, scene_model);
        give_up_model_load(&mut world, scene_model, load_origin);

        assert_eq!(requested_clip_name(&world), None);
    }

    #[test]
    fn added_model_loads_without_the_clip_requested_of_the_scene_model() {
        let mut world = world_requesting_clip_of_the_scene_model();

        let withheld = withhold_requested_clip(&mut world, ModelLoadOrigin::Added);

        assert_eq!(withheld.as_deref(), Some("run"));
        assert_eq!(requested_clip_name(&world), None);
    }

    #[test]
    fn failed_upload_brings_back_the_model_it_replaced() {
        let mut world = world_requesting_clip_of_the_scene_model();
        let mut assets = AssetStorage::default();
        let scene_model = crate::hooks::scene::spawn_scene_owner(
            &mut world,
            "scene model",
            ModelSource::new("models/avatar/avatar.fbx"),
        );
        let added_entity = crate::ecs::systems::model_load_systems::spawn_model_source(
            &mut world,
            "models/avatar/added.fbx",
        );
        let load_origin = begin_model_load(&mut world, added_entity);
        let withheld = withhold_requested_clip(&mut world, load_origin);
        let replaced_models = capture_model_sources_beside(&world, Some(added_entity));
        let _ = despawn_other_model_source_entities(&mut world, added_entity);
        assert!(world.get_component::<ModelSource>(scene_model).is_none());

        give_up_model_load(&mut world, added_entity, load_origin);
        bring_back_replaced_models(&mut world, &mut assets, &replaced_models, withheld);

        let model_paths: Vec<String> = world
            .iter_components::<ModelSource>()
            .map(|(_, source)| source.path.clone())
            .collect();
        assert_eq!(model_paths, vec!["models/avatar/avatar.fbx".to_string()]);
        assert_eq!(requested_clip_name(&world).as_deref(), Some("run"));
    }

    #[test]
    fn failed_upload_of_a_scene_model_leaves_only_the_model_it_replaced() {
        let mut world = world_requesting_clip_of_the_scene_model();
        let mut assets = AssetStorage::default();
        let failing_entity = crate::hooks::scene::spawn_scene_owner(
            &mut world,
            "failing model",
            ModelSource::new("models/avatar/failing.fbx"),
        );
        crate::hooks::scene::spawn_scene_owner(
            &mut world,
            "replaced model",
            ModelSource::new("models/avatar/avatar.fbx"),
        );
        let load_origin = begin_model_load(&mut world, failing_entity);
        let withheld = withhold_requested_clip(&mut world, load_origin);
        let replaced_models = capture_model_sources_beside(&world, Some(failing_entity));
        let _ = despawn_other_model_source_entities(&mut world, failing_entity);

        bring_back_replaced_models(&mut world, &mut assets, &replaced_models, withheld);
        give_up_model_load(&mut world, failing_entity, load_origin);

        let model_paths: Vec<String> = world
            .iter_components::<ModelSource>()
            .map(|(_, source)| source.path.clone())
            .collect();
        assert_eq!(model_paths, vec!["models/avatar/avatar.fbx".to_string()]);
        assert_eq!(world.iter_components::<ModelLoadMark>().count(), 0);
    }

    #[test]
    fn failed_generated_mesh_brings_back_the_scene_model_and_its_clip_request() {
        let mut world = world_requesting_clip_of_the_scene_model();
        let mut assets = AssetStorage::default();
        crate::hooks::scene::spawn_scene_owner(
            &mut world,
            "scene model",
            ModelSource::new("models/avatar/avatar.fbx"),
        );
        let withheld = withhold_requested_clip(&mut world, ModelLoadOrigin::Added);
        let replaced_models = capture_model_sources_beside(&world, None);
        for entity in crate::hooks::scene::entities_with::<ModelSource>(&world) {
            world.despawn(entity);
        }

        bring_back_replaced_models(&mut world, &mut assets, &replaced_models, withheld);

        let model_paths: Vec<String> = world
            .iter_components::<ModelSource>()
            .map(|(_, source)| source.path.clone())
            .collect();
        assert_eq!(model_paths, vec!["models/avatar/avatar.fbx".to_string()]);
        assert_eq!(requested_clip_name(&world).as_deref(), Some("run"));
    }

    #[test]
    fn history_follows_the_entities_a_rebuild_respawned() {
        let mut world = World::new();
        let mut history = EditHistory::default();
        history.push_schedule_edit(
            7,
            crate::ecs::component::ClipSchedule::default(),
            crate::ecs::component::ClipSchedule::default(),
            "kept",
        );
        history.push_schedule_edit(
            8,
            crate::ecs::component::ClipSchedule::default(),
            crate::ecs::component::ClipSchedule::default(),
            "left",
        );
        world.insert_resource(history);

        follow_respawned_entities(&world, &[7, 8], &[Some(21), None]);

        let mut history = world.resource_mut::<EditHistory>();
        let entry = history
            .pop_undo()
            .expect("the edit of the respawned entity");
        assert!(matches!(
            entry.command,
            crate::ecs::resource::EditCommand::ScheduleModified { entity: 21, .. }
        ));
        assert!(!history.can_undo());
    }

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
