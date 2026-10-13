use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::ecs::resource::{
    BloomSettings, Exposure, LensEffects, PostProcessPreview, SceneBrightness, ToneMapOperator,
    ToneMapping,
};
use crate::ecs::systems::current_post_process_preview;
use crate::ecs::PassContext;

struct TonemapSettings {
    tonemap: ToneMapping,
    exposure: Exposure,
    lens: LensEffects,
    bloom: BloomSettings,
}

pub unsafe fn record_tonemap_to_offscreen(
    ctx: &PassContext,
    command_buffer: vk::CommandBuffer,
    image_index: usize,
) -> Result<()> {
    let offscreen = ctx
        .offscreen
        .ok_or_else(|| anyhow::anyhow!("Offscreen framebuffer not initialized"))?;

    let render_pass = offscreen.render_pass;
    let framebuffer = offscreen.framebuffer;
    let extent = offscreen.extent();

    let pipeline = ctx
        .raytracing
        .tonemap_pipeline
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("ToneMap pipeline not initialized"))?;
    let descriptor = ctx
        .raytracing
        .tonemap_descriptor
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("ToneMap descriptor not initialized"))?;

    let settings = match current_post_process_preview(ctx.world) {
        PostProcessPreview::Enabled => collect_configured_tonemap_settings(ctx),
        PostProcessPreview::Disabled => neutral_tonemap_settings(),
    };

    let render = ctx.frame_render_context(image_index);

    let plume_data = ctx
        .world
        .get_resource::<crate::ecs::resource::HeatDistortionSource>()
        .and_then(|source| source.active)
        .map(|distortion| distortion.push_data());
    thyllore_vulkan_core::renderer::begin_tonemap_render_pass(
        &render,
        render_pass,
        framebuffer,
        extent,
        command_buffer,
    );
    let frame_slot = ctx
        .world
        .resource::<crate::vulkanr::context::FrameSync>()
        .current_frame;
    thyllore_vulkan_core::renderer::record_tonemap_draw(
        &render,
        pipeline,
        descriptor,
        frame_slot,
        &settings.tonemap,
        &settings.exposure,
        &settings.lens,
        &settings.bloom,
        extent,
        command_buffer,
        plume_data,
    )?;
    // The grid was drawn inside record_composite_to_hdr, below the effect stage.
    super::OverlayRenderer::new(&render, ctx.world).draw_all_overlays(command_buffer, false)?;
    thyllore_vulkan_core::renderer::end_tonemap_render_pass(&render, command_buffer);

    Ok(())
}

fn collect_configured_tonemap_settings(ctx: &PassContext) -> TonemapSettings {
    let tonemap = ctx
        .world
        .get_resource::<ToneMapping>()
        .map(|tonemap| tonemap.clone())
        .unwrap_or_default();
    let exposure = ctx
        .world
        .get_resource::<Exposure>()
        .map(|exposure| exposure.clone())
        .unwrap_or_default();
    let lens = ctx
        .world
        .get_resource::<LensEffects>()
        .map(|lens| lens.clone())
        .unwrap_or_default();
    let bloom = ctx
        .world
        .get_resource::<BloomSettings>()
        .map(|bloom| bloom.clone())
        .unwrap_or_default();
    let brightness = ctx
        .world
        .get_resource::<SceneBrightness>()
        .map(|b| *b)
        .unwrap_or_default();

    let displayed_exposure = Exposure {
        exposure_value: exposure.display_multiplier(brightness),
        ..Exposure::default()
    };

    TonemapSettings {
        tonemap,
        exposure: displayed_exposure,
        lens,
        bloom,
    }
}

fn neutral_tonemap_settings() -> TonemapSettings {
    TonemapSettings {
        tonemap: ToneMapping {
            operator: ToneMapOperator::None,
            ..ToneMapping::default()
        },
        exposure: Exposure {
            exposure_value: 1.0,
            ..Exposure::default()
        },
        lens: LensEffects {
            vignette_enabled: false,
            chromatic_aberration_enabled: false,
            ..LensEffects::default()
        },
        bloom: BloomSettings {
            enabled: false,
            ..BloomSettings::default()
        },
    }
}
