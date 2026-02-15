use anyhow::Result;
use cgmath::Matrix4;

use crate::animation::editable::{BlendMode, ClipInstanceId, EaseType, SourceClipId};
use crate::animation::{MorphAnimationSystem, SkeletonId, SkeletonPose};
use crate::asset::AssetId;
use crate::app::graphics_resource::{GraphicsResources, NodeData};
use crate::asset::AssetStorage;
use crate::ecs::component::{
    AnimationMeta, ClipSchedule, ConstraintSet, SpringBoneSetup, WithSpringBone,
};
use crate::ecs::resource::{AnimationType, ClipLibrary, SpringBoneMode, SpringBoneState};
use crate::ecs::world::{Animator, Entity, MeshRef, World};
use crate::ecs::{
    blend_poses_override, compute_crossfade_factor, compute_local_time,
    compute_pose_global_transforms, create_pose_from_rest, sample_clip_to_pose,
};
use crate::render::RenderBackend;

use super::constraint_solve_systems::apply_constraints;
use super::pose_blend_systems::blend_poses_additive;

pub struct AnimationEvalResult {
    pub updated_meshes: Vec<usize>,
    pub bone_transforms: Option<(SkeletonId, Vec<Matrix4<f32>>)>,
}

struct ActiveInstanceInfo {
    source_id: SourceClipId,
    asset_id: AssetId,
    instance_id: ClipInstanceId,
    local_time: f32,
    weight: f32,
    blend_mode: BlendMode,
    ease_out: EaseType,
    start_time: f32,
    end_time: f32,
}

struct AnimatedEntityInfo {
    entity: Entity,
    active_instances: Vec<ActiveInstanceInfo>,
    skeleton_id: SkeletonId,
    mesh_idx: usize,
    animation_type: AnimationType,
    node_animation_scale: f32,
    looping: bool,
}

pub fn evaluate_all_animators(
    world: &World,
    graphics: &mut GraphicsResources,
    nodes: &mut [NodeData],
    clip_library: &ClipLibrary,
    assets: &AssetStorage,
    dt: f32,
) -> AnimationEvalResult {
    let entity_infos = collect_animated_entities(world, graphics, clip_library, assets);

    let first_time = world
        .iter_components::<Animator>()
        .next()
        .map(|(_, a)| a.time)
        .unwrap_or(0.0);

    let morph_updated = if !clip_library.morph_animation.is_empty() {
        playback_apply_morph_animation(graphics, &clip_library.morph_animation, first_time)
    } else {
        Vec::new()
    };

    if entity_infos.is_empty() {
        return AnimationEvalResult {
            updated_meshes: morph_updated,
            bone_transforms: None,
        };
    }

    let (anim_updated, bone_transforms) =
        apply_blended_animations(&entity_infos, world, graphics, nodes, assets, dt);

    AnimationEvalResult {
        updated_meshes: merge_updated_indices(morph_updated, anim_updated),
        bone_transforms,
    }
}

fn collect_animated_entities(
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

    for (entity, animator) in world.iter_components::<Animator>() {
        let Some(schedule) = world.get_component::<ClipSchedule>(entity) else {
            if should_log {
                crate::log!("[PlaybackDebug] entity {:?}: no ClipSchedule", entity);
            }
            continue;
        };
        let Some(meta) = world.get_component::<AnimationMeta>(entity) else {
            continue;
        };
        let Some(mesh_ref) = world.get_component::<MeshRef>(entity) else {
            continue;
        };

        let Some(mesh_asset) = assets.get_mesh(mesh_ref.mesh_asset_id) else {
            continue;
        };

        let mesh_idx = mesh_asset.graphics_mesh_index;
        if mesh_idx >= graphics.meshes.len() {
            continue;
        }

        let skeleton_id = mesh_asset
            .skeleton_id
            .or_else(|| graphics.meshes.get(mesh_idx).and_then(|m| m.skeleton_id));
        let Some(skel_id) = skeleton_id else {
            continue;
        };

        if should_log {
            let src_id = schedule.instances.first().map(|i| i.source_id);
            let asset_id = src_id.and_then(|sid| clip_library.get_asset_id_for_source(sid));
            let asset_exists =
                asset_id.map_or(false, |aid| assets.animation_clips.contains_key(&aid));
            crate::log!(
                "[PlaybackDebug] entity {:?}: source_id={:?}, asset_id={:?}, asset_exists={}, time={:.3}, instances={}",
                entity, src_id, asset_id, asset_exists, animator.time, schedule.instances.len()
            );
        }

        let active_instances = build_active_instances(schedule, clip_library, animator);

        if should_log && active_instances.is_empty() {
            crate::log!(
                "[PlaybackDebug] entity {:?}: active_instances is EMPTY",
                entity
            );
        }

        if active_instances.is_empty() {
            continue;
        }

        infos.push(AnimatedEntityInfo {
            entity,
            active_instances,
            skeleton_id: skel_id,
            mesh_idx,
            animation_type: meta.animation_type.clone(),
            node_animation_scale: meta.node_animation_scale,
            looping: animator.looping,
        });
    }

    if should_log {
        crate::log!("[PlaybackDebug] total animated entities: {}", infos.len());
    }

    infos
}

fn build_active_instances(
    schedule: &ClipSchedule,
    clip_library: &ClipLibrary,
    animator: &Animator,
) -> Vec<ActiveInstanceInfo> {
    let active =
        super::clip_schedule_systems::clip_schedule_active_instances(schedule, animator.time);
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

            let weight = super::clip_schedule_systems::clip_schedule_effective_weight(
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

fn evaluate_entity_blend(info: &AnimatedEntityInfo, assets: &AssetStorage) -> Option<SkeletonPose> {
    let skeleton = assets.get_skeleton_by_skeleton_id(info.skeleton_id)?;
    let rest_pose = create_pose_from_rest(skeleton);

    let first_override = info
        .active_instances
        .iter()
        .find(|i| i.blend_mode == BlendMode::Override)?;

    let clip_asset = assets.animation_clips.get(&first_override.asset_id)?;

    let mut pose = rest_pose.clone();
    sample_clip_to_pose(
        &clip_asset.clip,
        first_override.local_time,
        skeleton,
        &mut pose,
        info.looping,
    );

    if first_override.weight < 1.0 {
        pose = blend_poses_override(&rest_pose, &pose, first_override.weight);
    }

    let mut prev_override = first_override;

    for inst in &info.active_instances {
        if std::ptr::eq(inst, first_override) {
            continue;
        }

        let Some(clip_asset) = assets.animation_clips.get(&inst.asset_id) else {
            continue;
        };

        match inst.blend_mode {
            BlendMode::Override => {
                let mut overlay = rest_pose.clone();
                sample_clip_to_pose(
                    &clip_asset.clip,
                    inst.local_time,
                    skeleton,
                    &mut overlay,
                    info.looping,
                );

                let crossfade = compute_crossfade_factor(
                    first_override.local_time + first_override.start_time,
                    prev_override.end_time,
                    inst.start_time,
                    prev_override.ease_out,
                    inst.blend_mode.default_ease_in(),
                );

                let blend_weight = crossfade * inst.weight;
                pose = blend_poses_override(&pose, &overlay, blend_weight);
                prev_override = inst;
            }
            BlendMode::Additive => {
                let mut additive_pose = rest_pose.clone();
                sample_clip_to_pose(
                    &clip_asset.clip,
                    inst.local_time,
                    skeleton,
                    &mut additive_pose,
                    info.looping,
                );

                pose = blend_poses_additive(&pose, &additive_pose, &rest_pose, inst.weight);
            }
        }
    }

    Some(pose)
}

fn apply_blended_animations(
    entities: &[AnimatedEntityInfo],
    world: &World,
    graphics: &mut GraphicsResources,
    nodes: &mut [NodeData],
    assets: &AssetStorage,
    dt: f32,
) -> (Vec<usize>, Option<(SkeletonId, Vec<Matrix4<f32>>)>) {
    let mut updated = Vec::new();
    let mut first_bone_transforms: Option<(SkeletonId, Vec<Matrix4<f32>>)> = None;

    let shared_constraints = find_shared_constraints(entities, world);

    let spring_result =
        compute_spring_bone_result(entities, world, assets, &shared_constraints, dt);

    for info in entities {
        let Some(skeleton) = assets.get_skeleton_by_skeleton_id(info.skeleton_id) else {
            continue;
        };

        let has_spring = spring_result
            .as_ref()
            .map_or(false, |(skel_id, _, _)| *skel_id == info.skeleton_id);

        let (globals, _pose) = if has_spring {
            let (_, ref cached_globals, ref cached_pose) = spring_result.as_ref().unwrap();

            if info.animation_type == AnimationType::Node {
                GraphicsResources::compute_node_global_transforms(nodes, skeleton, cached_pose);
            }

            (cached_globals.clone(), None)
        } else {
            let Some(mut pose) = evaluate_entity_blend(info, assets) else {
                continue;
            };

            if let Some(ref cs) = shared_constraints {
                apply_constraints(cs, skeleton, &mut pose);
            }

            if info.animation_type == AnimationType::Node {
                GraphicsResources::compute_node_global_transforms(nodes, skeleton, &pose);
            }

            let globals = compute_pose_global_transforms(skeleton, &pose);
            (globals, Some(pose))
        };

        if first_bone_transforms.is_none() {
            first_bone_transforms = Some((info.skeleton_id, globals.clone()));
        }

        let mesh_updated = match info.animation_type {
            AnimationType::Node => graphics.apply_node_animation_to_single_mesh(
                info.mesh_idx,
                nodes,
                info.node_animation_scale,
            ),
            _ => graphics.apply_skinning_to_single_mesh(info.mesh_idx, &globals, skeleton),
        };

        if mesh_updated && !updated.contains(&info.mesh_idx) {
            updated.push(info.mesh_idx);
        }
    }

    (updated, first_bone_transforms)
}

fn compute_spring_bone_result(
    entities: &[AnimatedEntityInfo],
    world: &World,
    assets: &AssetStorage,
    shared_constraints: &Option<ConstraintSet>,
    dt: f32,
) -> Option<(SkeletonId, Vec<Matrix4<f32>>, SkeletonPose)> {
    let info = entities
        .iter()
        .find(|e| world.has_component::<WithSpringBone>(e.entity))?;

    let setup = world.get_component::<SpringBoneSetup>(info.entity)?;
    let setup_clone = setup.clone();
    let mut pose = evaluate_entity_blend(info, assets)?;
    let skeleton = assets.get_skeleton_by_skeleton_id(info.skeleton_id)?;

    if let Some(ref cs) = shared_constraints {
        apply_constraints(cs, skeleton, &mut pose);
    }

    let mut globals = compute_pose_global_transforms(skeleton, &pose);

    apply_spring_bone_simulation(world, &setup_clone, skeleton, &mut globals, &mut pose, dt);

    Some((info.skeleton_id, globals, pose))
}

fn apply_spring_bone_simulation(
    world: &World,
    setup: &SpringBoneSetup,
    skeleton: &crate::animation::Skeleton,
    globals: &mut [Matrix4<f32>],
    pose: &mut SkeletonPose,
    dt: f32,
) {
    use super::spring_bone_systems::{
        collect_affected_bone_ids, spring_bone_initialize, spring_bone_update,
        spring_bone_write_back_to_pose,
    };

    if let Some(mut sb_state) = world.get_resource_mut::<SpringBoneState>() {
        match sb_state.mode {
            SpringBoneMode::Realtime => {
                if !sb_state.initialized {
                    *sb_state = spring_bone_initialize(setup, skeleton, globals);
                }

                spring_bone_update(setup, &mut sb_state, skeleton, globals, pose, dt);

                let affected_ids = collect_affected_bone_ids(setup);
                spring_bone_write_back_to_pose(skeleton, globals, pose, &affected_ids);
            }
            SpringBoneMode::Baked | SpringBoneMode::BakedOverride => {}
        }
    }
}

fn find_shared_constraints(
    entities: &[AnimatedEntityInfo],
    world: &World,
) -> Option<ConstraintSet> {
    entities.iter().find_map(|info| {
        world
            .get_component::<ConstraintSet>(info.entity)
            .map(|cs| cs.clone())
    })
}

fn merge_updated_indices(morph: Vec<usize>, anim: Vec<usize>) -> Vec<usize> {
    let mut all = morph;
    for idx in anim {
        if !all.contains(&idx) {
            all.push(idx);
        }
    }
    all
}

pub unsafe fn playback_upload_animations(
    backend: &mut dyn RenderBackend,
    updated_meshes: &[usize],
) -> Result<()> {
    for &mesh_idx in updated_meshes {
        backend.upload_mesh_vertices(mesh_idx)?;
    }

    if !updated_meshes.is_empty() {
        backend.update_acceleration_structure(updated_meshes)?;
        backend.rebuild_tlas()?;
    }

    Ok(())
}

pub fn playback_apply_morph_animation(
    graphics: &mut GraphicsResources,
    morph_animation: &MorphAnimationSystem,
    time: f32,
) -> Vec<usize> {
    if morph_animation.is_empty() {
        return Vec::new();
    }

    let animation_index = morph_animation.get_animation_index(time);
    let mesh_count = morph_animation.targets.len().min(graphics.meshes.len());
    let mut updated_mesh_indices = Vec::new();

    for mesh_idx in 0..mesh_count {
        let morph_targets = &morph_animation.targets[mesh_idx];
        if morph_targets.is_empty() {
            continue;
        }

        let base_vertices = &morph_animation.base_vertices[mesh_idx];
        let vertices = &mut graphics.meshes[mesh_idx].vertex_data.vertices;

        for (i, v) in vertices.iter_mut().enumerate() {
            if i < base_vertices.len() {
                let base = base_vertices[i];
                v.pos.x = base[0];
                v.pos.y = base[1];
                v.pos.z = base[2];
            }
        }

        let morph_anim = &morph_animation.animations[animation_index];
        let scale_factor = morph_animation.scale_factor;
        for (weight_idx, &weight) in morph_anim.weights.iter().enumerate() {
            if weight_idx >= morph_targets.len() {
                break;
            }
            let morph_target = &morph_targets[weight_idx];
            for (j, delta_pos) in morph_target.positions.iter().enumerate() {
                if j < vertices.len() {
                    vertices[j].pos.x += delta_pos[0] * weight * scale_factor;
                    vertices[j].pos.y += delta_pos[1] * weight * scale_factor;
                    vertices[j].pos.z += delta_pos[2] * weight * scale_factor;
                }
            }
        }

        updated_mesh_indices.push(mesh_idx);
    }

    updated_mesh_indices
}
