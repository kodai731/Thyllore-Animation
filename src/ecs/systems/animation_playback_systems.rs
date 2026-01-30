use anyhow::Result;

use crate::animation::{AnimationClipId, MorphAnimationSystem};
use crate::ecs::resource::{AnimationPlayback, AnimationType, ClipLibrary, HierarchyState, ModelState};
use crate::ecs::world::{Animator, World};
use crate::render::RenderBackend;
use crate::app::graphics_resource::{GraphicsResources, NodeData};

pub fn playback_play(playback: &mut AnimationPlayback, clip_id: AnimationClipId) {
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
    clip_library: &mut ClipLibrary,
    model_state: &ModelState,
    _time: f32,
    playback: &mut AnimationPlayback,
) -> Vec<usize> {
    let current_time = playback.time;

    let morph_updated = if !clip_library.morph_animation.is_empty() {
        playback_apply_morph_animation(graphics, &clip_library.morph_animation, current_time)
    } else {
        Vec::new()
    };

    if clip_library.animation.clips.is_empty() {
        return morph_updated;
    }

    let skeleton_id = graphics.meshes.first().and_then(|m| m.skeleton_id);
    if let (Some(skel_id), Some(clip_id)) = (skeleton_id, playback.current_clip_id) {
        clip_library.animation.apply_to_skeleton(skel_id, clip_id, playback.time, playback.looping);
    }

    let has_node_animation = model_state.animation_type == AnimationType::Node;

    let anim_updated = if has_node_animation {
        graphics.prepare_node_animation(
            nodes,
            &clip_library.animation,
            model_state.node_animation_scale,
        )
    } else {
        graphics.prepare_skinned_vertices(&clip_library.animation)
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
    clip_library: &mut ClipLibrary,
    model_state: &ModelState,
    hierarchy_state: &HierarchyState,
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
        playback_apply_morph_animation(graphics, &clip_library.morph_animation, current_time)
    } else {
        Vec::new()
    };

    if clip_library.animation.clips.is_empty() {
        return morph_updated;
    }

    let skeleton_id = graphics.meshes.first().and_then(|m| m.skeleton_id);
    if let (Some(skel_id), Some(clip_id)) = (skeleton_id, current_clip_id) {
        clip_library
            .animation
            .apply_to_skeleton(skel_id, clip_id, current_time, looping);
    }

    let anim_updated = match model_state.animation_type {
        AnimationType::Node => graphics.prepare_node_animation(
            nodes,
            &clip_library.animation,
            model_state.node_animation_scale,
        ),
        _ => graphics.prepare_skinned_vertices(&clip_library.animation),
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
