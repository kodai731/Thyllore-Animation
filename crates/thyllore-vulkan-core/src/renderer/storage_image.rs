use crate::core::device::RRDevice;
use crate::resource::VolumeImage;
use crate::types::{CommandBuffer, Extent2D, Extent3D, Image, Offset3D};
use vulkanalia::prelude::v1_0::*;

pub unsafe fn copy_color_image(
    rrdevice: &RRDevice,
    cmd: CommandBuffer,
    src: Image,
    dst: Image,
    extent: Extent2D,
) {
    let subresource = vk::ImageSubresourceLayers::builder()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .mip_level(0)
        .base_array_layer(0)
        .layer_count(1)
        .build();
    let region = vk::ImageCopy::builder()
        .src_subresource(subresource)
        .src_offset(Offset3D { x: 0, y: 0, z: 0 }.raw())
        .dst_subresource(subresource)
        .dst_offset(Offset3D { x: 0, y: 0, z: 0 }.raw())
        .extent(
            Extent3D {
                width: extent.width,
                height: extent.height,
                depth: 1,
            }
            .raw(),
        )
        .build();
    rrdevice.device().cmd_copy_image(
        cmd.raw(),
        src.raw(),
        vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
        dst.raw(),
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        &[region],
    );
}

/// Makes a storage image writable by compute after `reader_stage` last sampled it; the previous
/// contents are discarded.
pub(crate) unsafe fn insert_storage_image_write_barrier(
    device: &Device,
    cmd: vk::CommandBuffer,
    image: vk::Image,
    subresource_range: vk::ImageSubresourceRange,
    reader_stage: vk::PipelineStageFlags,
) {
    let barrier = vk::ImageMemoryBarrier::builder()
        .src_access_mask(vk::AccessFlags::SHADER_READ)
        .dst_access_mask(vk::AccessFlags::SHADER_WRITE)
        .old_layout(vk::ImageLayout::UNDEFINED)
        .new_layout(vk::ImageLayout::GENERAL)
        .image(image)
        .subresource_range(subresource_range);
    device.cmd_pipeline_barrier(
        cmd,
        reader_stage,
        vk::PipelineStageFlags::COMPUTE_SHADER,
        vk::DependencyFlags::empty(),
        &[] as &[vk::MemoryBarrier],
        &[] as &[vk::BufferMemoryBarrier],
        &[barrier],
    );
}

/// Publishes a compute-written storage image to `reader_stage`, keeping the general layout.
pub(crate) unsafe fn insert_storage_image_read_barrier(
    device: &Device,
    cmd: vk::CommandBuffer,
    image: vk::Image,
    subresource_range: vk::ImageSubresourceRange,
    reader_stage: vk::PipelineStageFlags,
) {
    let barrier = vk::ImageMemoryBarrier::builder()
        .src_access_mask(vk::AccessFlags::SHADER_WRITE)
        .dst_access_mask(vk::AccessFlags::SHADER_READ)
        .old_layout(vk::ImageLayout::GENERAL)
        .new_layout(vk::ImageLayout::GENERAL)
        .image(image)
        .subresource_range(subresource_range);
    device.cmd_pipeline_barrier(
        cmd,
        vk::PipelineStageFlags::COMPUTE_SHADER,
        reader_stage,
        vk::DependencyFlags::empty(),
        &[] as &[vk::MemoryBarrier],
        &[] as &[vk::BufferMemoryBarrier],
        &[barrier],
    );
}

pub unsafe fn storage_image_write_barrier_for_fragment(
    rrdevice: &RRDevice,
    cmd: CommandBuffer,
    volume: &VolumeImage,
) {
    insert_storage_image_write_barrier(
        &rrdevice.device(),
        cmd.raw(),
        volume.image,
        volume.subresource_range(),
        vk::PipelineStageFlags::FRAGMENT_SHADER,
    );
}

pub unsafe fn storage_image_read_barrier_for_fragment(
    rrdevice: &RRDevice,
    cmd: CommandBuffer,
    volume: &VolumeImage,
) {
    insert_storage_image_read_barrier(
        &rrdevice.device(),
        cmd.raw(),
        volume.image,
        volume.subresource_range(),
        vk::PipelineStageFlags::FRAGMENT_SHADER,
    );
}
