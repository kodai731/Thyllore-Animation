use anyhow::Result;

use crate::scene::components::{Renderable, RenderContext, Updatable, UpdateContext};
use crate::scene::render_resource::{ObjectDescriptorSet, ObjectUBO};
use crate::vulkanr::device::RRDevice;
use crate::vulkanr::vulkan::*;

pub fn update_all(updatables: &mut [&mut dyn Updatable], ctx: &UpdateContext) {
    for obj in updatables {
        obj.update(ctx);
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

        rrdevice.device.cmd_draw_indexed(
            command_buffer,
            obj.index_count(),
            1,
            0,
            0,
            0,
        );
    }
}
