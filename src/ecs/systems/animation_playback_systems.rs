use std::ffi::c_void;
use std::mem::size_of;

use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::animation::AnimationClipId;
use crate::ecs::resource::AnimationPlayback;
use crate::scene::graphics_resource::GraphicsResources;
use crate::vulkanr::command::RRCommandPool;
use crate::vulkanr::core::device::RRDevice;
use crate::vulkanr::data::Vertex;
use crate::vulkanr::raytracing::acceleration::RRAccelerationStructure;
use crate::vulkanr::vulkan::Instance;

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

pub unsafe fn playback_update_animations(
    graphics: &mut GraphicsResources,
    time: f32,
    playback: &mut AnimationPlayback,
    instance: &Instance,
    rrdevice: &RRDevice,
    command_pool: &RRCommandPool,
    acceleration_structure: &mut Option<RRAccelerationStructure>,
) -> Result<()> {
    if !graphics.morph_animation.is_empty() {
        playback_update_morph_animation(
            graphics,
            time,
            instance,
            rrdevice,
            command_pool,
            acceleration_structure,
        )?;
    }

    if graphics.animation.clips.is_empty() {
        return Ok(());
    }

    if !playback.playing {
        static mut LOGGED_PAUSED: bool = false;
        if !LOGGED_PAUSED {
            crate::log!("Animation is paused (animation_playing=false)");
            LOGGED_PAUSED = true;
        }
        return Ok(());
    }

    if let Some(clip_id) = playback.current_clip_id {
        if let Some(clip) = graphics.animation.clips.iter().find(|c| c.id == clip_id) {
            let duration = clip.duration;
            if duration > 0.0 {
                let prev_time = playback.time;
                playback.time = time % duration;

                static mut FRAME_COUNT: u32 = 0;
                FRAME_COUNT += 1;
                if FRAME_COUNT % 60 == 0 {
                    crate::log!(
                        "Animation update: time={:.4}/{:.4}s (elapsed={:.4}, prev={:.4})",
                        playback.time,
                        duration,
                        time,
                        prev_time
                    );
                }
            }
        }
    }

    let skeleton_id = graphics.meshes.first().and_then(|m| m.skeleton_id);
    if let Some(skel_id) = skeleton_id {
        graphics.animation.apply_to_skeleton(skel_id, playback);
    }

    let model_path = &playback.model_path;
    let is_gltf = model_path.ends_with(".glb") || model_path.ends_with(".gltf");
    let is_fbx = model_path.ends_with(".fbx");
    let has_node_animation = (is_gltf || is_fbx) && !graphics.has_skinned_meshes;

    if has_node_animation {
        graphics.update_node_animation(instance, rrdevice, command_pool, acceleration_structure)?;
    } else {
        graphics.update_skinned_vertex_buffers(instance, rrdevice, command_pool)?;
    }

    graphics.update_acceleration_structure(
        instance,
        rrdevice,
        command_pool,
        acceleration_structure,
    )?;

    Ok(())
}

pub fn playback_apply_morph_animation(graphics: &mut GraphicsResources, time: f32) -> Vec<usize> {
    if graphics.morph_animation.is_empty() {
        return Vec::new();
    }

    let animation_index = graphics.morph_animation.get_animation_index(time);
    let mesh_count = graphics
        .morph_animation
        .targets
        .len()
        .min(graphics.meshes.len());
    let mut updated_mesh_indices = Vec::new();

    for mesh_idx in 0..mesh_count {
        let morph_targets = &graphics.morph_animation.targets[mesh_idx];
        if morph_targets.is_empty() {
            continue;
        }

        let base_vertices = &graphics.morph_animation.base_vertices[mesh_idx];
        let vertices = &mut graphics.meshes[mesh_idx].vertex_data.vertices;

        for (i, v) in vertices.iter_mut().enumerate() {
            if i < base_vertices.len() {
                let base = base_vertices[i];
                v.pos.x = base[0];
                v.pos.y = base[1];
                v.pos.z = base[2];
            }
        }

        let morph_animation = &graphics.morph_animation.animations[animation_index];
        let scale_factor = graphics.morph_animation.scale_factor;
        for (weight_idx, &weight) in morph_animation.weights.iter().enumerate() {
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

pub unsafe fn playback_update_morph_animation(
    graphics: &mut GraphicsResources,
    time: f32,
    instance: &Instance,
    rrdevice: &RRDevice,
    command_pool: &RRCommandPool,
    acceleration_structure: &mut Option<RRAccelerationStructure>,
) -> Result<()> {
    let updated_indices = playback_apply_morph_animation(graphics, time);
    if updated_indices.is_empty() {
        return Ok(());
    }

    for mesh_idx in &updated_indices {
        let mesh = &mut graphics.meshes[*mesh_idx];
        let vertices = &mesh.vertex_data.vertices;

        mesh.vertex_buffer.update(
            instance,
            rrdevice,
            command_pool,
            (size_of::<Vertex>() * vertices.len()) as vk::DeviceSize,
            vertices.as_ptr() as *const c_void,
            vertices.len(),
        )?;

        if let Some(ref mut accel_struct) = acceleration_structure {
            if *mesh_idx < accel_struct.blas_list.len() {
                let blas = &accel_struct.blas_list[*mesh_idx];
                RRAccelerationStructure::update_blas(
                    instance,
                    rrdevice,
                    command_pool,
                    blas,
                    &mesh.vertex_buffer.buffer,
                    mesh.vertex_data.vertices.len() as u32,
                    size_of::<Vertex>() as u32,
                    &mesh.index_buffer.buffer,
                    mesh.vertex_data.indices.len() as u32,
                )?;
            }
        }
    }

    if let Some(ref mut accel_struct) = acceleration_structure {
        let tlas = &accel_struct.tlas;
        RRAccelerationStructure::update_tlas(
            instance,
            rrdevice,
            command_pool,
            tlas,
            &accel_struct.blas_list,
        )?;
    }

    Ok(())
}
