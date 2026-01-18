use anyhow::Result;
use cgmath::{InnerSpace, Matrix4, SquareMatrix, Vector3};

use crate::asset::AssetStorage;
use crate::ecs::components::{CameraState, RenderContext, Renderable, Updatable, UpdateContext};
use crate::ecs::world::{Entity, GlobalTransform, World};
use crate::scene::graphics_resource::{ObjectDescriptorSet, ObjectUBO};
use crate::vulkanr::device::RRDevice;
use crate::vulkanr::vulkan::*;

pub fn update_all(updatables: &mut [&mut dyn Updatable], ctx: &UpdateContext) {
    for obj in updatables {
        obj.update(ctx);
    }
}

pub fn transform_propagation_system(world: &mut World) {
    let root_entities = world.get_root_entities();

    fn propagate(world: &mut World, entity: Entity, parent_global: Matrix4<f32>) {
        let local_matrix = world
            .transforms
            .get(&entity)
            .map(|t| t.to_matrix())
            .unwrap_or_else(Matrix4::identity);

        let global_matrix = parent_global * local_matrix;

        if let Some(gt) = world.global_transforms.get_mut(&entity) {
            gt.0 = global_matrix;
        }

        let child_entities: Vec<Entity> = world
            .children
            .get(&entity)
            .map(|c| c.0.clone())
            .unwrap_or_default();

        for child in child_entities {
            propagate(world, child, global_matrix);
        }
    }

    for root in root_entities {
        propagate(world, root, Matrix4::identity());
    }
}

pub fn animation_time_system(world: &mut World, delta_time: f32, assets: &AssetStorage) {
    let animated_entities = world.query_animated();

    for entity in animated_entities {
        let Some(state) = world.animation_states.get_mut(&entity) else {
            continue;
        };

        if !state.playing {
            continue;
        }

        let Some(clip_id) = state.current_clip_id else {
            continue;
        };

        let duration = assets
            .animation_clips
            .values()
            .find(|c| c.clip_id == clip_id)
            .map(|c| c.clip.duration)
            .unwrap_or(1.0);

        state.time += delta_time * state.speed;

        if state.looping && duration > 0.0 {
            state.time = state.time % duration;
        } else if state.time > duration {
            state.time = duration;
            state.playing = false;
        }
    }
}

pub fn skeleton_animation_system(world: &mut World, assets: &mut AssetStorage) {
    let animated_entities = world.query_animated();

    struct AnimationData {
        skeleton_asset_id: crate::asset::AssetId,
        clip: crate::animation::AnimationClip,
        time: f32,
    }

    let mut animations_to_apply = Vec::new();

    for entity in animated_entities {
        let Some(state) = world.animation_states.get(&entity) else {
            continue;
        };
        let Some(skeleton_ref) = world.skeleton_refs.get(&entity) else {
            continue;
        };

        let Some(clip_id) = state.current_clip_id else {
            continue;
        };

        let Some(clip_asset) = assets
            .animation_clips
            .values()
            .find(|c| c.clip_id == clip_id)
        else {
            continue;
        };

        animations_to_apply.push(AnimationData {
            skeleton_asset_id: skeleton_ref.0,
            clip: clip_asset.clip.clone(),
            time: state.time,
        });
    }

    for anim_data in animations_to_apply {
        if let Some(skeleton_asset) = assets.get_skeleton_mut(anim_data.skeleton_asset_id) {
            anim_data
                .clip
                .sample(anim_data.time, &mut skeleton_asset.skeleton);
        }
    }
}

pub fn billboard_system(world: &mut World, camera: &CameraState) {
    let billboards = world.query_billboards();

    for entity in billboards {
        let Some(behavior) = world.billboard_behaviors.get(&entity) else {
            continue;
        };

        if !behavior.always_face_camera {
            continue;
        }

        let Some(transform) = world.transforms.get_mut(&entity) else {
            continue;
        };

        let position = transform.translation;
        let to_camera = camera.position - position;

        if to_camera.magnitude() > 0.001 {
            let forward = to_camera.normalize();
            let up = Vector3::new(0.0, 1.0, 0.0);
            let right = up.cross(forward).normalize();
            let adjusted_up = forward.cross(right);

            transform.rotation =
                cgmath::Quaternion::from(cgmath::Matrix3::from_cols(right, adjusted_up, forward));
        }
    }
}

pub unsafe fn update_object_ubos(
    renderables: &[&dyn Renderable],
    ctx: &RenderContext,
    objects: &ObjectDescriptorSet,
    rrdevice: &RRDevice,
) -> Result<()> {
    for obj in renderables {
        let ubo = ObjectUBO {
            model: obj.model_matrix(ctx),
        };
        objects.update(rrdevice, ctx.image_index, obj.object_index(), &ubo)?;
    }
    Ok(())
}

pub unsafe fn render_objects(
    renderables: &[&dyn Renderable],
    command_buffer: vk::CommandBuffer,
    image_index: usize,
    frame_set: vk::DescriptorSet,
    objects: &ObjectDescriptorSet,
    rrdevice: &RRDevice,
) {
    for obj in renderables {
        let vertex_buffer = obj.vertex_buffer();
        let index_buffer = obj.index_buffer();

        if vertex_buffer == vk::Buffer::null() || index_buffer == vk::Buffer::null() {
            continue;
        }

        rrdevice.device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            obj.pipeline().pipeline,
        );

        rrdevice.device.cmd_set_line_width(command_buffer, 1.0);

        rrdevice
            .device
            .cmd_bind_vertex_buffers(command_buffer, 0, &[vertex_buffer], &[0]);

        rrdevice.device.cmd_bind_index_buffer(
            command_buffer,
            index_buffer,
            0,
            vk::IndexType::UINT32,
        );

        rrdevice.device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            obj.pipeline().pipeline_layout,
            0,
            &[frame_set],
            &[],
        );

        let object_set_idx = objects.get_set_index(image_index, obj.object_index());
        let object_set = objects.sets[object_set_idx];
        rrdevice.device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            obj.pipeline().pipeline_layout,
            2,
            &[object_set],
            &[],
        );

        rrdevice
            .device
            .cmd_draw_indexed(command_buffer, obj.index_count(), 1, 0, 0, 0);
    }
}
