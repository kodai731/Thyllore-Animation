use std::ptr::copy_nonoverlapping;

use anyhow::{anyhow, Result};
use imgui::{SnapshotTextureId, TextureFormat, TextureId, TextureUploadRect};

use crate::ecs::resource::{ImguiTexture, ImguiTextures};
use crate::vulkanr::buffer::create_buffer;
use crate::vulkanr::command::{
    begin_single_time_commands, end_single_time_commands, RRCommandPool,
};
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::{shader_bindings, ReflectedSetLayout};
use crate::vulkanr::image::create_image as create_vulkan_image;
use crate::vulkanr::image::create_image_view as create_vulkan_image_view;
use crate::vulkanr::vulkan::*;

pub const TEXTURE_FORMAT: vk::Format = vk::Format::R8G8B8A8_UNORM;
pub const BYTES_PER_PIXEL: usize = 4;

pub struct PixelRegion<'a> {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub row_pitch: usize,
    pub pixels: &'a [u8],
}

impl<'a> PixelRegion<'a> {
    pub fn from_upload_rect(rect: &'a TextureUploadRect) -> Self {
        Self {
            x: u32::from(rect.rect.x),
            y: u32::from(rect.rect.y),
            width: u32::from(rect.rect.w),
            height: u32::from(rect.rect.h),
            row_pitch: rect.row_pitch,
            pixels: &rect.data,
        }
    }

    fn packed_len(&self) -> usize {
        self.width as usize * self.height as usize * BYTES_PER_PIXEL
    }
}

pub unsafe fn imgui_texture_create(
    instance: &Instance,
    rrdevice: &RRDevice,
    command_pool: &RRCommandPool,
    layout: &ReflectedSetLayout,
    sampler: vk::Sampler,
    imgui_textures: &mut ImguiTextures,
    id: SnapshotTextureId,
    region: PixelRegion<'_>,
) -> Result<TextureId> {
    let (width, height) = (region.width, region.height);
    let (image, memory) = create_vulkan_image(
        instance,
        rrdevice,
        width,
        height,
        1,
        vk::SampleCountFlags::_1,
        TEXTURE_FORMAT,
        vk::ImageTiling::OPTIMAL,
        vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;
    let view = create_vulkan_image_view(
        rrdevice,
        image,
        TEXTURE_FORMAT,
        vk::ImageAspectFlags::COLOR,
        1,
    )?;
    let descriptor_set = allocate_imgui_descriptor_set(rrdevice, layout, sampler, view)?;

    copy_regions_to_image(
        instance,
        rrdevice,
        command_pool,
        image,
        vk::ImageLayout::UNDEFINED,
        &[region],
    )?;

    let texture = ImguiTexture {
        image,
        memory,
        view,
        descriptor_set,
    };
    log!(
        "ImGui texture created: {}x{} ({:?})",
        width,
        height,
        descriptor_set
    );
    imgui_textures.textures.insert(id, texture);
    Ok(TextureId::new(texture.texture_id()))
}

unsafe fn allocate_imgui_descriptor_set(
    rrdevice: &RRDevice,
    layout: &ReflectedSetLayout,
    sampler: vk::Sampler,
    view: vk::ImageView,
) -> Result<vk::DescriptorSet> {
    let descriptor_set = layout.allocate_set(rrdevice)?;
    layout
        .writer(descriptor_set)
        .image(
            shader_bindings::imgui::TEX_SAMPLER,
            view,
            sampler,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        )?
        .apply(rrdevice);
    Ok(descriptor_set)
}

pub unsafe fn imgui_texture_update(
    instance: &Instance,
    rrdevice: &RRDevice,
    command_pool: &RRCommandPool,
    imgui_textures: &mut ImguiTextures,
    id: SnapshotTextureId,
    rects: &[TextureUploadRect],
) -> Result<TextureId> {
    let texture = imgui_textures
        .textures
        .get(&id)
        .copied()
        .ok_or_else(|| anyhow!("ImGui update for unknown texture {id:?}"))?;
    let regions: Vec<PixelRegion<'_>> = rects.iter().map(PixelRegion::from_upload_rect).collect();

    copy_regions_to_image(
        instance,
        rrdevice,
        command_pool,
        texture.image,
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        &regions,
    )?;
    Ok(TextureId::new(texture.texture_id()))
}

pub unsafe fn imgui_texture_destroy(
    rrdevice: &RRDevice,
    imgui_textures: &mut ImguiTextures,
    id: SnapshotTextureId,
) -> Result<()> {
    rrdevice.device.device_wait_idle()?;
    if !imgui_textures.destroy_texture(&rrdevice.device, id) {
        return Err(anyhow!("ImGui destroy for unknown texture {id:?}"));
    }
    Ok(())
}

pub fn ensure_rgba32(format: TextureFormat) -> Result<()> {
    if format == TextureFormat::RGBA32 {
        Ok(())
    } else {
        Err(anyhow!("ImGui texture format {format:?} is not RGBA32"))
    }
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

/// Packs the regions into one staging buffer and copies them into `image`, which ends in
/// `SHADER_READ_ONLY_OPTIMAL`; waits for the copy so the pixel slices can be released.
unsafe fn copy_regions_to_image(
    instance: &Instance,
    rrdevice: &RRDevice,
    command_pool: &RRCommandPool,
    image: vk::Image,
    old_layout: vk::ImageLayout,
    regions: &[PixelRegion<'_>],
) -> Result<()> {
    let total_len: usize = regions.iter().map(PixelRegion::packed_len).sum();
    if total_len == 0 {
        return Ok(());
    }
    let (staging_buffer, staging_memory) = create_buffer(
        instance,
        rrdevice,
        total_len as vk::DeviceSize,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;
    let copies = pack_regions(rrdevice, staging_memory, total_len, regions)?;

    let device = &rrdevice.device;
    let command_buffer = begin_single_time_commands(rrdevice, command_pool.command_pool)?;
    transition_for_transfer(device, command_buffer, image, old_layout);
    device.cmd_copy_buffer_to_image(
        command_buffer,
        staging_buffer,
        image,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        &copies,
    );
    transition_for_sampling(device, command_buffer, image);
    end_single_time_commands(
        rrdevice,
        rrdevice.graphics_queue,
        command_pool.command_pool,
        command_buffer,
    )?;

    device.destroy_buffer(staging_buffer, None);
    device.free_memory(staging_memory, None);
    Ok(())
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
        let row_len = region.width as usize * BYTES_PER_PIXEL;
        for row in 0..region.height as usize {
            let source = region
                .pixels
                .get(row * region.row_pitch..row * region.row_pitch + row_len)
                .ok_or_else(|| anyhow!("ImGui texture region pixels are shorter than its rows"))?;
            copy_nonoverlapping(source.as_ptr(), mapped.add(offset + row * row_len), row_len);
        }

        copies.push(
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
                .build(),
        );
        offset += region.packed_len();
    }

    rrdevice.device.unmap_memory(staging_memory);
    Ok(copies)
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
