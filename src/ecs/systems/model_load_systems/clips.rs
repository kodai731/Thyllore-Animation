use std::path::Path;

use super::entities::ModelReplacement;
use crate::asset::{AssetStorage, SkeletonAsset};
use crate::ecs::resource::{BatchRun, ClipLibrary, ModelState, TimelineState};
use crate::ecs::systems::{
    build_humanoid_rig, convert_clip_to_standard_space, find_first_skeleton,
    timeline_apply_fit_zoom, vrm_humanoid_to_mapping,
};
use crate::ecs::systems::{
    clip_library_register_and_activate, clip_library_register_empty_model_clip, find_best_clip,
    select_first_animated_clip, EMPTY_MODEL_CLIP_DURATION_SECONDS,
};
use crate::ecs::world::World;
use thyllore_anim_core::editable::{select_kept_clip_id, EditableAnimationClip, SourceClipId};
use thyllore_anim_core::AnimationClip;
use thyllore_importer_core::ModelLoadResult;

pub(super) fn register_imported_animation(
    world: &mut World,
    load_result: &ModelLoadResult,
    assets: &mut AssetStorage,
) {
    {
        let mut clip_library = world.resource_mut::<ClipLibrary>();
        clip_library.animation = load_result.animation_system.clone();
    }

    for skeleton in &load_result.skeletons {
        assets.add_skeleton(SkeletonAsset {
            id: 0,
            skeleton_id: skeleton.id,
            skeleton: skeleton.clone(),
        });
    }

    let mut model_state = world.resource_mut::<ModelState>();
    model_state.has_skinned_meshes = load_result.has_skinned_meshes;
    model_state.imported_humanoid = load_result
        .vrm_humanoid
        .as_ref()
        .and_then(vrm_humanoid_to_mapping);
}

/// A model that takes another model's place and brings animation starts on its own clip; the clip
/// selected before stays in the library.
pub(super) fn release_selection_to_replacing_model(
    world: &mut World,
    replacement: ModelReplacement,
    loaded_clips: &[AnimationClip],
) {
    if replacement == ModelReplacement::NothingReplaced || loaded_clips.is_empty() {
        return;
    }
    world.resource_mut::<TimelineState>().current_clip_id = None;
}

/// Returns the clip the timeline should start on, or `None` for a model that cannot animate.
pub(super) fn register_loaded_clips(
    world: &mut World,
    assets: &mut AssetStorage,
    model_file: &Path,
    loaded_clips: &[AnimationClip],
) -> Option<SourceClipId> {
    if let Some(first_clip_id) = register_clips_to_library(world, assets, model_file, loaded_clips)
    {
        return Some(first_clip_id);
    }
    if assets.skeletons.is_empty() {
        return None;
    }
    if let Some(kept_clip_id) = select_kept_clip(world) {
        return Some(kept_clip_id);
    }
    Some(register_empty_editable_clip(world, assets))
}

fn select_kept_clip(world: &mut World) -> Option<SourceClipId> {
    let kept_clip_id = {
        let library = world.resource::<ClipLibrary>();
        let selected = world.resource::<TimelineState>().current_clip_id;
        select_kept_clip_id(selected, library.all_clip_ids().copied())?
    };

    let clip_duration = world
        .resource::<ClipLibrary>()
        .get(kept_clip_id)
        .map(|c| c.duration)
        .unwrap_or(0.0);
    let mut timeline_state = world.resource_mut::<TimelineState>();
    timeline_state.current_clip_id = Some(kept_clip_id);
    timeline_apply_fit_zoom(&mut timeline_state, clip_duration);
    log!(
        "Model has no animations; timeline keeps clip {} from the library",
        kept_clip_id
    );

    Some(kept_clip_id)
}

/// The clips of a humanoid model are stored in the standard space, so they play on any humanoid model.
fn import_editable_clips(
    world: &World,
    assets: &AssetStorage,
    model_file: &Path,
    loaded_clips: &[AnimationClip],
) -> Vec<EditableAnimationClip> {
    let bone_names: std::collections::HashMap<u32, String> = assets
        .skeletons
        .values()
        .flat_map(|sa| sa.skeleton.bones.iter().map(|b| (b.id, b.name.clone())))
        .collect();
    let editable_clips = loaded_clips
        .iter()
        .map(|clip| thyllore_anim_core::editable::clip_from_animation(0, clip, &bone_names));

    let Some(skeleton) = find_first_skeleton(assets) else {
        return editable_clips.collect();
    };
    let imported = world
        .get_resource::<ModelState>()
        .and_then(|model_state| model_state.imported_humanoid.clone());
    let Some(rig) = build_humanoid_rig(model_file, skeleton, imported.as_ref()) else {
        return editable_clips.collect();
    };

    let fps = world
        .resource::<TimelineState>()
        .snap_settings
        .frame_rate
        .round() as u32;
    editable_clips
        .map(|editable| convert_clip_to_standard_space(&editable, skeleton, &rig, fps))
        .collect()
}

fn register_clips_to_library(
    world: &mut World,
    assets: &mut AssetStorage,
    model_file: &Path,
    loaded_clips: &[AnimationClip],
) -> Option<SourceClipId> {
    let editable_clips = import_editable_clips(world, assets, model_file, loaded_clips);

    let mut model_clips: Vec<(SourceClipId, String)> = Vec::new();
    {
        let mut clip_library = world.resource_mut::<ClipLibrary>();
        for editable in editable_clips {
            let clip_name = editable.name.clone();
            let editable_id =
                clip_library_register_and_activate(&mut clip_library, assets, editable);
            log!(
                "Registered clip '{}' (source_id={})",
                clip_name,
                editable_id,
            );
            model_clips.push((editable_id, clip_name));
        }
        clip_library
            .model_clip_ids
            .extend(model_clips.iter().map(|(id, _)| *id));
    }

    let requested_clip_name = world
        .resource_mut::<TimelineState>()
        .requested_clip_name
        .take();
    let (editable_id, clip_duration) = {
        let clip_library = world.resource::<ClipLibrary>();
        let editable_id =
            select_start_clip(&clip_library, &model_clips, requested_clip_name.as_deref())?;
        let clip_duration = clip_library
            .get(editable_id)
            .map(|c| c.duration)
            .unwrap_or(0.0);
        (editable_id, clip_duration)
    };
    let keeps_user = keeps_selected_user_clip(world);
    let mut timeline_state = world.resource_mut::<TimelineState>();
    if !keeps_user {
        timeline_state.current_clip_id = Some(editable_id);
        timeline_apply_fit_zoom(&mut timeline_state, clip_duration);
    }
    log!("Set timeline current_clip_id to {}", editable_id);

    Some(editable_id)
}

/// The clip the scene named when the model brought it, otherwise the model's first animated clip.
fn select_start_clip(
    clip_library: &ClipLibrary,
    model_clips: &[(SourceClipId, String)],
    requested_clip_name: Option<&str>,
) -> Option<SourceClipId> {
    let requested_clip = model_clips
        .iter()
        .find(|(_, clip_name)| requested_clip_name == Some(clip_name.as_str()))
        .map(|(id, _)| *id);
    if requested_clip.is_some() {
        return requested_clip;
    }

    let model_clip_ids: Vec<SourceClipId> = model_clips.iter().map(|(id, _)| *id).collect();
    select_first_animated_clip(clip_library, &model_clip_ids)
}

fn register_empty_editable_clip(world: &mut World, assets: &mut AssetStorage) -> SourceClipId {
    let source_id =
        clip_library_register_empty_model_clip(&mut world.resource_mut::<ClipLibrary>(), assets);

    if !keeps_selected_user_clip(world) {
        world.resource_mut::<TimelineState>().current_clip_id = Some(source_id);
    }

    log!(
        "Auto-created empty animation clip 'New Animation' (source_id={}, duration={}s) for model with no animations",
        source_id,
        EMPTY_MODEL_CLIP_DURATION_SECONDS
    );

    source_id
}

/// A batch run plays the loaded model from its start, whatever time the scene was saved at.
pub(super) fn restart_batch_playback(world: &World) {
    let requested_clip_id = match world
        .get_resource::<BatchRun>()
        .and_then(|batch_run| batch_run.playback.clone())
    {
        Some(playback) => playback.clip_id,
        None => return,
    };

    let clip_still_loaded = requested_clip_id.is_some_and(|id| {
        world
            .get_resource::<ClipLibrary>()
            .is_some_and(|library| library.get(id).is_some())
    });
    let clip_id = if clip_still_loaded {
        requested_clip_id
    } else {
        find_best_clip(world)
    };

    let Some(mut timeline) = world.get_resource_mut::<TimelineState>() else {
        return;
    };
    timeline.playing = true;
    timeline.current_time = 0.0;
    if let Some(id) = clip_id {
        timeline.current_clip_id = Some(id);
    }
}

pub(super) fn keeps_selected_user_clip(world: &World) -> bool {
    let (Some(timeline), Some(library)) = (
        world.get_resource::<TimelineState>(),
        world.get_resource::<ClipLibrary>(),
    ) else {
        return false;
    };
    library.is_user_clip_selected(timeline.current_clip_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Quat;
    use thyllore_anim_core::{Keyframe, Skeleton, TransformChannel};

    const MODEL_FILE: &str = "model.fbx";

    #[test]
    fn model_load_keeps_a_selected_user_clip() {
        let mut world = World::new();
        let mut assets = AssetStorage::default();
        world.insert_resource(ClipLibrary::default());
        world.insert_resource(TimelineState::default());

        let user_clip = EditableAnimationClip::new(0, "user".to_string());
        let user_source_id = {
            let mut clip_library = world.resource_mut::<ClipLibrary>();
            clip_library_register_and_activate(&mut clip_library, &mut assets, user_clip)
        };

        world.resource_mut::<TimelineState>().current_clip_id = Some(user_source_id);

        register_empty_editable_clip(&mut world, &mut assets);

        assert_eq!(
            world.resource::<TimelineState>().current_clip_id,
            Some(user_source_id)
        );
    }

    #[test]
    fn model_load_with_clips_keeps_a_selected_user_clip() {
        let mut world = World::new();
        let mut assets = AssetStorage::default();
        world.insert_resource(ClipLibrary::default());
        world.insert_resource(TimelineState::default());

        assets.add_skeleton(SkeletonAsset {
            id: 0,
            skeleton_id: 0,
            skeleton: Skeleton::default(),
        });

        let user_clip = EditableAnimationClip::new(0, "user".to_string());
        let user_source_id = {
            let mut clip_library = world.resource_mut::<ClipLibrary>();
            clip_library_register_and_activate(&mut clip_library, &mut assets, user_clip)
        };

        world.resource_mut::<TimelineState>().current_clip_id = Some(user_source_id);

        let loaded_clips: Vec<AnimationClip> = vec![AnimationClip::new("fbx")];
        register_clips_to_library(
            &mut world,
            &mut assets,
            Path::new(MODEL_FILE),
            &loaded_clips,
        );

        assert_eq!(
            world.resource::<TimelineState>().current_clip_id,
            Some(user_source_id)
        );
    }

    fn world_with_selected_user_clip() -> (World, AssetStorage, SourceClipId) {
        let mut world = World::new();
        let mut assets = AssetStorage::default();
        world.insert_resource(ClipLibrary::default());
        world.insert_resource(TimelineState::default());
        assets.add_skeleton(SkeletonAsset {
            id: 0,
            skeleton_id: 0,
            skeleton: Skeleton::default(),
        });

        let user_clip_id = clip_library_register_and_activate(
            &mut world.resource_mut::<ClipLibrary>(),
            &mut assets,
            EditableAnimationClip::new(0, "user".to_string()),
        );
        world.resource_mut::<TimelineState>().current_clip_id = Some(user_clip_id);

        (world, assets, user_clip_id)
    }

    #[test]
    fn replacing_model_with_clips_starts_on_its_own_clip() {
        let (mut world, mut assets, user_clip_id) = world_with_selected_user_clip();
        let loaded_clips: Vec<AnimationClip> = vec![AnimationClip::new("fbx")];

        release_selection_to_replacing_model(&mut world, ModelReplacement::Replaced, &loaded_clips);
        let start_clip_id = register_loaded_clips(
            &mut world,
            &mut assets,
            Path::new(MODEL_FILE),
            &loaded_clips,
        );

        assert_ne!(start_clip_id, Some(user_clip_id));
        assert_eq!(
            world.resource::<TimelineState>().current_clip_id,
            start_clip_id
        );
        assert!(world.resource::<ClipLibrary>().get(user_clip_id).is_some());
    }

    #[test]
    fn replacing_model_without_clips_keeps_the_selected_user_clip() {
        let (mut world, mut assets, user_clip_id) = world_with_selected_user_clip();

        release_selection_to_replacing_model(&mut world, ModelReplacement::Replaced, &[]);
        let start_clip_id =
            register_loaded_clips(&mut world, &mut assets, Path::new(MODEL_FILE), &[]);

        assert_eq!(start_clip_id, Some(user_clip_id));
        assert_eq!(
            world.resource::<TimelineState>().current_clip_id,
            Some(user_clip_id)
        );
    }

    #[test]
    fn model_that_replaces_no_model_keeps_the_selected_user_clip() {
        let (mut world, mut assets, user_clip_id) = world_with_selected_user_clip();
        let loaded_clips: Vec<AnimationClip> = vec![AnimationClip::new("fbx")];

        release_selection_to_replacing_model(
            &mut world,
            ModelReplacement::NothingReplaced,
            &loaded_clips,
        );
        register_loaded_clips(
            &mut world,
            &mut assets,
            Path::new(MODEL_FILE),
            &loaded_clips,
        );

        assert_eq!(
            world.resource::<TimelineState>().current_clip_id,
            Some(user_clip_id)
        );
    }

    #[test]
    fn humanoid_model_clips_are_stored_by_role_before_the_model_entity_exists() {
        use crate::ecs::systems::humanoid_rig_systems::{
            copy_test_humanoid_fixture, test_humanoid_world,
        };

        let fixture_dir = tempfile::tempdir().expect("temp dir");
        let (model_file, _) = copy_test_humanoid_fixture(fixture_dir.path());
        let (_, mut assets) = test_humanoid_world(&model_file);
        let mut world = World::new();
        world.insert_resource(ClipLibrary::new());
        world.insert_resource(TimelineState::default());
        let mut animated_take = AnimationClip::new("take");
        animated_take.duration = 0.1;
        animated_take.add_channel(
            0,
            TransformChannel {
                rotation: vec![Keyframe::new(0.0, Quat::IDENTITY)],
                ..Default::default()
            },
        );
        let loaded_clips = vec![AnimationClip::new("empty take"), animated_take];

        let clip_id = register_loaded_clips(&mut world, &mut assets, &model_file, &loaded_clips)
            .expect("start clip");

        let library = world.resource::<ClipLibrary>();
        let start_clip = library.get(clip_id).expect("registered clip");
        assert_eq!(start_clip.name, "take");
        assert!(start_clip
            .tracks
            .values()
            .any(|track| track.bone_name == "Hips"));
    }

    fn world_requesting_clip(requested_clip_name: Option<&str>) -> (World, AssetStorage) {
        let mut world = World::new();
        world.insert_resource(ClipLibrary::new());
        world.insert_resource(TimelineState::default());
        world.resource_mut::<TimelineState>().requested_clip_name =
            requested_clip_name.map(str::to_string);
        (world, AssetStorage::default())
    }

    fn current_clip_name(world: &World) -> Option<String> {
        let clip_id = world.resource::<TimelineState>().current_clip_id?;
        let library = world.resource::<ClipLibrary>();
        library.get(clip_id).map(|clip| clip.name.clone())
    }

    #[test]
    fn model_load_selects_the_model_clip_the_scene_named() {
        let (mut world, mut assets) = world_requesting_clip(Some("run"));
        let loaded_clips = vec![AnimationClip::new("walk"), AnimationClip::new("run")];

        let start_clip_id = register_clips_to_library(
            &mut world,
            &mut assets,
            Path::new(MODEL_FILE),
            &loaded_clips,
        );

        assert_eq!(current_clip_name(&world).as_deref(), Some("run"));
        let timeline = world.resource::<TimelineState>();
        assert_eq!(start_clip_id, timeline.current_clip_id);
        assert_eq!(timeline.requested_clip_name, None);
    }

    #[test]
    fn model_load_starts_on_the_first_clip_when_the_named_one_is_absent() {
        let (mut world, mut assets) = world_requesting_clip(Some("jump"));
        let loaded_clips = vec![AnimationClip::new("walk"), AnimationClip::new("run")];

        register_clips_to_library(
            &mut world,
            &mut assets,
            Path::new(MODEL_FILE),
            &loaded_clips,
        );

        assert_eq!(current_clip_name(&world).as_deref(), Some("walk"));
        assert_eq!(world.resource::<TimelineState>().requested_clip_name, None);
    }

    #[test]
    fn model_load_skips_an_empty_take_for_the_first_animated_clip() {
        use thyllore_anim_core::{Keyframe, TransformChannel};

        let (mut world, mut assets) = world_requesting_clip(None);
        let mut animated_clip = AnimationClip::new("jog");
        animated_clip.add_channel(
            0,
            TransformChannel {
                translation: vec![Keyframe::new(0.0, glam::Vec3::X)],
                rotation: Vec::new(),
                scale: Vec::new(),
            },
        );
        let loaded_clips = vec![AnimationClip::new("empty take"), animated_clip];

        register_clips_to_library(
            &mut world,
            &mut assets,
            Path::new(MODEL_FILE),
            &loaded_clips,
        );

        assert_eq!(current_clip_name(&world).as_deref(), Some("jog"));
    }

    #[test]
    fn model_load_skips_an_empty_take_for_a_morph_only_clip() {
        use thyllore_anim_core::MorphWeightChannel;

        let (mut world, mut assets) = world_requesting_clip(None);
        let mut expression_clip = AnimationClip::new("smile");
        expression_clip.add_morph_channel(MorphWeightChannel {
            source_mesh: "Face".to_string(),
            channel: "Smile".to_string(),
            keyframes: vec![Keyframe::new(0.0, 1.0)],
        });
        let loaded_clips = vec![AnimationClip::new("empty take"), expression_clip];

        register_clips_to_library(
            &mut world,
            &mut assets,
            Path::new(MODEL_FILE),
            &loaded_clips,
        );

        assert_eq!(current_clip_name(&world).as_deref(), Some("smile"));
    }

    #[test]
    fn model_without_animations_keeps_library_clip_instead_of_creating_an_empty_one() {
        let mut world = World::new();
        let mut assets = AssetStorage::default();
        world.insert_resource(ClipLibrary::default());
        world.insert_resource(TimelineState::default());

        assets.add_skeleton(SkeletonAsset {
            id: 0,
            skeleton_id: 0,
            skeleton: Skeleton::default(),
        });

        let user_clip = EditableAnimationClip::new(0, "user".to_string());
        let user_source_id = {
            let mut clip_library = world.resource_mut::<ClipLibrary>();
            clip_library_register_and_activate(&mut clip_library, &mut assets, user_clip)
        };

        let first_clip_id =
            register_loaded_clips(&mut world, &mut assets, Path::new(MODEL_FILE), &[]);

        assert_eq!(first_clip_id, Some(user_source_id));
        assert_eq!(world.resource::<ClipLibrary>().clip_count(), 1);
        assert_eq!(
            world.resource::<TimelineState>().current_clip_id,
            Some(user_source_id)
        );
    }

    #[test]
    fn auto_created_clip_keeps_its_length_after_the_first_key() {
        use thyllore_anim_core::editable::{clip_add_keyframe, clip_recalculate_duration};
        use thyllore_anim_core::editable::{BoneTrack, PropertyType};

        let mut world = World::new();
        let mut assets = AssetStorage::default();
        world.insert_resource(ClipLibrary::default());
        world.insert_resource(TimelineState::default());

        let source_id = register_empty_editable_clip(&mut world, &mut assets);
        let mut library = world.resource_mut::<ClipLibrary>();
        let clip = library.get_mut(source_id).expect("clip registered");
        clip.tracks
            .insert(1, BoneTrack::new(1, "Hips".to_string(), 0));
        clip_add_keyframe(clip, 1, PropertyType::RotationX, 0.5, 10.0);
        clip_recalculate_duration(clip);

        assert!((clip.duration - EMPTY_MODEL_CLIP_DURATION_SECONDS).abs() < f32::EPSILON);
    }

    #[test]
    fn model_without_animations_and_empty_library_creates_an_empty_clip() {
        let mut world = World::new();
        let mut assets = AssetStorage::default();
        world.insert_resource(ClipLibrary::default());
        world.insert_resource(TimelineState::default());

        assets.add_skeleton(SkeletonAsset {
            id: 0,
            skeleton_id: 0,
            skeleton: Skeleton::default(),
        });

        let first_clip_id =
            register_loaded_clips(&mut world, &mut assets, Path::new(MODEL_FILE), &[]);

        assert!(first_clip_id.is_some());
        assert_eq!(world.resource::<ClipLibrary>().clip_count(), 1);
    }

    #[test]
    fn model_load_replaces_a_selected_model_clip() {
        let mut world = World::new();
        let mut assets = AssetStorage::default();
        world.insert_resource(ClipLibrary::default());
        world.insert_resource(TimelineState::default());

        let model_clip = EditableAnimationClip::new(0, "model".to_string());
        let model_source_id = {
            let mut clip_library = world.resource_mut::<ClipLibrary>();
            let id = clip_library_register_and_activate(&mut clip_library, &mut assets, model_clip);
            clip_library.model_clip_ids.insert(id);
            id
        };

        world.resource_mut::<TimelineState>().current_clip_id = Some(model_source_id);

        register_empty_editable_clip(&mut world, &mut assets);

        assert_ne!(
            world.resource::<TimelineState>().current_clip_id,
            Some(model_source_id)
        );
    }
}
