use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::ecs::components::RenderData;
use crate::scene::graphics_resource::{ObjectDescriptorSet, ObjectUBO};
use crate::vulkanr::device::RRDevice;

pub unsafe fn update_object_ubo_system(
    render_data: &[&RenderData],
    image_index: usize,
    objects: &ObjectDescriptorSet,
    rrdevice: &RRDevice,
) -> Result<()> {
    for data in render_data {
        let ubo = ObjectUBO {
            model: data.model_matrix,
        };
        objects.update(rrdevice, image_index, data.object_index, &ubo)?;
    }
    Ok(())
}

pub unsafe fn render_system(
    render_data: &[&RenderData],
    command_buffer: vk::CommandBuffer,
    image_index: usize,
    frame_set: vk::DescriptorSet,
    objects: &ObjectDescriptorSet,
    rrdevice: &RRDevice,
) {
    for data in render_data {
        if data.vertex_buffer == vk::Buffer::null() || data.index_buffer == vk::Buffer::null() {
            continue;
        }

        rrdevice.device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            data.pipeline.pipeline,
        );

        rrdevice.device.cmd_set_line_width(command_buffer, 1.0);

        rrdevice
            .device
            .cmd_bind_vertex_buffers(command_buffer, 0, &[data.vertex_buffer], &[0]);

        rrdevice.device.cmd_bind_index_buffer(
            command_buffer,
            data.index_buffer,
            0,
            vk::IndexType::UINT32,
        );

        rrdevice.device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            data.pipeline.pipeline_layout,
            0,
            &[frame_set],
            &[],
        );

        let object_set_idx = objects.get_set_index(image_index, data.object_index);
        let object_set = objects.sets[object_set_idx];
        rrdevice.device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            data.pipeline.pipeline_layout,
            2,
            &[object_set],
            &[],
        );

        rrdevice
            .device
            .cmd_draw_indexed(command_buffer, data.index_count, 1, 0, 0, 0);
    }
}
