use anyhow::Result;

use crate::animation::{AnimationClipId, MorphAnimationSystem};
use crate::asset::AssetStorage;
use crate::ecs::resource::{
    AnimationPlayback, AnimationType, ClipLibrary, HierarchyState, ModelState,
};
use crate::ecs::world::{Animator, World};
use crate::ecs::{create_pose_from_rest, sample_clip_to_pose, compute_pose_global_transforms};
use crate::render::RenderBackend;
use crate::app::graphics_resource::{GraphicsResources, NodeData};

pub fn playback_play(
    playback: &mut AnimationPlayback,
    clip_id: AnimationClipId,
) {
    playback.current_clip_id = Some(clip_id);
    playback.playing = true;
    playback.time = 0.0;
}

pub fn playback_stop(playback: &mut AnimationPlayback) {
    playback.playing = false;
    playback.current_clip_id = None;
}

pub fn playback_pause(playback: &mut AnimationPlayback) {
    playback.playing = false;
}

pub fn playback_resume(playback: &mut AnimationPlayback) {
    playback.playing = true;
}

pub fn playback_prepare_animations(
    graphics: &mut GraphicsResources,
    nodes: &mut [NodeData],
    clip_library: &ClipLibrary,
    model_state: &ModelState,
    _time: f32,
    playback: &AnimationPlayback,
    assets: &AssetStorage,
) -> Vec<usize> {
    let current_time = playback.time;

    let morph_updated = if !clip_library.morph_animation.is_empty() {
        playback_apply_morph_animation(
            graphics,
            &clip_library.morph_animation,
            current_time,
        )
    } else {
        Vec::new()
    };

    if clip_library.animation.clips.is_empty() {
        return morph_updated;
    }

    let skeleton_id = graphics.meshes.first().and_then(|m| m.skeleton_id);
    let Some(skel_id) = skeleton_id else {
        return morph_updated;
    };
    let Some(clip_id) = playback.current_clip_id else {
        return morph_updated;
    };

    let skeleton = assets.get_skeleton_by_skeleton_id(skel_id);
    let clip = clip_library.animation.get_clip(clip_id);
    let (Some(skeleton), Some(clip)) = (skeleton, clip) else {
        return morph_updated;
    };

    let mut pose = create_pose_from_rest(skeleton);
    sample_clip_to_pose(clip, current_time, skeleton, &mut pose, playback.looping);

    let anim_updated = match model_state.animation_type {
        AnimationType::Node => {
            graphics.prepare_node_animation(
                nodes,
                skeleton,
                &pose,
                model_state.node_animation_scale,
            )
        }
        _ => {
            let globals = compute_pose_global_transforms(skeleton, &pose);
            graphics.prepare_skinned_vertices(&globals, skeleton)
        }
    };

    let mut all_updated = morph_updated;
    for mesh_id in anim_updated {
        if !all_updated.contains(&mesh_id) {
            all_updated.push(mesh_id);
        }
    }
    all_updated
}

pub fn evaluate_animators(
    world: &World,
    graphics: &mut GraphicsResources,
    nodes: &mut [NodeData],
    clip_library: &ClipLibrary,
    model_state: &ModelState,
    hierarchy_state: &HierarchyState,
    assets: &AssetStorage,
) -> Vec<usize> {
    let animator = hierarchy_state
        .selected_entity
        .and_then(|entity| world.get_component::<Animator>(entity))
        .or_else(|| {
            world
                .iter_animated_entities()
                .next()
                .map(|(e, _)| e)
                .and_then(|e| world.get_component::<Animator>(e))
        });

    let current_time = animator.map(|a| a.time).unwrap_or(0.0);
    let current_clip_id = animator.and_then(|a| a.current_clip_id);
    let looping = animator.map(|a| a.looping).unwrap_or(true);

    let morph_updated = if !clip_library.morph_animation.is_empty() {
        playback_apply_morph_animation(
            graphics,
            &clip_library.morph_animation,
            current_time,
        )
    } else {
        Vec::new()
    };

    if clip_library.animation.clips.is_empty() {
        return morph_updated;
    }

    let skeleton_id = graphics.meshes.first().and_then(|m| m.skeleton_id);
    let Some(skel_id) = skeleton_id else {
        return morph_updated;
    };
    let Some(clip_id) = current_clip_id else {
        return morph_updated;
    };

    let skeleton = assets.get_skeleton_by_skeleton_id(skel_id);
    let clip = clip_library.animation.get_clip(clip_id);
    let (Some(skeleton), Some(clip)) = (skeleton, clip) else {
        return morph_updated;
    };

    let mut pose = create_pose_from_rest(skeleton);
    sample_clip_to_pose(clip, current_time, skeleton, &mut pose, looping);

    let anim_updated = match model_state.animation_type {
        AnimationType::Node => {
            graphics.prepare_node_animation(
                nodes,
                skeleton,
                &pose,
                model_state.node_animation_scale,
            )
        }
        _ => {
            let globals = compute_pose_global_transforms(skeleton, &pose);
            graphics.prepare_skinned_vertices(&globals, skeleton)
        }
    };

    let mut all_updated = morph_updated;
    for mesh_id in anim_updated {
        if !all_updated.contains(&mesh_id) {
            all_updated.push(mesh_id);
        }
    }
    all_updated
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
