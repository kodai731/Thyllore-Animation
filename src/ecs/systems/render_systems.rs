use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::ecs::component::RenderData;
use crate::ecs::resource::PipelineManager;
use crate::render::ObjectUBO;
use crate::scene::graphics_resource::ObjectDescriptorSet;
use crate::vulkanr::device::RRDevice;
use crate::vulkanr::resource::GpuBufferRegistry;

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

pub unsafe fn render_scene_objects_system(
    render_data: &[&RenderData],
    command_buffer: vk::CommandBuffer,
    image_index: usize,
    frame_set: vk::DescriptorSet,
    objects: &ObjectDescriptorSet,
    rrdevice: &RRDevice,
    pipeline_manager: &PipelineManager,
    buffer_registry: &GpuBufferRegistry,
) {
    for data in render_data {
        let vertex_buffer = match buffer_registry.get_vertex_buffer(data.vertex_buffer_handle) {
            Some(b) => b,
            None => continue,
        };
        let index_buffer = match buffer_registry.get_index_buffer(data.index_buffer_handle) {
            Some(b) => b,
            None => continue,
        };

        let pipeline_id = match data.pipeline_id {
            Some(id) => id,
            None => continue,
        };
        let pipeline = match pipeline_manager.get(pipeline_id) {
            Some(p) => p,
            None => continue,
        };

        rrdevice.device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            pipeline.pipeline,
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
            pipeline.pipeline_layout,
            0,
            &[frame_set],
            &[],
        );

        let object_set_idx = objects.get_set_index(image_index, data.object_index);
        let object_set = objects.sets[object_set_idx];
        rrdevice.device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            pipeline.pipeline_layout,
            2,
            &[object_set],
            &[],
        );

        rrdevice
            .device
            .cmd_draw_indexed(command_buffer, data.index_count, 1, 0, 0, 0);
    }
}

pub unsafe fn render_system(
    render_data: &[&RenderData],
    command_buffer: vk::CommandBuffer,
    image_index: usize,
    frame_set: vk::DescriptorSet,
    objects: &ObjectDescriptorSet,
    rrdevice: &RRDevice,
    pipeline_manager: &PipelineManager,
    buffer_registry: &GpuBufferRegistry,
) {
    for data in render_data {
        let vertex_buffer = match buffer_registry.get_vertex_buffer(data.vertex_buffer_handle) {
            Some(b) => b,
            None => continue,
        };
        let index_buffer = match buffer_registry.get_index_buffer(data.index_buffer_handle) {
            Some(b) => b,
            None => continue,
        };

        let pipeline_id = match data.pipeline_id {
            Some(id) => id,
            None => continue,
        };
        let pipeline = match pipeline_manager.get(pipeline_id) {
            Some(p) => p,
            None => continue,
        };

        rrdevice.device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            pipeline.pipeline,
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
            pipeline.pipeline_layout,
            0,
            &[frame_set],
            &[],
        );

        let object_set_idx = objects.get_set_index(image_index, data.object_index);
        let object_set = objects.sets[object_set_idx];
        rrdevice.device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            pipeline.pipeline_layout,
            2,
            &[object_set],
            &[],
        );

        rrdevice
            .device
            .cmd_draw_indexed(command_buffer, data.index_count, 1, 0, 0, 0);
    }
}
