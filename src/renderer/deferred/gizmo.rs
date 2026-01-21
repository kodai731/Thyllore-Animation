use vulkanalia::prelude::v1_0::*;

use crate::ecs::component::{GizmoRayToModel, GizmoVerticalLines};
use crate::scene::graphics_resource::GraphicsResources;
use crate::vulkanr::core::Device;
use crate::vulkanr::pipeline::RRPipeline;
use crate::vulkanr::resource::GpuBufferRegistry;

pub unsafe fn gizmo_draw_ray_with_pipeline(
    ray: &GizmoRayToModel,
    registry: &GpuBufferRegistry,
    device: &Device,
    command_buffer: vk::CommandBuffer,
    pipeline: &RRPipeline,
    graphics_resources: &GraphicsResources,
    object_index: usize,
    image_index: usize,
) {
    let vertex_buffer = match registry.get_vertex_buffer(ray.vertex_buffer_handle) {
        Some(vb) => vb,
        None => return,
    };
    let index_buffer = match registry.get_index_buffer(ray.index_buffer_handle) {
        Some(ib) => ib,
        None => return,
    };

    device.cmd_bind_pipeline(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        pipeline.pipeline,
    );

    device.cmd_set_line_width(command_buffer, 1.0);
    device.cmd_bind_vertex_buffers(command_buffer, 0, &[vertex_buffer], &[0]);
    device.cmd_bind_index_buffer(command_buffer, index_buffer, 0, vk::IndexType::UINT32);

    let frame_set = graphics_resources.frame_set.sets[image_index];
    device.cmd_bind_descriptor_sets(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        pipeline.pipeline_layout,
        0,
        &[frame_set],
        &[],
    );

    let object_set_idx = graphics_resources
        .objects
        .get_set_index(image_index, object_index);
    let object_set = graphics_resources.objects.sets[object_set_idx];
    device.cmd_bind_descriptor_sets(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        pipeline.pipeline_layout,
        2,
        &[object_set],
        &[],
    );

    device.cmd_draw_indexed(command_buffer, ray.indices.len() as u32, 1, 0, 0, 0);
}

pub unsafe fn gizmo_draw_vertical_lines_with_pipeline(
    lines: &GizmoVerticalLines,
    registry: &GpuBufferRegistry,
    device: &Device,
    command_buffer: vk::CommandBuffer,
    pipeline: &RRPipeline,
    graphics_resources: &GraphicsResources,
    object_index: usize,
    image_index: usize,
) {
    if lines.indices.is_empty() {
        return;
    }

    let vertex_buffer = match registry.get_vertex_buffer(lines.vertex_buffer_handle) {
        Some(vb) => vb,
        None => return,
    };
    let index_buffer = match registry.get_index_buffer(lines.index_buffer_handle) {
        Some(ib) => ib,
        None => return,
    };

    device.cmd_bind_pipeline(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        pipeline.pipeline,
    );

    device.cmd_set_line_width(command_buffer, 1.0);
    device.cmd_bind_vertex_buffers(command_buffer, 0, &[vertex_buffer], &[0]);
    device.cmd_bind_index_buffer(command_buffer, index_buffer, 0, vk::IndexType::UINT32);

    let frame_set = graphics_resources.frame_set.sets[image_index];
    device.cmd_bind_descriptor_sets(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        pipeline.pipeline_layout,
        0,
        &[frame_set],
        &[],
    );

    let object_set_idx = graphics_resources
        .objects
        .get_set_index(image_index, object_index);
    let object_set = graphics_resources.objects.sets[object_set_idx];
    device.cmd_bind_descriptor_sets(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        pipeline.pipeline_layout,
        2,
        &[object_set],
        &[],
    );

    device.cmd_draw_indexed(command_buffer, lines.indices.len() as u32, 1, 0, 0, 0);
}
