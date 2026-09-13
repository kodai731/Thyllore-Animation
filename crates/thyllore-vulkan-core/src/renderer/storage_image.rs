use vulkanalia::prelude::v1_0::*;

/// Makes a storage image writable by compute after `reader_stage` last sampled it; the previous
/// contents are discarded.
pub unsafe fn insert_storage_image_write_barrier(
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
pub unsafe fn insert_storage_image_read_barrier(
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
