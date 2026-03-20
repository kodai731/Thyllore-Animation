use crate::animation::SkeletonId;
use crate::asset::AssetStorage;
use crate::ecs::component::{AnimationMeta, ClipSchedule};
use crate::ecs::compute_local_time;
use crate::ecs::resource::{ClipLibrary, SpringBoneMode, SpringBoneState};
use crate::ecs::world::{Animator, Entity, MeshRef, World};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

use super::{ActiveInstanceInfo, AnimatedEntityInfo};

pub(crate) fn collect_animated_entities(
    world: &World,
    graphics: &GraphicsResources,
    clip_library: &ClipLibrary,
    assets: &AssetStorage,
) -> Vec<AnimatedEntityInfo> {
    let mut infos = Vec::new();

    let is_baked = world
        .get_resource::<SpringBoneState>()
        .map_or(false, |s| s.mode == SpringBoneMode::Baked);
    let should_log = is_baked
        && world
            .get_resource::<SpringBoneState>()
            .map_or(false, |s| s.frame_count < 3);

    for (parent_entity, animator) in world.iter_components::<Animator>() {
        let Some(schedule) = world.get_component::<ClipSchedule>(parent_entity) else {
            if should_log {
                log!(
                    "[PlaybackDebug] entity {:?}: no ClipSchedule",
                    parent_entity
                );
            }
            continue;
        };
        let Some(meta) = world.get_component::<AnimationMeta>(parent_entity) else {
            continue;
        };

        if should_log {
            let src_id = schedule.instances.first().map(|i| i.source_id);
            let asset_id = src_id.and_then(|sid| clip_library.get_asset_id_for_source(sid));
            let asset_exists =
                asset_id.map_or(false, |aid| assets.animation_clips.contains_key(&aid));
            log!(
                "[PlaybackDebug] entity {:?}: source_id={:?}, asset_id={:?}, asset_exists={}, time={:.3}, instances={}",
                parent_entity, src_id, asset_id, asset_exists, animator.time, schedule.instances.len()
            );
        }

        let active_instances = build_active_instances(schedule, clip_library, animator);

        if should_log && active_instances.is_empty() {
            log!(
                "[PlaybackDebug] entity {:?}: active_instances is EMPTY",
                parent_entity
            );
        }

        if active_instances.is_empty() {
            continue;
        }

        collect_mesh_entities(
            world,
            graphics,
            assets,
            parent_entity,
            animator,
            meta,
            &active_instances,
            &mut infos,
        );
    }

    if should_log {
        log!("[PlaybackDebug] total animated entities: {}", infos.len());
    }

    infos
}

fn collect_mesh_entities(
    world: &World,
    graphics: &GraphicsResources,
    assets: &AssetStorage,
    parent_entity: Entity,
    animator: &Animator,
    meta: &AnimationMeta,
    active_instances: &[ActiveInstanceInfo],
    infos: &mut Vec<AnimatedEntityInfo>,
) {
    let child_meshes = world.find_child_mesh_entities(parent_entity);
    for mesh_entity in child_meshes {
        let Some(mesh_ref) = world.get_component::<MeshRef>(mesh_entity) else {
            continue;
        };
        let Some(mesh_asset) = assets.get_mesh(mesh_ref.mesh_asset_id) else {
            continue;
        };

        let mesh_idx = mesh_asset.graphics_mesh_index;
        if mesh_idx >= graphics.meshes.len() {
            continue;
        }

        let skeleton_id = resolve_skeleton_id(mesh_asset, graphics, mesh_idx);
        let Some(skel_id) = skeleton_id else {
            continue;
        };

        infos.push(AnimatedEntityInfo {
            entity: parent_entity,
            active_instances: active_instances.to_vec(),
            skeleton_id: skel_id,
            mesh_idx,
            animation_type: meta.animation_type.clone(),
            node_animation_scale: meta.node_animation_scale,
            looping: animator.looping,
        });
    }
}

fn resolve_skeleton_id(
    mesh_asset: &crate::asset::MeshAsset,
    graphics: &GraphicsResources,
    mesh_idx: usize,
) -> Option<SkeletonId> {
    mesh_asset
        .skeleton_id
        .or_else(|| graphics.meshes.get(mesh_idx).and_then(|m| m.skeleton_id))
}

pub(crate) fn build_active_instances(
    schedule: &ClipSchedule,
    clip_library: &ClipLibrary,
    animator: &Animator,
) -> Vec<ActiveInstanceInfo> {
    let active = crate::ecs::systems::clip_schedule_systems::clip_schedule_active_instances(
        schedule,
        animator.time,
    );

    let mut instances: Vec<ActiveInstanceInfo> = active
        .into_iter()
        .filter_map(|inst| {
            let asset_id = clip_library.get_asset_id_for_source(inst.source_id)?;

            let local_time = compute_local_time(
                animator.time,
                inst.start_time,
                inst.clip_in,
                inst.clip_out,
                inst.speed,
                inst.cycle_count,
                animator.looping,
            );

            let weight = crate::ecs::systems::clip_schedule_systems::clip_schedule_effective_weight(
                schedule,
                inst.instance_id,
            );

            Some(ActiveInstanceInfo {
                source_id: inst.source_id,
                asset_id,
                instance_id: inst.instance_id,
                local_time,
                weight,
                blend_mode: inst.blend_mode,
                ease_out: inst.ease_out,
                start_time: inst.start_time,
                end_time: inst.end_time(),
            })
        })
        .collect();

    instances.sort_by(|a, b| {
        a.start_time
            .partial_cmp(&b.start_time)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    instances
}
