use crate::asset::AssetStorage;
use crate::ecs::component::ClipSchedule;
use crate::ecs::events::UIEvent;
use crate::ecs::resource::ClipLibrary;
use crate::ecs::world::World;

pub fn dispatch_spring_bone_bake_ecs_events(
    events: &[UIEvent],
    world: &mut World,
    assets: &mut AssetStorage,
) {
    for event in events {
        match event {
            UIEvent::SpringBoneBake => {
                handle_spring_bone_bake(world, assets);
            }
            UIEvent::SpringBoneDiscardBake => {
                handle_spring_bone_discard(world, assets);
            }
            UIEvent::SpringBoneRebake => {
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

fn handle_spring_bone_bake(world: &mut World, assets: &mut AssetStorage) {
    use crate::ecs::component::{ConstraintSet, SpringBoneSetup, WithSpringBone};
    use crate::ecs::resource::TimelineState;
    use crate::ecs::resource::{SpringBoneMode, SpringBoneState};
    use crate::ecs::systems::spring_bone_bake_systems::{
        merge_bake_into_clip, spring_bone_bake, BakeConfig,
    };

    let skeleton = match assets.skeletons.values().next() {
        Some(skel_asset) => skel_asset.skeleton.clone(),
        None => {
            msg_warn!("Spring bone bake failed: no skeleton found");
            return;
        }
    };

    let spring_entity = world
        .iter_components::<WithSpringBone>()
        .next()
        .map(|(entity, _)| entity);

    let Some(entity) = spring_entity else {
        msg_warn!("Spring bone bake failed: no WithSpringBone entity");
        return;
    };

    let setup = match world.get_component::<SpringBoneSetup>(entity) {
        Some(s) => s.clone(),
        None => {
            msg_warn!("Spring bone bake failed: no SpringBoneSetup");
            return;
        }
    };

    let constraints = world.get_component::<ConstraintSet>(entity).cloned();

    let timeline_state = world.resource::<TimelineState>();
    let source_id = timeline_state.current_clip_id;
    let looping = timeline_state.looping;
    drop(timeline_state);

    let clip_library = world.resource::<ClipLibrary>();
    let (anim_clip, source_editable) = match source_id.and_then(|id| clip_library.get(id)) {
        Some(editable) => (
            crate::animation::editable::clip_to_animation(editable),
            editable.clone(),
        ),
        None => {
            drop(clip_library);
            msg_warn!("Spring bone bake failed: no current clip");
            return;
        }
    };
    drop(clip_library);

    let config = BakeConfig {
        start_time: 0.0,
        end_time: anim_clip.duration,
        sample_rate: 30.0,
    };

    let bake_result = spring_bone_bake(
        &config,
        &setup,
        &skeleton,
        &anim_clip,
        constraints.as_ref(),
        looping,
    );

    let mut merged = source_editable;
    merge_bake_into_clip(&mut merged, &bake_result, &skeleton);
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

    let new_id = register_baked_clip_and_update_schedules(world, assets, merged, source_id);

    let baked_bone_ids = bake_result.baked_bone_ids.clone();

    let mut spring_state = world.resource_mut::<SpringBoneState>();
    spring_state.mode = SpringBoneMode::Baked;
    spring_state.baked_clip_source_id = Some(new_id);
    spring_state.baked_bone_ids = bake_result.baked_bone_ids;
    spring_state.original_clip_source_id = source_id;

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

pub fn dispatch_spring_bone_edit_events(
    events: &[UIEvent],
    world: &mut World,
    assets: &AssetStorage,
) {
    use crate::ecs::systems::spring_bone_edit_systems::*;

    let skeleton = assets
        .skeletons
        .values()
        .next()
        .map(|sa| sa.skeleton.clone());

    for event in events {
        match event {
            UIEvent::SpringChainAdd {
                entity,
                root_bone_id,
                chain_length,
            } => {
                if let Some(ref skel) = skeleton {
                    handle_spring_chain_add(world, *entity, *root_bone_id, *chain_length, skel);
                }
            }

            UIEvent::SpringChainRemove { entity, chain_id } => {
                handle_spring_chain_remove(world, *entity, *chain_id);
            }

            UIEvent::SpringChainUpdate {
                entity,
                chain_id,
                chain,
            } => {
                handle_spring_chain_update(world, *entity, *chain_id, chain.clone());
            }

            UIEvent::SpringJointUpdate {
                entity,
                chain_id,
                joint_index,
                joint,
            } => {
                handle_spring_joint_update(world, *entity, *chain_id, *joint_index, joint.clone());
            }

            UIEvent::SpringColliderAdd {
                entity,
                bone_id,
                shape,
            } => {
                handle_spring_collider_add(world, *entity, *bone_id, shape.clone());
            }

            UIEvent::SpringColliderRemove {
                entity,
                collider_id,
            } => {
                handle_spring_collider_remove(world, *entity, *collider_id);
            }

            UIEvent::SpringColliderUpdate {
                entity,
                collider_id,
                collider,
            } => {
                handle_spring_collider_update(world, *entity, *collider_id, collider.clone());
            }

            UIEvent::SpringColliderGroupAdd { entity, name } => {
                handle_spring_collider_group_add(world, *entity, name.clone());
            }

            UIEvent::SpringColliderGroupRemove { entity, group_id } => {
                handle_spring_collider_group_remove(world, *entity, *group_id);
            }

            UIEvent::SpringColliderGroupUpdate {
                entity,
                group_id,
                group,
            } => {
                handle_spring_collider_group_update(world, *entity, *group_id, group.clone());
            }

            UIEvent::SpringBoneToggleGizmo(visible) => {
                if let Some(mut gizmo) =
                    world.get_resource_mut::<crate::ecs::resource::gizmo::SpringBoneGizmoData>()
                {
                    gizmo.visible = *visible;
                }
            }

            _ => {}
        }
    }
}
