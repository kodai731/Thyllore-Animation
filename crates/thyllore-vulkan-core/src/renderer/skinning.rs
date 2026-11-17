use std::mem::size_of;

use vulkanalia::prelude::v1_0::*;

use crate::core::RRDevice;
use crate::descriptor::shader_bindings::skinning::PUSH_CONSTANT;
use crate::pipeline::RRPipeline;
use crate::resource::SkinStreamAddresses;

const SKINNING_GROUP_SIZE: u32 = 64;

/// Mirrors `SkinningPush` of `shaders/model/skinningCompute.slang`.
#[repr(C, packed(4))]
#[derive(Clone, Copy)]
struct SkinningPush {
    rest_address: u64,
    influence_address: u64,
    palette_address: u64,
    skinned_address: u64,
    vertex_count: u32,
}

const _: () = assert!(size_of::<SkinningPush>() == PUSH_CONSTANT.size as usize);

pub(crate) unsafe fn record_skinning_dispatch(
    rrdevice: &RRDevice,
    command_buffer: vk::CommandBuffer,
    pipeline: &RRPipeline,
    streams: SkinStreamAddresses,
    skinned_address: vk::DeviceAddress,
    vertex_count: u32,
) {
    let device = rrdevice.device();
    device.cmd_bind_pipeline(
        command_buffer,
        vk::PipelineBindPoint::COMPUTE,
        pipeline.pipeline,
    );

    let push = SkinningPush {
        rest_address: streams.rest,
        influence_address: streams.influences,
        palette_address: streams.palette,
        skinned_address,
        vertex_count,
    };
    device.cmd_push_constants(
        command_buffer,
        pipeline.pipeline_layout,
        vk::ShaderStageFlags::COMPUTE,
        0,
        std::slice::from_raw_parts((&raw const push).cast::<u8>(), size_of::<SkinningPush>()),
    );

    device.cmd_dispatch(
        command_buffer,
        vertex_count.div_ceil(SKINNING_GROUP_SIZE),
        1,
        1,
    );
}
