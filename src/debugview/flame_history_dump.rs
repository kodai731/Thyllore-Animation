use anyhow::Result;

use crate::ecs::component::{FlameBaked, FlameEffect, FlameTemporalAccum};
use crate::ecs::resource::{FlameDumpSink, FlameRenderTargets};
use crate::ecs::systems::flame_dump_npy_path;
use crate::hooks::batch_capture::{BatchCapture, CaptureContext};
use crate::vulkanr::vulkan::*;
use thyllore_effect_core::{build_flame_ubo, FlameUBO};
use thyllore_math_core::{f16_to_f32, write_npy_f32};

/// With `--flame-dump`, writes the current flame history image and the first flame's UBO next to
/// the dump file (`flame_XX.npy` in sequence mode).
impl BatchCapture for FlameDumpSink {
    unsafe fn capture(&self, ctx: &CaptureContext) -> Result<()> {
        let npy_path = match &ctx.slot.sequence_dir {
            Some(dir) => dir.join(format!("flame_{:02}.npy", ctx.slot.index)),
            None => flame_dump_npy_path(&self.path),
        };
        save_flame_history_npy(ctx, &npy_path)?;

        let ubo_path = npy_path.with_file_name(format!(
            "{}.ubo.bin",
            npy_path.file_stem().unwrap_or_default().to_string_lossy()
        ));
        write_first_flame_ubo(ctx, &ubo_path)
    }
}

crate::batch_capture!(FlameDumpSink);

fn write_first_flame_ubo(ctx: &CaptureContext, path: &std::path::Path) -> Result<()> {
    let Some(&first) = ctx.world.entities_with::<FlameEffect>().first() else {
        return Ok(());
    };
    let (Some(effect), Some(baked), Some(temporal)) = (
        ctx.world.get_component::<FlameEffect>(first),
        ctx.world.get_component::<FlameBaked>(first),
        ctx.world.get_component::<FlameTemporalAccum>(first),
    ) else {
        return Ok(());
    };
    let ubo = build_flame_ubo(&effect, &baked, &temporal);
    let bytes = unsafe {
        std::slice::from_raw_parts(
            &ubo as *const FlameUBO as *const u8,
            std::mem::size_of::<FlameUBO>(),
        )
    };
    std::fs::write(path, bytes)?;
    Ok(())
}

unsafe fn save_flame_history_npy(ctx: &CaptureContext, path: &std::path::Path) -> Result<()> {
    let device = &ctx.device.device;
    let flame_targets = ctx
        .world
        .get_resource::<FlameRenderTargets>()
        .ok_or_else(|| anyhow::anyhow!("flame buffer not initialized"))?;
    let flame_buffer = &flame_targets.buffer;

    let history_index = ctx
        .world
        .entities_with::<FlameEffect>()
        .first()
        .and_then(|&first| ctx.world.get_component::<FlameTemporalAccum>(first))
        .map(|temporal| (temporal.frame_index as usize) & 1)
        .unwrap_or(0);
    let history_image = flame_buffer.history_images[history_index];
    let width = flame_buffer.width;
    let height = flame_buffer.height;
    let image_size = (width * height * 8) as vk::DeviceSize;

    let (buffer, buffer_memory) = ctx.copy_image_to_buffer(
        history_image,
        width,
        height,
        image_size,
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
    )?;
    let data_ptr = device.map_memory(buffer_memory, 0, image_size, vk::MemoryMapFlags::empty())?;
    let slice = std::slice::from_raw_parts(data_ptr as *const u8, image_size as usize);
    let f32_data = decode_rgba16f(slice);
    device.unmap_memory(buffer_memory);
    device.free_memory(buffer_memory, None);
    device.destroy_buffer(buffer, None);

    write_npy_f32(path, &[height as usize, width as usize, 4], &f32_data)?;
    Ok(())
}

pub(crate) fn decode_rgba16f(bytes: &[u8]) -> Vec<f32> {
    let mut values = Vec::with_capacity(bytes.len() / 2);
    for chunk in bytes.chunks_exact(8) {
        for component in chunk.chunks_exact(2) {
            values.push(f16_to_f32(u16::from_le_bytes([component[0], component[1]])));
        }
    }
    values
}
