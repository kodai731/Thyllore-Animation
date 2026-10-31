use super::event::SpringBoneEvent;
use crate::animation::spring_bone::SpringSpace;
use crate::asset::AssetStorage;
use crate::ecs::component::ClipSchedule;
use crate::ecs::resource::ClipLibrary;
use crate::ecs::systems::spring_bone_systems::spring_space_for_entity;
use crate::ecs::world::{Entity, World};

pub(super) fn dispatch_spring_bone_bake_events(
    events: &[SpringBoneEvent],
    world: &mut World,
    assets: &mut AssetStorage,
) {
    for event in events {
        match event {
            SpringBoneEvent::Bake => {
                handle_spring_bone_bake(world, assets);
            }
            SpringBoneEvent::DiscardBake => {
                handle_spring_bone_discard(world, assets);
            }
            SpringBoneEvent::Rebake => {
                handle_spring_bone_discard(world, assets);
                handle_spring_bone_bake(world, assets);
            }
            _ => {}
        }
    }
}

pub fn transition_to_baked_override_if_needed(world: &mut World) {
    use crate::ecs::resource::{SpringBoneMode, SpringBoneState};

    if let Some(mut state) = world.get_resource_mut::<SpringBoneState>() {
        if state.mode == SpringBoneMode::Baked {
            state.mode = SpringBoneMode::BakedOverride;
            log!("Spring bone mode: Baked -> BakedOverride (manual edit detected)");
        }
    }
}

pub(super) struct SpringBakeInputs {
    pub(super) entity: Entity,
    pub(super) space: SpringSpace,
    pub(super) skeleton: crate::animation::Skeleton,
    pub(super) setup: crate::ecs::component::SpringBoneSetup,
    pub(super) constraints: Option<crate::ecs::component::ConstraintSet>,
    pub(super) source_id: Option<u64>,
    pub(super) looping: bool,
    pub(super) anim_clip: crate::animation::AnimationClip,
    pub(super) source_editable: crate::animation::editable::EditableAnimationClip,
}

pub(super) fn find_spring_bake_inputs(
    world: &World,
    assets: &AssetStorage,
) -> Option<SpringBakeInputs> {
    use crate::ecs::component::{ConstraintSet, SpringBoneSetup, WithSpringBone};
    use crate::ecs::resource::{BakedHumanoidClips, TimelineState};

    let skeleton = match assets.skeletons.values().next() {
        Some(skel_asset) => skel_asset.skeleton.clone(),
        None => {
            msg_warn!("Spring bone bake failed: no skeleton found");
            return None;
        }
    };

    let spring_entity = world
        .iter_components::<WithSpringBone>()
        .next()
        .map(|(entity, _)| entity);

    let Some(entity) = spring_entity else {
        msg_warn!("Spring bone bake failed: no WithSpringBone entity");
        return None;
    };

    let setup = match world.get_component::<SpringBoneSetup>(entity) {
        Some(s) => s.clone(),
        None => {
            msg_warn!("Spring bone bake failed: no SpringBoneSetup");
            return None;
        }
    };

    let constraints = world.get_component::<ConstraintSet>(entity).cloned();

    let timeline_state = world.resource::<TimelineState>();
    let source_id = timeline_state.current_clip_id;
    let looping = timeline_state.looping;
    drop(timeline_state);

    let clip_library = world.resource::<ClipLibrary>();
    let source_editable = match source_id.and_then(|id| clip_library.get(id)) {
        Some(editable) => editable.clone(),
        None => {
            drop(clip_library);
            msg_warn!("Spring bone bake failed: no current clip");
            return None;
        }
    };

    let baked = match world.get_resource::<BakedHumanoidClips>() {
        Some(b) => b,
        None => {
            drop(clip_library);
            msg_warn!("Spring bone bake failed: no BakedHumanoidClips resource");
            return None;
        }
    };

    let source_id = match source_id {
        Some(id) => id,
        None => {
            drop(clip_library);
            msg_warn!("Spring bone bake failed: no current clip id");
            return None;
        }
    };

    let asset_id = match crate::ecs::systems::animation::collect::resolve_playable_asset(
        source_id,
        entity,
        &clip_library,
        &baked,
    ) {
        Some(id) => id,
        None => {
            drop(clip_library);
            msg_warn!("Spring bone bake failed: no playable asset for source");
            return None;
        }
    };
    drop(clip_library);

    let anim_clip = match assets.animation_clips.get(&asset_id) {
        Some(entry) => entry.clip.clone(),
        None => {
            msg_warn!("Spring bone bake failed: animation clip asset not found");
            return None;
        }
    };

    Some(SpringBakeInputs {
        entity,
        space: spring_space_for_entity(world, entity),
        skeleton,
        setup,
        constraints,
        source_id: Some(source_id),
        looping,
        anim_clip,
        source_editable,
    })
}

fn handle_spring_bone_bake(world: &mut World, assets: &mut AssetStorage) {
    use crate::ecs::resource::{SpringBoneMode, SpringBoneState, TimelineState};
    use crate::ecs::systems::spring_bone_bake_systems::{
        merge_bake_into_clip, spring_bone_bake, BakeConfig,
    };

    let inputs = match find_spring_bake_inputs(world, assets) {
        Some(i) => i,
        None => return,
    };

    let config = BakeConfig {
        start_time: 0.0,
        end_time: inputs.anim_clip.duration,
        sample_rate: 30.0,
        space: inputs.space,
    };

    let bake_result = spring_bone_bake(
        &config,
        &inputs.setup,
        &inputs.skeleton,
        &inputs.anim_clip,
        inputs.constraints.as_ref(),
        inputs.looping,
    );

    let mut merged = inputs.source_editable;
    merge_bake_into_clip(&mut merged, &bake_result, &inputs.skeleton);
    merged.name = format!("{}_spring_baked", merged.name);

    log!(
        "[BakeDebug] bake_result: baked_bone_ids={:?}, clip_tracks={}",
        bake_result.baked_bone_ids,
        bake_result.clip.tracks.len()
    );
    log!(
        "[BakeDebug] merged clip: name={}, tracks={}, duration={}",
        merged.name,
        merged.tracks.len(),
        merged.duration
    );

    let new_id = register_baked_clip_and_update_schedules(world, assets, merged, inputs.source_id);

    let baked_bone_ids = bake_result.baked_bone_ids.clone();

    let mut spring_state = world.resource_mut::<SpringBoneState>();
    spring_state.mode = SpringBoneMode::Baked;
    spring_state.baked_clip_source_id = Some(new_id);
    spring_state.baked_bone_ids = bake_result.baked_bone_ids;
    spring_state.original_clip_source_id = inputs.source_id;

    let mut timeline_state = world.resource_mut::<TimelineState>();
    timeline_state.current_clip_id = Some(new_id);
    timeline_state.baked_bone_ids = baked_bone_ids;

    log!("Spring bone baked to new clip (id={})", new_id);
}

fn register_baked_clip_and_update_schedules(
    world: &mut World,
    assets: &mut AssetStorage,
    clip: crate::animation::editable::EditableAnimationClip,
    source_id: Option<u64>,
) -> u64 {
    use crate::ecs::systems::clip_library_systems::clip_library_register_and_activate;

    let mut clip_library = world.resource_mut::<ClipLibrary>();
    let new_id = clip_library_register_and_activate(&mut clip_library, assets, clip);
    drop(clip_library);

    let mut updated_count = 0;
    let schedule_entities = world.component_entities::<ClipSchedule>();
    log!(
        "[BakeDebug] ClipSchedule entities count={}, original source_id={:?}",
        schedule_entities.len(),
        source_id
    );
    for sched_entity in &schedule_entities {
        if let Some(schedule) = world.get_component_mut::<ClipSchedule>(*sched_entity) {
            if let Some(first) = schedule.instances.first_mut() {
                log!(
                    "[BakeDebug]   entity {:?}: schedule source_id={}, match={}",
                    sched_entity,
                    first.source_id,
                    Some(first.source_id) == source_id
                );
                if Some(first.source_id) == source_id {
                    first.source_id = new_id;
                    updated_count += 1;
                }
            }
        }
    }
    log!(
        "[BakeDebug] updated {} ClipSchedule(s) to new source_id={}",
        updated_count,
        new_id
    );

    new_id
}

pub fn handle_spring_bone_discard(world: &mut World, assets: &mut AssetStorage) {
    use crate::ecs::resource::{SpringBoneMode, SpringBoneState, TimelineState};

    let mut spring_state = world.resource_mut::<SpringBoneState>();
    let original_id = spring_state.original_clip_source_id;
    let baked_id = spring_state.baked_clip_source_id;

    spring_state.mode = SpringBoneMode::Realtime;
    spring_state.baked_clip_source_id = None;
    spring_state.baked_bone_ids = Vec::new();
    spring_state.original_clip_source_id = None;
    spring_state.initialized = false;
    drop(spring_state);

    if let (Some(orig_id), Some(baked_source_id)) = (original_id, baked_id) {
        let schedule_entities = world.component_entities::<ClipSchedule>();
        for entity in &schedule_entities {
            if let Some(schedule) = world.get_component_mut::<ClipSchedule>(*entity) {
                if let Some(first) = schedule.instances.first_mut() {
                    if first.source_id == baked_source_id {
                        first.source_id = orig_id;
                    }
                }
            }
        }
    }

    if let Some(baked_id) = baked_id {
        let mut clip_library = world.resource_mut::<ClipLibrary>();
        if let Some(asset_id) = clip_library.source_to_asset_id.remove(&baked_id) {
            assets.animation_clips.remove(&asset_id);
        }
        clip_library.remove(baked_id);
    }

    let mut timeline_state = world.resource_mut::<TimelineState>();
    timeline_state.baked_bone_ids.clear();
    if let Some(orig) = original_id {
        timeline_state.current_clip_id = Some(orig);
    }

    log!("Discarded spring bone bake, restored original clip");
}
