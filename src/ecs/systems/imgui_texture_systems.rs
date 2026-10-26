use anyhow::{anyhow, Result};
use imgui::{SnapshotTextureId, TextureFormat, TextureId, TextureUploadRect};

use crate::ecs::resource::ImguiTextures;
use crate::vulkanr::command::RRCommandPool;
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::{shader_bindings, ReflectedSetLayout};
use crate::vulkanr::resource::{GpuResource, PixelRegion, SampledTexture};
use crate::vulkanr::vulkan::*;
use thyllore_vulkan_core::types::Sampler;

fn pixel_region(rect: &TextureUploadRect) -> PixelRegion<'_> {
    PixelRegion {
        x: u32::from(rect.rect.x),
        y: u32::from(rect.rect.y),
        width: u32::from(rect.rect.w),
        height: u32::from(rect.rect.h),
        row_pitch: rect.row_pitch,
        pixels: &rect.data,
    }
}

pub unsafe fn imgui_texture_create(
    instance: &Instance,
    rrdevice: &RRDevice,
    command_pool: &RRCommandPool,
    layout: &ReflectedSetLayout,
    sampler: Sampler,
    imgui_textures: &mut ImguiTextures,
    id: SnapshotTextureId,
    region: PixelRegion<'_>,
) -> Result<TextureId> {
    let (width, height) = (region.width, region.height);
    let texture = SampledTexture::create(
        instance,
        rrdevice,
        command_pool,
        layout,
        shader_bindings::imgui::TEX_SAMPLER,
        sampler,
        region,
    )?;
    let descriptor_set = texture.descriptor_set();
    log!(
        "ImGui texture created: {}x{} ({:?})",
        width,
        height,
        descriptor_set
    );

    imgui_textures.textures.insert(id, texture);
    Ok(TextureId::new(descriptor_set.as_raw()))
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
        .ok_or_else(|| anyhow!("ImGui update for unknown texture {id:?}"))?;
    let regions: Vec<PixelRegion<'_>> = rects.iter().map(pixel_region).collect();

    texture.upload_regions(instance, rrdevice, command_pool, &regions)?;
    Ok(TextureId::new(texture.descriptor_set().as_raw()))
}

pub unsafe fn imgui_texture_destroy(
    rrdevice: &RRDevice,
    imgui_textures: &mut ImguiTextures,
    id: SnapshotTextureId,
) -> Result<()> {
    rrdevice.device.device_wait_idle()?;
    let mut texture = imgui_textures
        .textures
        .remove(&id)
        .ok_or_else(|| anyhow!("ImGui destroy for unknown texture {id:?}"))?;
    texture.destroy_gpu(rrdevice);
    Ok(())
}

pub fn ensure_rgba32(format: TextureFormat) -> Result<()> {
    if format == TextureFormat::RGBA32 {
        Ok(())
    } else {
        Err(anyhow!("ImGui texture format {format:?} is not RGBA32"))
    }
}
