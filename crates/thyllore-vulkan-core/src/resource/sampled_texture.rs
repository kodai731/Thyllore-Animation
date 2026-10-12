use std::ptr::copy_nonoverlapping;

use anyhow::{anyhow, Result};
use vulkanalia::prelude::v1_0::*;

use crate::command::{begin_single_time_commands, end_single_time_commands, RRCommandPool};
use crate::core::device::RRDevice;
use crate::descriptor::ReflectedSetLayout;
use crate::resource::allocation::{create_buffer, ImageAllocation};
use crate::resource::gpu_resource::{GpuResource, OwnedDescriptorSet};
use crate::resource::image::{create_image, create_image_view};
use thyllore_spirv_reflect::ShaderBinding;

pub const SAMPLED_TEXTURE_FORMAT: vk::Format = vk::Format::R8G8B8A8_UNORM;
pub const SAMPLED_TEXTURE_BYTES_PER_PIXEL: usize = 4;

/// Tightly packed rows of RGBA8 pixels destined for a rectangle of a texture.
pub struct PixelRegion<'a> {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub row_pitch: usize,
    pub pixels: &'a [u8],
}

impl PixelRegion<'_> {
    fn packed_len(&self) -> usize {
        self.width as usize * self.height as usize * SAMPLED_TEXTURE_BYTES_PER_PIXEL
    }
}

/// A 2D RGBA8 texture with its view and the descriptor set that samples it.
#[must_use]
#[derive(Debug, Default, GpuResource)]
pub struct SampledTexture {
    allocation: ImageAllocation,
    view: vk::ImageView,
    descriptor_set: OwnedDescriptorSet,
}

impl SampledTexture {
    pub unsafe fn create(
        instance: &Instance,
        rrdevice: &RRDevice,
        command_pool: &RRCommandPool,
        layout: &ReflectedSetLayout,
        sampler_binding: ShaderBinding,
        sampler: vk::Sampler,
        region: PixelRegion<'_>,
    ) -> Result<Self> {
        let allocation = create_image(
            instance,
            rrdevice,
            region.width,
            region.height,
            1,
            vk::SampleCountFlags::_1,
            SAMPLED_TEXTURE_FORMAT,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;
        let view = create_image_view(
            rrdevice,
            allocation.image,
            SAMPLED_TEXTURE_FORMAT,
            vk::ImageAspectFlags::COLOR,
            1,
        )?;

        let descriptor_set = layout.allocate_set(rrdevice)?;
        layout
            .writer(descriptor_set)
            .image(
                sampler_binding,
                view,
                sampler,
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            )?
            .apply(rrdevice);

        let texture = Self {
            allocation,
            view,
            descriptor_set: OwnedDescriptorSet::new(descriptor_set, layout.handle),
        };
        texture.copy_regions(
            instance,
            rrdevice,
            command_pool,
            vk::ImageLayout::UNDEFINED,
            &[region],
        )?;
        Ok(texture)
    }

    pub fn descriptor_set(&self) -> vk::DescriptorSet {
        self.descriptor_set.handle()
    }

    pub unsafe fn upload_regions(
        &self,
        instance: &Instance,
        rrdevice: &RRDevice,
        command_pool: &RRCommandPool,
        regions: &[PixelRegion<'_>],
    ) -> Result<()> {
        self.copy_regions(
            instance,
            rrdevice,
            command_pool,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            regions,
        )
    }

    /// Packs the regions into one staging buffer and copies them in; the image ends in
    /// `SHADER_READ_ONLY_OPTIMAL` and the copy is complete when this returns.
    unsafe fn copy_regions(
        &self,
        instance: &Instance,
        rrdevice: &RRDevice,
        command_pool: &RRCommandPool,
        old_layout: vk::ImageLayout,
        regions: &[PixelRegion<'_>],
    ) -> Result<()> {
        let total_len: usize = regions.iter().map(PixelRegion::packed_len).sum();
        if total_len == 0 {
            return Ok(());
        }
        let mut staging = create_buffer(
            instance,
            rrdevice,
            total_len as vk::DeviceSize,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        let copies = pack_regions(rrdevice, staging.memory, total_len, regions)?;

        let device = &rrdevice.device;
        let command_buffer = begin_single_time_commands(rrdevice, command_pool.command_pool)?;
        transition_for_transfer(device, command_buffer, self.allocation.image, old_layout);
        device.cmd_copy_buffer_to_image(
            command_buffer,
            staging.buffer,
            self.allocation.image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &copies,
        );
        transition_for_sampling(device, command_buffer, self.allocation.image);
        end_single_time_commands(
            rrdevice,
            rrdevice.graphics_queue,
            command_pool.command_pool,
            command_buffer,
        )?;

        staging.destroy_gpu(rrdevice);
        Ok(())
    }
}

unsafe fn pack_regions(
    rrdevice: &RRDevice,
    staging_memory: vk::DeviceMemory,
    total_len: usize,
    regions: &[PixelRegion<'_>],
) -> Result<Vec<vk::BufferImageCopy>> {
    let mapped = rrdevice
        .device
        .map_memory(
            staging_memory,
            0,
            total_len as vk::DeviceSize,
            vk::MemoryMapFlags::empty(),
        )?
        .cast::<u8>();

    let mut copies = Vec::with_capacity(regions.len());
    let mut offset = 0usize;
    for region in regions {
        let row_len = region.width as usize * SAMPLED_TEXTURE_BYTES_PER_PIXEL;
        for row in 0..region.height as usize {
            let source = region
                .pixels
                .get(row * region.row_pitch..row * region.row_pitch + row_len)
                .ok_or_else(|| anyhow!("pixel region is shorter than its rows"))?;
            copy_nonoverlapping(source.as_ptr(), mapped.add(offset + row * row_len), row_len);
        }

        copies.push(buffer_image_copy(offset, region));
        offset += region.packed_len();
    }

    rrdevice.device.unmap_memory(staging_memory);
    Ok(copies)
}

fn buffer_image_copy(offset: usize, region: &PixelRegion<'_>) -> vk::BufferImageCopy {
    vk::BufferImageCopy::builder()
        .buffer_offset(offset as vk::DeviceSize)
        .buffer_row_length(0)
        .buffer_image_height(0)
        .image_subresource(vk::ImageSubresourceLayers {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            mip_level: 0,
            base_array_layer: 0,
            layer_count: 1,
        })
        .image_offset(vk::Offset3D {
            x: region.x as i32,
            y: region.y as i32,
            z: 0,
        })
        .image_extent(vk::Extent3D {
            width: region.width,
            height: region.height,
            depth: 1,
        })
        .build()
}

fn color_subresource_range() -> vk::ImageSubresourceRange {
    vk::ImageSubresourceRange {
        aspect_mask: vk::ImageAspectFlags::COLOR,
        base_mip_level: 0,
        level_count: 1,
        base_array_layer: 0,
        layer_count: 1,
    }
}

unsafe fn transition_for_transfer(
    device: &vulkanalia::Device,
    command_buffer: vk::CommandBuffer,
    image: vk::Image,
    old_layout: vk::ImageLayout,
) {
    let (src_access, src_stage) = if old_layout == vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL {
        (
            vk::AccessFlags::SHADER_READ,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
        )
    } else {
        (
            vk::AccessFlags::empty(),
            vk::PipelineStageFlags::TOP_OF_PIPE,
        )
    };
    let barrier = vk::ImageMemoryBarrier::builder()
        .old_layout(old_layout)
        .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(image)
        .subresource_range(color_subresource_range())
        .src_access_mask(src_access)
        .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE);
    device.cmd_pipeline_barrier(
        command_buffer,
        src_stage,
        vk::PipelineStageFlags::TRANSFER,
        vk::DependencyFlags::empty(),
        &[] as &[vk::MemoryBarrier],
        &[] as &[vk::BufferMemoryBarrier],
        &[barrier],
    );
}

unsafe fn transition_for_sampling(
    device: &vulkanalia::Device,
    command_buffer: vk::CommandBuffer,
    image: vk::Image,
) {
    let barrier = vk::ImageMemoryBarrier::builder()
        .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
        .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(image)
        .subresource_range(color_subresource_range())
        .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
        .dst_access_mask(vk::AccessFlags::SHADER_READ);
    device.cmd_pipeline_barrier(
        command_buffer,
        vk::PipelineStageFlags::TRANSFER,
        vk::PipelineStageFlags::FRAGMENT_SHADER,
        vk::DependencyFlags::empty(),
        &[] as &[vk::MemoryBarrier],
        &[] as &[vk::BufferMemoryBarrier],
        &[barrier],
    );
}
