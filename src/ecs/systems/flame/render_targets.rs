use anyhow::Result;

use crate::ecs::resource::{FlameGpuState, FlameHistorySnapshotState, FlameRenderTargets};
use crate::ecs::{EffectContext, MAX_FRAMES_IN_FLIGHT};
use crate::hooks::effect::EffectHook;
use crate::vulkanr::context::RenderTargets;
use crate::vulkanr::descriptor::FlameImageBindings;
use crate::vulkanr::render::RRRender;
use crate::vulkanr::resource::FlameBuffer;

pub const FLAME_EFFECT_HOOK: EffectHook = EffectHook {
    name: "flame",
    setup: Some(setup_flame),
    after_overrides: Some(super::sdf::init_sdf_texture),
    on_viewport_resize: Some(resize_flame_render_targets),
    passes: &[&super::passes::FlamePassNode],
};

unsafe fn create_flame_render_targets(ctx: &mut EffectContext) -> Result<()> {
    let Some(hdr_view) = ctx.hdr_color_view else {
        return Ok(());
    };

    let buffer = FlameBuffer::new(
        ctx.instance,
        ctx.rrdevice,
        ctx.storage,
        ctx.raytracing.command_pool,
        ctx.viewport_width,
        ctx.viewport_height,
        hdr_view,
    )?;

    for image in buffer.history_images {
        ctx.pass_image_states.mark_shader_read_only(image);
    }
    ctx.world.insert_resource(FlameRenderTargets { buffer });
    Ok(())
}

unsafe fn setup_flame(ctx: &mut EffectContext, rrrender: &RRRender) -> Result<()> {
    create_flame_render_targets(ctx)?;
    let Some(flame_targets) = ctx.world.get_resource::<FlameRenderTargets>() else {
        log!("Flame buffer not available, skipping flame pipeline");
        return Ok(());
    };
    let flame_buffer = &flame_targets.buffer;

    let position_image_view = match ctx.raytracing.gbuffer {
        Some(ref gbuffer) => gbuffer.position_image_view,
        None => {
            log!("GBuffer not available, skipping flame pipeline");
            return Ok(());
        }
    };

    let position_sampler = match ctx.raytracing.gbuffer_sampler {
        Some(sampler) => sampler,
        None => {
            log!("GBuffer sampler not available, skipping flame pipeline");
            return Ok(());
        }
    };

    let gpu_state = super::pipeline::create_flame_pipeline(
        ctx.instance,
        ctx.rrdevice,
        rrrender,
        ctx.graphics,
        flame_buffer,
        position_image_view,
        position_sampler,
        rrrender.gbuffer_depth_image_view,
    )?;
    drop(flame_targets);
    ctx.world.insert_resource(gpu_state);

    crate::ecs::systems::raytracing_systems::ensure_effect_trace_pipeline(
        ctx.instance,
        ctx.rrdevice,
        ctx.raytracing,
        MAX_FRAMES_IN_FLIGHT,
    )?;

    log!("Flame pipeline created successfully");
    Ok(())
}

unsafe fn resize_flame_render_targets(ctx: &mut EffectContext) -> Result<()> {
    let Some(hdr_view) = ctx.hdr_color_view else {
        return Ok(());
    };
    let (width, height) = (ctx.viewport_width, ctx.viewport_height);
    let command_pool = ctx.raytracing.command_pool;
    let scene_depth_view = ctx
        .world
        .resource::<RenderTargets>()
        .render
        .gbuffer_depth_image_view;

    let Some(mut targets) = ctx.world.get_resource_mut::<FlameRenderTargets>() else {
        return Ok(());
    };
    targets.buffer.resize(
        ctx.instance,
        ctx.rrdevice,
        ctx.storage,
        command_pool,
        width,
        height,
        hdr_view,
    )?;
    for image in targets.buffer.history_images {
        ctx.pass_image_states.mark_shader_read_only(image);
    }

    if let Some(gpu_state) = ctx.world.get_resource::<FlameGpuState>() {
        if let Some(descriptor) = gpu_state.descriptor.as_ref() {
            descriptor.update_image_views(
                ctx.rrdevice,
                FlameImageBindings {
                    history_image_views: targets.buffer.history_image_views,
                    flame_sampler: targets.buffer.sampler,
                    sdf_image_view: gpu_state.sdf.image_view,
                    sdf_sampler: gpu_state.sdf.sampler,
                    scene_depth_view,
                },
            )?;
        }
    }
    drop(targets);

    if let Some(mut state) = ctx.world.get_resource_mut::<FlameHistorySnapshotState>() {
        state.previous = None;
    }
    Ok(())
}
