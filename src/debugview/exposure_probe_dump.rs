use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use thyllore_color_core::srgb_to_linear;
use thyllore_render_core::{luminance_statistics, rec709_luminance, LuminanceStatistics};
use thyllore_vulkan_core::renderer::BACKGROUND_RADIANCE;

use crate::debugview::water_probe_dump::read_hdr_rgba16f;
use crate::ecs::resource::{
    AutoExposure, Camera, Exposure, ExposureProbeCapture, SceneBrightness, ViewportInput,
};
use crate::ecs::systems::current_unix_time;
use crate::hooks::batch_capture::{BatchCapture, CaptureContext};
use crate::vulkanr::context::SwapchainState;
use crate::vulkanr::vulkan::*;

const EXPOSURE_PROBE_DIRECTORY: &str = "log/exposure";

/// Writes the metered luminance, the adapted exposure and the luminance distribution of the HDR
/// scene and of the displayed viewport, so brightness can be compared across camera poses.
impl BatchCapture for ExposureProbeCapture {
    unsafe fn capture(&self, ctx: &CaptureContext) -> Result<()> {
        let record = build_exposure_probe_record(ctx)?;
        let path = write_exposure_probe(&record, current_unix_time())?;
        msg_info!("Exposure probe dumped: {}", path.display());
        Ok(())
    }
}

crate::batch_capture!(ExposureProbeCapture);
crate::capture_action!("dump_exposure", ExposureProbeCapture);

unsafe fn build_exposure_probe_record(ctx: &CaptureContext) -> Result<Value> {
    let hdr = ctx
        .hdr
        .ok_or_else(|| anyhow!("hdr buffer not initialized"))?;
    let hdr_rgba = read_hdr_rgba16f(ctx, hdr.color_image, hdr.width, hdr.height)?;
    let hdr_luminance: Vec<f32> = hdr_rgba
        .chunks_exact(4)
        .map(|px| rec709_luminance([px[0], px[1], px[2]]))
        .collect();
    let hdr_stats = luminance_statistics(&hdr_luminance, BACKGROUND_RADIANCE);

    let display = read_viewport_display_luminance(ctx)?;
    let display_stats = luminance_statistics(&display.linear, 0.0);

    let exposure = ctx.world.resource::<Exposure>().clone();
    let brightness = *ctx.world.resource::<SceneBrightness>();
    let auto_exposure = ctx
        .world
        .get_resource::<AutoExposure>()
        .map(|ae| ae.clone());
    let camera = ctx.world.resource::<Camera>();

    Ok(json!({
        "camera": {
            "distance": camera.distance,
            "near_plane": camera.near_plane,
            "pivot": [camera.pivot.x, camera.pivot.y, camera.pivot.z],
        },
        "exposure": {
            "exposure_value": exposure.exposure_value,
            "ev100": exposure.ev100,
            "metered_luminance": exposure.metered_luminance,
            "scene_brightness_ev": brightness.ev,
            "display_multiplier": exposure.display_multiplier(brightness),
        },
        "auto_exposure": auto_exposure.map(auto_exposure_json),
        "hdr": {
            "width": hdr.width,
            "height": hdr.height,
            "background_radiance": BACKGROUND_RADIANCE,
            "luminance": stats_json(hdr_stats.as_ref()),
        },
        "display": {
            "width": display.width,
            "height": display.height,
            "mean_srgb_255": display.mean_srgb_255,
            "linear_luminance": stats_json(display_stats.as_ref()),
        },
    }))
}

fn auto_exposure_json(ae: AutoExposure) -> Value {
    json!({
        "enabled": ae.enabled,
        "metering": ae.metering.name(),
        "white_level": ae.white_level,
        "min_ev": ae.min_ev,
        "max_ev": ae.max_ev,
        "low_percent": ae.low_percent,
        "high_percent": ae.high_percent,
        "adaptation_speed_up": ae.adaptation_speed_up,
        "adaptation_speed_down": ae.adaptation_speed_down,
    })
}

fn stats_json(stats: Option<&LuminanceStatistics>) -> Value {
    stats
        .map(|s| serde_json::to_value(s).unwrap_or(Value::Null))
        .unwrap_or(Value::Null)
}

struct ViewportDisplayLuminance {
    width: u32,
    height: u32,
    mean_srgb_255: f32,
    linear: Vec<f32>,
}

/// The presented swapchain image cropped to the Scene viewport, decoded from sRGB.
unsafe fn read_viewport_display_luminance(
    ctx: &CaptureContext,
) -> Result<ViewportDisplayLuminance> {
    let swapchain = &ctx.world.resource::<SwapchainState>().swapchain;
    let image = swapchain.swapchain_images[ctx.image_index];
    let extent = swapchain.swapchain_extent;
    let image_size = (extent.width * extent.height * 4) as vk::DeviceSize;

    let (buffer, buffer_memory) = ctx.copy_image_to_buffer(
        image,
        extent.width,
        extent.height,
        image_size,
        vk::ImageLayout::PRESENT_SRC_KHR,
    )?;
    let device = &ctx.device.device;
    let data = device.map_memory(buffer_memory, 0, image_size, vk::MemoryMapFlags::empty())?;
    let bgra = std::slice::from_raw_parts(data as *const u8, image_size as usize);

    let rect = viewport_pixel_rect(ctx.world, extent.width, extent.height);
    let mut linear = Vec::with_capacity((rect.width * rect.height) as usize);
    let mut srgb_sum = 0.0f64;
    for y in rect.y..rect.y + rect.height {
        for x in rect.x..rect.x + rect.width {
            let offset = ((y * extent.width + x) * 4) as usize;
            let b = bgra[offset] as f32 / 255.0;
            let g = bgra[offset + 1] as f32 / 255.0;
            let r = bgra[offset + 2] as f32 / 255.0;
            srgb_sum += ((r + g + b) / 3.0 * 255.0) as f64;
            linear.push(rec709_luminance([
                srgb_to_linear(r),
                srgb_to_linear(g),
                srgb_to_linear(b),
            ]));
        }
    }

    device.unmap_memory(buffer_memory);
    device.free_memory(buffer_memory, None);
    device.destroy_buffer(buffer, None);

    let pixel_count = linear.len().max(1) as f64;
    Ok(ViewportDisplayLuminance {
        width: rect.width,
        height: rect.height,
        mean_srgb_255: (srgb_sum / pixel_count) as f32,
        linear,
    })
}

struct PixelRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

fn viewport_pixel_rect(
    world: &crate::ecs::World,
    image_width: u32,
    image_height: u32,
) -> PixelRect {
    let viewport = world.resource::<ViewportInput>();
    let x = (viewport.position[0].max(0.0) as u32).min(image_width);
    let y = (viewport.position[1].max(0.0) as u32).min(image_height);
    let width = (viewport.size[0].max(0.0) as u32).min(image_width - x);
    let height = (viewport.size[1].max(0.0) as u32).min(image_height - y);
    if width == 0 || height == 0 {
        return PixelRect {
            x: 0,
            y: 0,
            width: image_width,
            height: image_height,
        };
    }
    PixelRect {
        x,
        y,
        width,
        height,
    }
}

fn write_exposure_probe(record: &Value, unix_time: u64) -> Result<PathBuf> {
    let directory = Path::new(EXPOSURE_PROBE_DIRECTORY);
    std::fs::create_dir_all(directory)?;
    let path = directory.join(format!("exposure_probe_{unix_time}.json"));
    std::fs::write(&path, serde_json::to_string_pretty(record)?)?;
    Ok(path)
}
