use crate::animation::editable::{EditableAnimationClip, SourceClipId};
use crate::animation::AnimationClip;
use crate::asset::{AssetStorage, SkeletonAsset};
use crate::ecs::component::ClipSchedule;
use crate::ecs::resource::{BatchRun, ClipLibrary, ModelState, TimelineState};
use crate::ecs::systems::clip_library_systems::{
    clip_library_create_from_imported, clip_library_register_and_activate, find_best_clip,
};
use crate::ecs::systems::clip_schedule_systems::clip_schedule_add_instance;
use crate::ecs::systems::timeline_apply_fit_zoom;
use crate::ecs::world::World;
use crate::loader::ModelLoadResult;

const EMPTY_CLIP_DEFAULT_DURATION_SECONDS: f32 = 5.0;

pub(super) fn register_imported_animation(
    world: &mut World,
    load_result: &ModelLoadResult,
    assets: &mut AssetStorage,
) {
    {
        let mut clip_library = world.resource_mut::<ClipLibrary>();
        clip_library.animation = load_result.animation_system.clone();
        clip_library.morph_animation = load_result.morph_animation.clone();
    }

    for skeleton in &load_result.skeletons {
        assets.add_skeleton(SkeletonAsset {
            id: 0,
            skeleton_id: skeleton.id,
            skeleton: skeleton.clone(),
        });
    }

    world.resource_mut::<ModelState>().has_skinned_meshes = load_result.has_skinned_meshes;
}

/// Returns the clip the timeline should start on, or `None` when the scene restores its own clips.
pub(super) fn register_loaded_clips(
    world: &mut World,
    assets: &mut AssetStorage,
    loaded_clips: &[AnimationClip],
    scene_will_provide_clips: bool,
) -> Option<SourceClipId> {
    if scene_will_provide_clips {
        return None;
    }

    if let Some(first_clip_id) = register_clips_to_library(world, assets, loaded_clips) {
        return Some(first_clip_id);
    }
    if assets.skeletons.is_empty() {
        return None;
    }
    Some(register_empty_editable_clip(world, assets))
}

fn register_clips_to_library(
    world: &mut World,
    assets: &mut AssetStorage,
    loaded_clips: &[AnimationClip],
) -> Option<SourceClipId> {
    let bone_names: std::collections::HashMap<u32, String> = assets
        .skeletons
        .values()
        .flat_map(|sa| sa.skeleton.bones.iter().map(|b| (b.id, b.name.clone())))
        .collect();

    let mut first_editable_clip_id = None;
    {
        let mut clip_library = world.resource_mut::<ClipLibrary>();
        for clip in loaded_clips {
            let editable_id =
                clip_library_create_from_imported(&mut clip_library, assets, clip, &bone_names);
            first_editable_clip_id.get_or_insert(editable_id);
            log!(
                "Registered clip '{}' (source_id={})",
                clip.name,
                editable_id,
            );
        }
    }

    let editable_id = first_editable_clip_id?;
    let clip_duration = world
        .resource::<ClipLibrary>()
        .get(editable_id)
        .map(|c| c.duration)
        .unwrap_or(0.0);
    let mut timeline_state = world.resource_mut::<TimelineState>();
    timeline_state.current_clip_id = Some(editable_id);
    timeline_apply_fit_zoom(&mut timeline_state, clip_duration);
    log!("Set timeline current_clip_id to {}", editable_id);

    Some(editable_id)
}

fn register_empty_editable_clip(world: &mut World, assets: &mut AssetStorage) -> SourceClipId {
    let mut editable = EditableAnimationClip::new(0, "New Animation".to_string());
    editable.duration = EMPTY_CLIP_DEFAULT_DURATION_SECONDS;
    let source_id = {
        let mut clip_library = world.resource_mut::<ClipLibrary>();
        clip_library_register_and_activate(&mut clip_library, assets, editable)
    };

    world.resource_mut::<TimelineState>().current_clip_id = Some(source_id);

    log!(
        "Auto-created empty animation clip 'New Animation' (source_id={}, duration={}s) for model with no animations",
        source_id,
        EMPTY_CLIP_DEFAULT_DURATION_SECONDS
    );

    source_id
}

pub(super) fn restore_batch_playback(world: &World) {
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
    if let Some(id) = clip_id {
        timeline.current_clip_id = Some(id);
    }
}

pub fn build_initial_clip_schedule(
    first_source_id: Option<SourceClipId>,
    world: &World,
) -> ClipSchedule {
    let mut schedule = ClipSchedule::new();

    let Some(source_id) = first_source_id else {
        return schedule;
    };

    let duration = world
        .resource::<ClipLibrary>()
        .get(source_id)
        .map(|c| c.duration)
        .unwrap_or(1.0);

    clip_schedule_add_instance(&mut schedule, source_id, duration);
    schedule
}
