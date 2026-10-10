use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::core::RRDevice;
use crate::pipeline::RRPipeline;
use crate::renderer::storage_image::{
    insert_storage_image_read_barrier, insert_storage_image_write_barrier,
};
use crate::resource::VolumeImage;

#[derive(Clone, Copy, Debug)]
pub struct ShadowVolumeSpec {
    pub radial: u32,
    pub height: u32,
    pub theta: u32,
    pub slots: u32,
    pub format: vk::Format,
}

pub fn shadow_volume_extent(spec: &ShadowVolumeSpec) -> vk::Extent3D {
    vk::Extent3D {
        width: spec.radial * spec.slots,
        height: spec.height,
        depth: spec.theta,
    }
}

pub unsafe fn create_shadow_volume(
    instance: &Instance,
    rrdevice: &RRDevice,
    spec: &ShadowVolumeSpec,
) -> Result<VolumeImage> {
    VolumeImage::new(
        instance,
        rrdevice,
        shadow_volume_extent(spec),
        spec.format,
        vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
    )
}

pub struct ShadowVolumeBake<'a> {
    pub volume: &'a VolumeImage,
    pub spec: &'a ShadowVolumeSpec,
    pub pipeline: &'a RRPipeline,
    pub descriptor_sets: &'a [vk::DescriptorSet],
    pub workgroup_size: u32,
    pub reader_stage: vk::PipelineStageFlags,
}

pub unsafe fn record_shadow_volume_bake<'a>(
    device: &Device,
    cmd: vk::CommandBuffer,
    bake: &ShadowVolumeBake,
    instance_dynamic_offsets: impl IntoIterator<Item = &'a [u32]>,
) {
    let subresource_range = bake.volume.subresource_range();
    insert_storage_image_write_barrier(
        device,
        cmd,
        bake.volume.image,
        subresource_range,
        bake.reader_stage,
    );

    device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::COMPUTE, bake.pipeline.pipeline);
    let group_count_x = bake.spec.radial.div_ceil(bake.workgroup_size);
    let group_count_y = bake.spec.height.div_ceil(bake.workgroup_size);
    for dynamic_offsets in instance_dynamic_offsets {
        device.cmd_bind_descriptor_sets(
            cmd,
            vk::PipelineBindPoint::COMPUTE,
            bake.pipeline.pipeline_layout,
            0,
            bake.descriptor_sets,
            dynamic_offsets,
        );
        device.cmd_dispatch(cmd, group_count_x, group_count_y, bake.spec.theta);
    }

    insert_storage_image_read_barrier(
        device,
        cmd,
        bake.volume.image,
        subresource_range,
        bake.reader_stage,
    );
}
