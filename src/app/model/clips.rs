use thyllore_avatar_core::humanoid::components::mapping::HumanoidMapping;
use thyllore_avatar_core::humanoid::components::role::HumanoidRole;
use thyllore_avatar_core::humanoid::components::vrm_version::VrmVersion;

use crate::animation::editable::{EditableAnimationClip, SourceClipId};
use crate::animation::AnimationClip;
use crate::asset::{AssetStorage, SkeletonAsset};
use crate::ecs::component::ClipSchedule;
use crate::ecs::resource::{BatchRun, ClipLibrary, ModelState, TimelineState};
use crate::ecs::systems::clip_library_systems::{
    clip_library_register_and_activate, find_best_clip,
};
use crate::ecs::systems::clip_schedule_systems::clip_schedule_add_instance;
use crate::ecs::systems::{
    build_humanoid_rig, convert_clip_to_standard_space, find_first_skeleton, find_model_path,
    timeline_apply_fit_zoom,
};
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
    if let Some(vrm) = &load_result.vrm_humanoid {
        let version = if vrm.is_v1 {
            VrmVersion::V1
        } else {
            VrmVersion::V0
        };
        let mut by_role = std::collections::BTreeMap::new();
        for (name, joint) in &vrm.bones {
            if let Some(role) = HumanoidRole::from_vrm_name(name, version) {
                by_role.insert(role, *joint as usize);
            }
        }
        if !by_role.is_empty() {
            model_state.imported_humanoid = Some(HumanoidMapping { by_role });
        } else {
            model_state.imported_humanoid = None;
        }
    } else {
        model_state.imported_humanoid = None;
    }
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

fn import_editable_clips(
    world: &World,
    assets: &AssetStorage,
    loaded_clips: &[AnimationClip],
    bone_names: &std::collections::HashMap<u32, String>,
) -> Vec<EditableAnimationClip> {
    let editable_clips = loaded_clips
        .iter()
        .map(|clip| crate::animation::editable::clip_from_animation(0, clip, bone_names));

    let (Some(model_path), Some(skeleton)) = (find_model_path(world), find_first_skeleton(assets))
    else {
        return editable_clips.collect();
    };
    let imported = world.resource::<ModelState>().imported_humanoid.clone();
    let Some(rig) = build_humanoid_rig(
        std::path::Path::new(&model_path),
        skeleton,
        imported.as_ref(),
    ) else {
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
    loaded_clips: &[AnimationClip],
) -> Option<SourceClipId> {
    let bone_names: std::collections::HashMap<u32, String> = assets
        .skeletons
        .values()
        .flat_map(|sa| sa.skeleton.bones.iter().map(|b| (b.id, b.name.clone())))
        .collect();

    let editable_clips = import_editable_clips(world, assets, loaded_clips, &bone_names);

    let mut first_editable_clip_id = None;
    let mut model_ids: Vec<SourceClipId> = Vec::new();
    {
        let mut clip_library = world.resource_mut::<ClipLibrary>();
        for editable in editable_clips {
            let clip_name = editable.name.clone();
            let editable_id =
                clip_library_register_and_activate(&mut clip_library, assets, editable);
            model_ids.push(editable_id);
            first_editable_clip_id.get_or_insert(editable_id);
            log!(
                "Registered clip '{}' (source_id={})",
                clip_name,
                editable_id,
            );
        }
        clip_library.model_clip_ids.extend(model_ids.iter());
    }

    let editable_id = first_editable_clip_id?;
    let clip_duration = world
        .resource::<ClipLibrary>()
        .get(editable_id)
        .map(|c| c.duration)
        .unwrap_or(0.0);
    let keeps_user = keeps_selected_user_clip(world);
    let mut timeline_state = world.resource_mut::<TimelineState>();
    if !keeps_user {
        timeline_state.current_clip_id = Some(editable_id);
        timeline_apply_fit_zoom(&mut timeline_state, clip_duration);
    }
    log!("Set timeline current_clip_id to {}", editable_id);

    Some(editable_id)
}

fn register_empty_editable_clip(world: &mut World, assets: &mut AssetStorage) -> SourceClipId {
    let mut editable = EditableAnimationClip::new(0, "New Animation".to_string());
    editable.duration = EMPTY_CLIP_DEFAULT_DURATION_SECONDS;
    let source_id = {
        let mut clip_library = world.resource_mut::<ClipLibrary>();
        let id = clip_library_register_and_activate(&mut clip_library, assets, editable);
        clip_library.model_clip_ids.insert(id);
        id
    };

    if !keeps_selected_user_clip(world) {
        world.resource_mut::<TimelineState>().current_clip_id = Some(source_id);
    }

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

pub fn keeps_selected_user_clip(world: &World) -> bool {
    let timeline = match world.get_resource::<TimelineState>() {
        Some(t) => t,
        None => return false,
    };
    let Some(current_id) = timeline.current_clip_id else {
        return false;
    };
    let library = match world.get_resource::<ClipLibrary>() {
        Some(l) => l,
        None => return false,
    };
    library.get(current_id).is_some() && !library.model_clip_ids.contains(&current_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::Skeleton;

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
        register_clips_to_library(&mut world, &mut assets, &loaded_clips);

        assert_eq!(
            world.resource::<TimelineState>().current_clip_id,
            Some(user_source_id)
        );
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
