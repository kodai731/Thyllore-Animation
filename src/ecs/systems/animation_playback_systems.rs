use std::collections::HashMap;

use anyhow::Result;

use crate::animation::{AnimationClipId, MorphAnimationSystem, SkeletonId};
use crate::asset::AssetStorage;
use crate::ecs::component::{AnimationMeta, ClipSchedule};
use crate::ecs::resource::{AnimationType, ClipLibrary};
use crate::ecs::world::{Animator, MeshRef, World};
use crate::ecs::{
    compute_pose_global_transforms, create_pose_from_rest, sample_clip_to_pose,
};
use crate::render::RenderBackend;
use crate::app::graphics_resource::{GraphicsResources, NodeData};

struct AnimatedEntityInfo {
    time: f32,
    looping: bool,
    clip_id: AnimationClipId,
    skeleton_id: SkeletonId,
    mesh_idx: usize,
    animation_type: AnimationType,
    node_animation_scale: f32,
}

pub fn evaluate_all_animators(
    world: &World,
    graphics: &mut GraphicsResources,
    nodes: &mut [NodeData],
    clip_library: &ClipLibrary,
    assets: &AssetStorage,
) -> Vec<usize> {
    let entity_infos = collect_animated_entities(
        world, graphics, clip_library, assets,
    );

    let first_time = entity_infos.first().map(|e| e.time).unwrap_or(0.0);

    let morph_updated = if !clip_library.morph_animation.is_empty() {
        playback_apply_morph_animation(
            graphics,
            &clip_library.morph_animation,
            first_time,
        )
    } else {
        Vec::new()
    };

    if entity_infos.is_empty() {
        return morph_updated;
    }

    let grouped = group_by_pose(&entity_infos);
    let anim_updated =
        apply_grouped_animations(grouped, graphics, nodes, assets);

    merge_updated_indices(morph_updated, anim_updated)
}

fn collect_animated_entities(
    world: &World,
    graphics: &GraphicsResources,
    clip_library: &ClipLibrary,
    assets: &AssetStorage,
) -> Vec<AnimatedEntityInfo> {
    let mut infos = Vec::new();

    for (entity, animator) in world.iter_components::<Animator>() {
        let Some(schedule) =
            world.get_component::<ClipSchedule>(entity)
        else {
            continue;
        };
        let Some(meta) =
            world.get_component::<AnimationMeta>(entity)
        else {
            continue;
        };
        let Some(mesh_ref) =
            world.get_component::<MeshRef>(entity)
        else {
            continue;
        };

        let Some(mesh_asset) = assets.get_mesh(mesh_ref.mesh_asset_id)
        else {
            continue;
        };

        let mesh_idx = mesh_asset.graphics_mesh_index;
        if mesh_idx >= graphics.meshes.len() {
            continue;
        }

        let skeleton_id = mesh_asset.skeleton_id.or_else(|| {
            graphics.meshes.get(mesh_idx).and_then(|m| m.skeleton_id)
        });
        let Some(skel_id) = skeleton_id else {
            continue;
        };

        let resolved_clip_id = schedule
            .first_instance()
            .and_then(|inst| {
                clip_library.get_anim_clip_id_for_source(inst.source_id)
            });
        let Some(clip_id) = resolved_clip_id else {
            continue;
        };

        infos.push(AnimatedEntityInfo {
            time: animator.time,
            looping: animator.looping,
            clip_id,
            skeleton_id: skel_id,
            mesh_idx,
            animation_type: meta.animation_type.clone(),
            node_animation_scale: meta.node_animation_scale,
        });
    }

    infos
}

type PoseKey = (SkeletonId, AnimationClipId, u32, bool);

fn make_pose_key(info: &AnimatedEntityInfo) -> PoseKey {
    let time_bits = info.time.to_bits();
    (info.skeleton_id, info.clip_id, time_bits, info.looping)
}

fn group_by_pose(
    infos: &[AnimatedEntityInfo],
) -> HashMap<PoseKey, Vec<&AnimatedEntityInfo>> {
    let mut groups: HashMap<PoseKey, Vec<&AnimatedEntityInfo>> =
        HashMap::new();

    for info in infos {
        let key = make_pose_key(info);
        groups.entry(key).or_default().push(info);
    }

    groups
}

fn apply_grouped_animations(
    groups: HashMap<PoseKey, Vec<&AnimatedEntityInfo>>,
    graphics: &mut GraphicsResources,
    nodes: &mut [NodeData],
    assets: &AssetStorage,
) -> Vec<usize> {
    let mut updated = Vec::new();

    for (_key, group) in &groups {
        let first = group[0];

        let skeleton =
            assets.get_skeleton_by_skeleton_id(first.skeleton_id);
        let Some(skeleton) = skeleton else {
            continue;
        };

        let clip_library_clip = assets
            .animation_clips
            .values()
            .find(|c| c.clip_id == first.clip_id);
        let Some(clip_asset) = clip_library_clip else {
            continue;
        };

        let mut pose = create_pose_from_rest(skeleton);
        sample_clip_to_pose(
            &clip_asset.clip,
            first.time,
            skeleton,
            &mut pose,
            first.looping,
        );

        let has_node_anim = group
            .iter()
            .any(|e| e.animation_type == AnimationType::Node);

        if has_node_anim {
            GraphicsResources::compute_node_global_transforms(
                nodes, skeleton, &pose,
            );
        }

        let globals = compute_pose_global_transforms(skeleton, &pose);

        for info in group {
            let mesh_updated = match info.animation_type {
                AnimationType::Node => {
                    graphics.apply_node_animation_to_single_mesh(
                        info.mesh_idx,
                        nodes,
                        info.node_animation_scale,
                    )
                }
                _ => graphics.apply_skinning_to_single_mesh(
                    info.mesh_idx,
                    &globals,
                    skeleton,
                ),
            };

            if mesh_updated && !updated.contains(&info.mesh_idx) {
                updated.push(info.mesh_idx);
            }
        }
    }

    updated
}

fn merge_updated_indices(
    morph: Vec<usize>,
    anim: Vec<usize>,
) -> Vec<usize> {
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
    let mesh_count =
        morph_animation.targets.len().min(graphics.meshes.len());
    let mut updated_mesh_indices = Vec::new();

    for mesh_idx in 0..mesh_count {
        let morph_targets = &morph_animation.targets[mesh_idx];
        if morph_targets.is_empty() {
            continue;
        }

        let base_vertices = &morph_animation.base_vertices[mesh_idx];
        let vertices =
            &mut graphics.meshes[mesh_idx].vertex_data.vertices;

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
        for (weight_idx, &weight) in
            morph_anim.weights.iter().enumerate()
        {
            if weight_idx >= morph_targets.len() {
                break;
            }
            let morph_target = &morph_targets[weight_idx];
            for (j, delta_pos) in
                morph_target.positions.iter().enumerate()
            {
                if j < vertices.len() {
                    vertices[j].pos.x +=
                        delta_pos[0] * weight * scale_factor;
                    vertices[j].pos.y +=
                        delta_pos[1] * weight * scale_factor;
                    vertices[j].pos.z +=
                        delta_pos[2] * weight * scale_factor;
                }
            }
        }

        updated_mesh_indices.push(mesh_idx);
    }

    updated_mesh_indices
}
