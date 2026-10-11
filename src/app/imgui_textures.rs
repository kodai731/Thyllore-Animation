use std::ptr::copy_nonoverlapping;

use anyhow::{anyhow, Result};
use imgui::{
    SnapshotTextureId, TextureFeedback, TextureFormat, TextureId, TextureOp, TextureRequest,
    TextureUploadRect,
};

use crate::app::App;
use crate::platform::imgui::ImguiTexture;
use crate::vulkanr::command::RRCommandPool;
use crate::vulkanr::context::CommandState;
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::shader_bindings;
use crate::vulkanr::vulkan::*;

const TEXTURE_FORMAT: vk::Format = vk::Format::R8G8B8A8_UNORM;
const BYTES_PER_PIXEL: usize = 4;

struct PixelRegion<'a> {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    row_pitch: usize,
    pixels: &'a [u8],
}

impl<'a> PixelRegion<'a> {
    fn from_upload_rect(rect: &'a TextureUploadRect) -> Self {
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

impl App {
    /// Answer every texture request of the frame Dear ImGui just rendered.
    pub unsafe fn apply_imgui_texture_requests(
        &mut self,
        requests: &[TextureRequest],
    ) -> Result<Vec<TextureFeedback>> {
        let mut feedback = Vec::with_capacity(requests.len());
        for request in requests {
            let answer = match request.operation() {
                TextureOp::Create {
                    format,
                    width,
                    height,
                    row_pitch,
                    pixels,
                } => {
                    ensure_rgba32(*format)?;
                    let region = PixelRegion {
                        x: 0,
                        y: 0,
                        width: *width,
                        height: *height,
                        row_pitch: *row_pitch,
                        pixels,
                    };
                    let texture_id = self.create_imgui_texture(request.texture(), region)?;
                    request.uploaded(texture_id)?
                }
                TextureOp::Update { format, rects, .. } => {
                    ensure_rgba32(*format)?;
                    let texture_id = self.update_imgui_texture(request.texture(), rects)?;
                    request.uploaded(texture_id)?
                }
                TextureOp::Destroy => {
                    self.destroy_imgui_texture(request.texture())?;
                    request.destroyed()?
                }
            };
            feedback.push(answer);
        }
        Ok(feedback)
    }

    unsafe fn create_imgui_texture(
        &mut self,
        id: SnapshotTextureId,
        region: PixelRegion<'_>,
    ) -> Result<TextureId> {
        let (width, height) = (region.width, region.height);
        let (image, memory) = create_image(&self.instance, &self.rrdevice, width, height)?;
        let view = create_image_view(&self.rrdevice.device, image)?;
        let descriptor_set = self.allocate_imgui_descriptor_set(view)?;

        let command_pool = self.resource::<CommandState>().pool.clone();
        copy_regions_to_image(
            &self.instance,
            &self.rrdevice,
            &command_pool,
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
        self.data.imgui.textures.insert(id, texture);
        Ok(TextureId::new(texture.texture_id()))
    }

    unsafe fn allocate_imgui_descriptor_set(
        &self,
        view: vk::ImageView,
    ) -> Result<vk::DescriptorSet> {
        let layout = self
            .data
            .imgui
            .descriptor_set_layout
            .as_ref()
            .ok_or_else(|| anyhow!("ImGui descriptor set layout not initialized"))?;
        let sampler = self
            .data
            .imgui
            .sampler
            .ok_or_else(|| anyhow!("ImGui sampler not initialized"))?;

        let descriptor_set = layout.allocate_set(&self.rrdevice)?;
        layout
            .writer(descriptor_set)
            .image(
                shader_bindings::imgui::TEX_SAMPLER,
                view,
                sampler,
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            )?
            .apply(&self.rrdevice);
        Ok(descriptor_set)
    }

    unsafe fn update_imgui_texture(
        &mut self,
        id: SnapshotTextureId,
        rects: &[TextureUploadRect],
    ) -> Result<TextureId> {
        let texture = self
            .data
            .imgui
            .textures
            .get(&id)
            .copied()
            .ok_or_else(|| anyhow!("ImGui update for unknown texture {id:?}"))?;
        let regions: Vec<PixelRegion<'_>> =
            rects.iter().map(PixelRegion::from_upload_rect).collect();

        let command_pool = self.resource::<CommandState>().pool.clone();
        copy_regions_to_image(
            &self.instance,
            &self.rrdevice,
            &command_pool,
            texture.image,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            &regions,
        )?;
        Ok(TextureId::new(texture.texture_id()))
    }

    unsafe fn destroy_imgui_texture(&mut self, id: SnapshotTextureId) -> Result<()> {
        self.rrdevice.device.device_wait_idle()?;
        if !self.data.imgui.destroy_texture(&self.rrdevice.device, id) {
            return Err(anyhow!("ImGui destroy for unknown texture {id:?}"));
        }
        Ok(())
    }
}

fn ensure_rgba32(format: TextureFormat) -> Result<()> {
    if format == TextureFormat::RGBA32 {
        Ok(())
    } else {
        Err(anyhow!("ImGui texture format {format:?} is not RGBA32"))
    }
}

unsafe fn create_image(
    instance: &Instance,
    rrdevice: &RRDevice,
    width: u32,
    height: u32,
) -> Result<(vk::Image, vk::DeviceMemory)> {
    let image_info = vk::ImageCreateInfo::builder()
        .image_type(vk::ImageType::_2D)
        .format(TEXTURE_FORMAT)
        .extent(vk::Extent3D {
            width,
            height,
            depth: 1,
        })
        .mip_levels(1)
        .array_layers(1)
        .samples(vk::SampleCountFlags::_1)
        .tiling(vk::ImageTiling::OPTIMAL)
        .usage(vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .initial_layout(vk::ImageLayout::UNDEFINED);
    let image = rrdevice.device.create_image(&image_info, None)?;

    let requirements = rrdevice.device.get_image_memory_requirements(image);
    let memory_type_index = get_memory_type_index(
        instance,
        rrdevice.physical_device,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
        requirements,
    )?;
    let allocate_info = vk::MemoryAllocateInfo::builder()
        .allocation_size(requirements.size)
        .memory_type_index(memory_type_index);
    let memory = rrdevice.device.allocate_memory(&allocate_info, None)?;
    rrdevice.device.bind_image_memory(image, memory, 0)?;

    Ok((image, memory))
}

unsafe fn create_image_view(
    device: &vulkanalia::Device,
    image: vk::Image,
) -> Result<vk::ImageView> {
    let view_info = vk::ImageViewCreateInfo::builder()
        .image(image)
        .view_type(vk::ImageViewType::_2D)
        .format(TEXTURE_FORMAT)
        .subresource_range(color_subresource_range());
    Ok(device.create_image_view(&view_info, None)?)
}

pub(crate) unsafe fn create_sampler(device: &vulkanalia::Device) -> Result<vk::Sampler> {
    let sampler_info = vk::SamplerCreateInfo::builder()
        .mag_filter(vk::Filter::LINEAR)
        .min_filter(vk::Filter::LINEAR)
        .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
        .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .min_lod(0.0)
        .max_lod(1.0);
    Ok(device.create_sampler(&sampler_info, None)?)
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
    let (staging_buffer, staging_memory) = create_staging_buffer(instance, rrdevice, total_len)?;
    let copies = pack_regions(rrdevice, staging_memory, total_len, regions)?;

    let device = &rrdevice.device;
    let command_buffer = begin_one_time_commands(device, command_pool)?;
    transition_for_transfer(device, command_buffer, image, old_layout);
    device.cmd_copy_buffer_to_image(
        command_buffer,
        staging_buffer,
        image,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        &copies,
    );
    transition_for_sampling(device, command_buffer, image);
    submit_and_wait(
        device,
        command_pool,
        &rrdevice.graphics_queue,
        command_buffer,
    )?;

    device.destroy_buffer(staging_buffer, None);
    device.free_memory(staging_memory, None);
    Ok(())
}

unsafe fn create_staging_buffer(
    instance: &Instance,
    rrdevice: &RRDevice,
    len: usize,
) -> Result<(vk::Buffer, vk::DeviceMemory)> {
    let buffer_info = vk::BufferCreateInfo::builder()
        .size(len as vk::DeviceSize)
        .usage(vk::BufferUsageFlags::TRANSFER_SRC)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);
    let buffer = rrdevice.device.create_buffer(&buffer_info, None)?;

    let requirements = rrdevice.device.get_buffer_memory_requirements(buffer);
    let memory_type_index = get_memory_type_index(
        instance,
        rrdevice.physical_device,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        requirements,
    )?;
    let allocate_info = vk::MemoryAllocateInfo::builder()
        .allocation_size(requirements.size)
        .memory_type_index(memory_type_index);
    let memory = rrdevice.device.allocate_memory(&allocate_info, None)?;
    rrdevice.device.bind_buffer_memory(buffer, memory, 0)?;
    Ok((buffer, memory))
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

unsafe fn begin_one_time_commands(
    device: &vulkanalia::Device,
    command_pool: &RRCommandPool,
) -> Result<vk::CommandBuffer> {
    let allocate_info = vk::CommandBufferAllocateInfo::builder()
        .command_pool(command_pool.command_pool)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(1);
    let command_buffer = device.allocate_command_buffers(&allocate_info)?[0];
    let begin_info =
        vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
    device.begin_command_buffer(command_buffer, &begin_info)?;
    Ok(command_buffer)
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

unsafe fn submit_and_wait(
    device: &vulkanalia::Device,
    command_pool: &RRCommandPool,
    graphics_queue: &vk::Queue,
    command_buffer: vk::CommandBuffer,
) -> Result<()> {
    device.end_command_buffer(command_buffer)?;
    let command_buffers = [command_buffer];
    let submit_info = vk::SubmitInfo::builder().command_buffers(&command_buffers);
    device.queue_submit(*graphics_queue, &[submit_info], vk::Fence::null())?;
    device.queue_wait_idle(*graphics_queue)?;
    device.free_command_buffers(command_pool.command_pool, &[command_buffer]);
    Ok(())
}
