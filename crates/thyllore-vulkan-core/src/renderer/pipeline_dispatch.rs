use crate::core::device::RRDevice;
use crate::pipeline::{RRPipeline, RRRayTracingPipeline};
use crate::types::CommandBuffer;
use crate::vulkan::*;
use vulkanalia::vk::KhrRayTracingPipelineExtension;

pub enum ClearColor {
    Uint([u32; 4]),
    Float([f32; 4]),
}

pub unsafe fn bind_compute(
    rrdevice: &RRDevice,
    command_buffer: CommandBuffer,
    pipeline: &RRPipeline,
    descriptor_sets: &[crate::types::DescriptorSet],
    dynamic_offsets: &[u32],
) {
    let command_buffer = command_buffer.raw();
    let descriptor_sets: Vec<vk::DescriptorSet> =
        descriptor_sets.iter().map(|set| set.raw()).collect();
    rrdevice.device().cmd_bind_pipeline(
        command_buffer,
        vk::PipelineBindPoint::COMPUTE,
        pipeline.pipeline,
    );
    rrdevice.device().cmd_bind_descriptor_sets(
        command_buffer,
        vk::PipelineBindPoint::COMPUTE,
        pipeline.pipeline_layout,
        0,
        &descriptor_sets,
        dynamic_offsets,
    );
}

pub unsafe fn dispatch(
    rrdevice: &RRDevice,
    command_buffer: CommandBuffer,
    group_x: u32,
    group_y: u32,
    group_z: u32,
) {
    let command_buffer = command_buffer.raw();
    rrdevice
        .device()
        .cmd_dispatch(command_buffer, group_x, group_y, group_z);
}

pub unsafe fn bind_ray_tracing(
    rrdevice: &RRDevice,
    command_buffer: CommandBuffer,
    pipeline: &RRRayTracingPipeline,
    descriptor_sets: &[crate::types::DescriptorSet],
) {
    let command_buffer = command_buffer.raw();
    rrdevice.device().cmd_bind_pipeline(
        command_buffer,
        vk::PipelineBindPoint::RAY_TRACING_KHR,
        pipeline.pipeline,
    );
    rrdevice.device().cmd_bind_descriptor_sets(
        command_buffer,
        vk::PipelineBindPoint::RAY_TRACING_KHR,
        pipeline.pipeline_layout,
        0,
        &descriptor_sets
            .iter()
            .map(|ds| ds.raw())
            .collect::<Vec<_>>(),
        &[],
    );
}

pub unsafe fn push_constants_and_trace_rays(
    rrdevice: &RRDevice,
    command_buffer: CommandBuffer,
    pipeline: &RRRayTracingPipeline,
    push_range: &crate::types::PushConstantRange,
    push_bytes: &[u8],
    width: u32,
    height: u32,
) {
    let command_buffer = command_buffer.raw();
    let push_range = push_range.raw();
    rrdevice.device().cmd_push_constants(
        command_buffer,
        pipeline.pipeline_layout,
        push_range.stage_flags,
        push_range.offset,
        push_bytes,
    );
    rrdevice.device().cmd_trace_rays_khr(
        command_buffer,
        &pipeline.raygen_region,
        &pipeline.miss_region,
        &pipeline.hit_region,
        &pipeline.callable_region,
        width,
        height,
        1,
    );
}

pub unsafe fn clear_color_image(
    rrdevice: &RRDevice,
    command_buffer: CommandBuffer,
    image: crate::types::Image,
    color: ClearColor,
) {
    let command_buffer = command_buffer.raw();
    let clear_value = match color {
        ClearColor::Uint(uint32) => vk::ClearColorValue { uint32 },
        ClearColor::Float(float32) => vk::ClearColorValue { float32 },
    };
    let subresource_range = vk::ImageSubresourceRange {
        aspect_mask: vk::ImageAspectFlags::COLOR,
        base_mip_level: 0,
        level_count: 1,
        base_array_layer: 0,
        layer_count: 1,
    };
    rrdevice.device().cmd_clear_color_image(
        command_buffer,
        image.raw(),
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        &clear_value,
        &[subresource_range],
    );
}
