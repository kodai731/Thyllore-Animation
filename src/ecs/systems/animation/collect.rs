use crate::animation::editable::{BlendMode, EaseType, SourceClipId};
use crate::animation::SkeletonId;
use crate::asset::AssetStorage;
use crate::ecs::component::{AnimationMeta, ClipSchedule};
use crate::ecs::compute_local_time;
use crate::ecs::resource::{
    BakedHumanoidClips, ClipLibrary, ClipPreview, SpringBoneMode, SpringBoneState, TimelineState,
};
use crate::ecs::world::{Animator, Entity, MeshRef, World};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

use super::{ActiveInstanceInfo, AnimatedEntityInfo};

fn resolve_playable_asset(
    source_id: SourceClipId,
    entity: Entity,
    clip_library: &ClipLibrary,
    baked: &BakedHumanoidClips,
) -> Option<crate::asset::AssetId> {
    if let Some(baked_entry) = baked.by_key.get(&(source_id, entity)) {
        return Some(baked_entry.asset_id);
    }
    clip_library.get_asset_id_for_source(source_id)
}

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

    let solo_clip_id = world
        .get_resource::<TimelineState>()
        .filter(|timeline| timeline.preview == ClipPreview::Solo)
        .and_then(|timeline| timeline.current_clip_id);
    let solo_preview = solo_clip_id.and_then(|clip_id| {
        crate::ecs::systems::clip_schedule_systems::find_preview_owner(world)
            .map(|owner| (owner, clip_id))
    });

    let empty_baked = BakedHumanoidClips::default();
    let baked_ref = world.get_resource::<BakedHumanoidClips>();
    let baked: &BakedHumanoidClips = baked_ref.as_deref().unwrap_or(&empty_baked);

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

        let active_instances = match solo_preview {
            Some((owner, clip_id)) if owner == parent_entity => {
                build_solo_instance(clip_id, parent_entity, clip_library, baked, animator)
                    .into_iter()
                    .collect()
            }
            _ => build_active_instances(schedule, parent_entity, clip_library, baked, animator),
        };

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
    entity: Entity,
    clip_library: &ClipLibrary,
    baked: &BakedHumanoidClips,
    animator: &Animator,
) -> Vec<ActiveInstanceInfo> {
    let active = crate::ecs::systems::clip_schedule_systems::clip_schedule_active_instances(
        schedule,
        animator.time,
    );

    let mut instances: Vec<ActiveInstanceInfo> = active
        .into_iter()
        .filter_map(|inst| {
            let asset_id = resolve_playable_asset(inst.source_id, entity, clip_library, baked)?;

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
                instance_id: Some(inst.instance_id),
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

pub(crate) fn build_solo_instance(
    clip_id: SourceClipId,
    entity: Entity,
    clip_library: &ClipLibrary,
    baked: &BakedHumanoidClips,
    animator: &Animator,
) -> Option<ActiveInstanceInfo> {
    let asset_id = resolve_playable_asset(clip_id, entity, clip_library, baked)?;
    let duration = clip_library.get(clip_id)?.duration;

    let local_time = compute_local_time(
        animator.time,
        0.0,
        0.0,
        duration,
        1.0,
        1.0,
        animator.looping,
    );

    Some(ActiveInstanceInfo {
        source_id: clip_id,
        asset_id,
        instance_id: None,
        local_time,
        weight: 1.0,
        blend_mode: BlendMode::Override,
        ease_out: EaseType::Linear,
        start_time: 0.0,
        end_time: duration,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::editable::{EditableAnimationClip, SourceClip};
    use crate::ecs::systems::clip_schedule_systems::clip_schedule_add_instance;

    fn register_clip(clip_library: &mut ClipLibrary, clip_id: SourceClipId, duration: f32) {
        let mut clip = EditableAnimationClip::new(clip_id, format!("clip_{clip_id}"));
        clip.duration = duration;
        clip_library
            .source_clips
            .insert(clip_id, SourceClip::new(clip_id, clip));
        clip_library
            .source_to_asset_id
            .insert(clip_id, clip_id + 100);
    }

    fn animator_at(time: f32) -> Animator {
        Animator {
            time,
            ..Animator::new()
        }
    }

    fn collect_solo(
        clip_id: SourceClipId,
        entity: Entity,
        clip_library: &ClipLibrary,
        baked: &BakedHumanoidClips,
        animator: &Animator,
    ) -> Vec<ActiveInstanceInfo> {
        build_solo_instance(clip_id, entity, clip_library, baked, animator)
            .into_iter()
            .collect()
    }

    #[test]
    fn solo_preview_plays_current_clip_without_instances() {
        let mut clip_library = ClipLibrary::new();
        register_clip(&mut clip_library, 1, 2.0);
        let schedule = ClipSchedule::new();
        let animator = animator_at(0.5);
        let baked = BakedHumanoidClips::default();
        let entity: Entity = 0;

        assert!(
            build_active_instances(&schedule, entity, &clip_library, &baked, &animator).is_empty()
        );

        let instances = collect_solo(1, entity, &clip_library, &baked, &animator);

        assert_eq!(instances.len(), 1);
        let solo = &instances[0];
        assert_eq!(solo.source_id, 1);
        assert_eq!(solo.asset_id, 101);
        assert_eq!(solo.instance_id, None);
        assert_eq!(solo.weight, 1.0);
        assert_eq!(solo.blend_mode, BlendMode::Override);
        assert_eq!(solo.start_time, 0.0);
        assert_eq!(solo.end_time, 2.0);
        assert!((solo.local_time - 0.5).abs() < 1e-6);
    }

    #[test]
    fn solo_preview_ignores_schedule() {
        let mut clip_library = ClipLibrary::new();
        register_clip(&mut clip_library, 1, 2.0);
        register_clip(&mut clip_library, 2, 2.0);
        register_clip(&mut clip_library, 3, 2.0);
        let mut schedule = ClipSchedule::new();
        clip_schedule_add_instance(&mut schedule, 1, 2.0);
        clip_schedule_add_instance(&mut schedule, 2, 2.0);
        schedule.instances[1].muted = true;
        let muted_before: Vec<bool> = schedule.instances.iter().map(|i| i.muted).collect();
        let animator = animator_at(0.5);
        let baked = BakedHumanoidClips::default();
        let entity: Entity = 0;

        let instances = collect_solo(3, entity, &clip_library, &baked, &animator);

        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].source_id, 3);
        assert_eq!(instances[0].weight, 1.0);
        let muted_after: Vec<bool> = schedule.instances.iter().map(|i| i.muted).collect();
        assert_eq!(muted_after, muted_before);
    }

    #[test]
    fn mix_respects_mute_and_weight() {
        let mut clip_library = ClipLibrary::new();
        register_clip(&mut clip_library, 1, 2.0);
        register_clip(&mut clip_library, 2, 2.0);
        let mut schedule = ClipSchedule::new();
        let audible_id = clip_schedule_add_instance(&mut schedule, 1, 2.0);
        clip_schedule_add_instance(&mut schedule, 2, 2.0);
        schedule.instances[0].weight = 0.4;
        schedule.instances[1].muted = true;
        let animator = animator_at(0.5);
        let baked = BakedHumanoidClips::default();
        let entity: Entity = 0;

        let instances = build_active_instances(&schedule, entity, &clip_library, &baked, &animator);

        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].source_id, 1);
        assert_eq!(instances[0].instance_id, Some(audible_id));
        assert!((instances[0].weight - 0.4).abs() < 1e-6);
    }

    #[test]
    fn clip_instance_plays_the_baked_asset() {
        let mut clip_library = ClipLibrary::new();
        let mut clip = EditableAnimationClip::new(1, "role".to_string());
        clip.duration = 2.0;
        clip_library
            .source_clips
            .insert(1, SourceClip::new(1, clip));

        let baked = {
            let mut baked = BakedHumanoidClips::default();
            baked.by_key.insert(
                (1, 42),
                crate::ecs::resource::BakedHumanoidClip {
                    fps: 30,
                    asset_id: 999,
                    clip: EditableAnimationClip::new(1, "baked".to_string()),
                },
            );
            baked
        };

        let mut schedule = ClipSchedule::new();
        clip_schedule_add_instance(&mut schedule, 1, 2.0);
        let animator = animator_at(0.5);

        let instances = build_active_instances(&schedule, 42, &clip_library, &baked, &animator);

        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].source_id, 1);
        assert_eq!(instances[0].asset_id, 999);
    }

    #[test]
    fn clip_without_bake_is_skipped() {
        let mut clip_library = ClipLibrary::new();
        let mut clip = EditableAnimationClip::new(1, "role".to_string());
        clip.duration = 2.0;
        clip_library
            .source_clips
            .insert(1, SourceClip::new(1, clip));

        let baked = BakedHumanoidClips::default();

        let mut schedule = ClipSchedule::new();
        clip_schedule_add_instance(&mut schedule, 1, 2.0);
        let animator = animator_at(0.5);

        let instances = build_active_instances(&schedule, 42, &clip_library, &baked, &animator);

        assert!(instances.is_empty());
    }
}
