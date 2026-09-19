use std::path::Path;

use crate::command::*;
use crate::core::device::*;
use crate::resource::buffer::create_buffer;
use crate::vulkan::*;

pub unsafe fn copy_image_to_host_buffer(
    instance: &Instance,
    rrdevice: &RRDevice,
    queue: vk::Queue,
    command_pool: vk::CommandPool,
    image: vk::Image,
    width: u32,
    height: u32,
    image_size: vk::DeviceSize,
    layout: vk::ImageLayout,
) -> Result<(vk::Buffer, vk::DeviceMemory)> {
    let (buffer, buffer_memory) = create_buffer(
        instance,
        rrdevice,
        image_size,
        vk::BufferUsageFlags::TRANSFER_DST,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;

    let command_buffer = begin_single_time_commands(rrdevice, command_pool)?;
    record_image_to_buffer_copy(
        &rrdevice.device,
        command_buffer,
        image,
        buffer,
        width,
        height,
        layout,
    );
    end_single_time_commands(rrdevice, queue, command_pool, command_buffer)?;

    Ok((buffer, buffer_memory))
}

unsafe fn record_image_to_buffer_copy(
    device: &crate::core::device::Device,
    command_buffer: vk::CommandBuffer,
    image: vk::Image,
    buffer: vk::Buffer,
    width: u32,
    height: u32,
    layout: vk::ImageLayout,
) {
    let subresource_range = vk::ImageSubresourceRange {
        aspect_mask: vk::ImageAspectFlags::COLOR,
        base_mip_level: 0,
        level_count: 1,
        base_array_layer: 0,
        layer_count: 1,
    };

    let barrier_to_transfer = vk::ImageMemoryBarrier::builder()
        .old_layout(layout)
        .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(image)
        .subresource_range(subresource_range)
        .src_access_mask(vk::AccessFlags::MEMORY_READ)
        .dst_access_mask(vk::AccessFlags::TRANSFER_READ);
    device.cmd_pipeline_barrier(
        command_buffer,
        vk::PipelineStageFlags::TRANSFER,
        vk::PipelineStageFlags::TRANSFER,
        vk::DependencyFlags::empty(),
        &[] as &[vk::MemoryBarrier],
        &[] as &[vk::BufferMemoryBarrier],
        &[barrier_to_transfer.build()],
    );

    let region = vk::BufferImageCopy::builder()
        .buffer_offset(0)
        .buffer_row_length(0)
        .buffer_image_height(0)
        .image_subresource(vk::ImageSubresourceLayers {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            mip_level: 0,
            base_array_layer: 0,
            layer_count: 1,
        })
        .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
        .image_extent(vk::Extent3D {
            width,
            height,
            depth: 1,
        });
    device.cmd_copy_image_to_buffer(
        command_buffer,
        image,
        vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
        buffer,
        &[region.build()],
    );

    let barrier_back = vk::ImageMemoryBarrier::builder()
        .old_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
        .new_layout(layout)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(image)
        .subresource_range(subresource_range)
        .src_access_mask(vk::AccessFlags::TRANSFER_READ)
        .dst_access_mask(vk::AccessFlags::MEMORY_READ);
    device.cmd_pipeline_barrier(
        command_buffer,
        vk::PipelineStageFlags::TRANSFER,
        vk::PipelineStageFlags::TRANSFER,
        vk::DependencyFlags::empty(),
        &[] as &[vk::MemoryBarrier],
        &[] as &[vk::BufferMemoryBarrier],
        &[barrier_back.build()],
    );
}

/// Writes a host-visible BGRA8 readback buffer as an RGBA PNG.
pub unsafe fn write_host_buffer_bgra_png(
    device: &crate::core::device::Device,
    buffer_memory: vk::DeviceMemory,
    image_size: vk::DeviceSize,
    width: u32,
    height: u32,
    path: &Path,
) -> Result<()> {
    let data = device.map_memory(buffer_memory, 0, image_size, vk::MemoryMapFlags::empty())?;
    let bgra = std::slice::from_raw_parts(data as *const u8, image_size as usize);
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    for (pixel_out, pixel_in) in rgba.chunks_exact_mut(4).zip(bgra.chunks_exact(4)) {
        pixel_out[0] = pixel_in[2];
        pixel_out[1] = pixel_in[1];
        pixel_out[2] = pixel_in[0];
        pixel_out[3] = pixel_in[3];
    }
    device.unmap_memory(buffer_memory);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let writer = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut encoder = png::Encoder::new(writer, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&rgba)?;
    Ok(())
}
