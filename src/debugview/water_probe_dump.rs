use std::path::Path;

use anyhow::{anyhow, Result};
use cgmath::{Matrix4, SquareMatrix, Vector3};

use crate::debugview::flame_history_dump::decode_rgba16f;
use crate::ecs::component::WaterTorusEffect;
use crate::ecs::resource::{ProjectionData, WaterProbeCapture};
use crate::ecs::systems::water::probe::{
    compute_water_probe_report, inverse_view_proj_f64, ProbeRoot,
};
use crate::hooks::batch_capture::{BatchCapture, CaptureContext};
use crate::vulkanr::vulkan::*;

/// With `--batch-water-probe <path>`, compares the HDR image against the analytic torus and
/// writes `<path>.json` / `<path>.npy`.
impl BatchCapture for WaterProbeCapture {
    unsafe fn capture(&self, ctx: &CaptureContext) -> Result<()> {
        let hdr = ctx
            .hdr
            .ok_or_else(|| anyhow!("hdr buffer not initialized"))?;
        let Some(&water) = ctx.world.query_waters().first() else {
            log_warn!("water probe skipped: no water torus effect entity");
            return Ok(());
        };
        let effect = ctx
            .world
            .get_component::<WaterTorusEffect>(water)
            .ok_or_else(|| anyhow!("water entity has no effect component"))?
            .clone();

        let (width, height) = (hdr.width, hdr.height);
        let f32_data = read_hdr_rgba16f(ctx, hdr.color_image, width, height)?;

        let proj_data = ctx.world.resource::<ProjectionData>();
        let inv_view_proj = inverse_view_proj_f64(proj_data.proj, proj_data.view);
        let inv_view = proj_data.view.invert().unwrap_or_else(Matrix4::identity);
        let camera_pos = Vector3::new(inv_view[3][0], inv_view[3][1], inv_view[3][2]);
        drop(proj_data);

        let inverse_model = thyllore_effect_core::build_water_model_matrix(&effect)
            .invert()
            .unwrap_or_else(Matrix4::identity);
        let root = probe_root_of(ctx);
        let report = compute_water_probe_report(
            &f32_data,
            width,
            height,
            inv_view_proj,
            inverse_model,
            effect.major_radius,
            camera_pos,
            effect.minor_radius / effect.major_radius,
            root,
        );

        let json_path = sibling_with_extension(&self.path, "json");
        std::fs::write(&json_path, serde_json::to_string_pretty(&report)?)?;
        let npy_path = sibling_with_extension(&self.path, "npy");
        thyllore_math_core::write_npy_f32(
            &npy_path,
            &[height as usize, width as usize, 4],
            &f32_data,
        )?;

        println!(
            "water probe dumped to {} ({} pixels, {} mismatch, root={})",
            json_path.display(),
            report.pixels,
            report.count_mismatch,
            report.root
        );
        Ok(())
    }
}

crate::batch_capture!(WaterProbeCapture);

fn probe_root_of(ctx: &CaptureContext) -> ProbeRoot {
    let debug_view = ctx
        .world
        .get_resource::<crate::ecs::resource::WaterRenderSettings>()
        .map(|settings| settings.debug_view)
        .unwrap_or(3);
    if debug_view == 4 {
        ProbeRoot::Exit
    } else {
        ProbeRoot::Nearest
    }
}

unsafe fn read_hdr_rgba16f(
    ctx: &CaptureContext,
    image: vk::Image,
    width: u32,
    height: u32,
) -> Result<Vec<f32>> {
    let image_size = (width * height * 8) as vk::DeviceSize;
    let (buffer, buffer_memory) = ctx.copy_image_to_buffer(
        image,
        width,
        height,
        image_size,
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
    )?;
    let device = &ctx.device.device;
    let data_ptr = device.map_memory(buffer_memory, 0, image_size, vk::MemoryMapFlags::empty())?;
    let slice = std::slice::from_raw_parts(data_ptr as *const u8, image_size as usize);
    let values = decode_rgba16f(slice);
    device.unmap_memory(buffer_memory);
    device.free_memory(buffer_memory, None);
    device.destroy_buffer(buffer, None);
    Ok(values)
}

fn sibling_with_extension(path: &Path, extension: &str) -> std::path::PathBuf {
    path.with_file_name(format!(
        "{}.{extension}",
        path.file_stem().unwrap_or_default().to_string_lossy()
    ))
}
