use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::ecs::effect_context::MAX_FRAMES_IN_FLIGHT;
use crate::ecs::resource::WaterRenderTargets;
use crate::ecs::EffectContext;
use crate::hooks::effect::EffectHook;
use crate::vulkanr::context::RenderTargets;
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::render::RRRender;
use crate::vulkanr::resource::WaterBuffer;
use crate::AppData;

pub const WATER_EFFECT_HOOK: EffectHook = EffectHook {
    name: "water",
    setup: Some(setup_water),
    on_viewport_resize: Some(resize_water_render_targets),
    destroy: Some(destroy_water_render_targets),
    passes: super::passes::WATER_PASS_NODES,
};

unsafe fn create_water_render_targets(
    ctx: &mut EffectContext,
    depth_view: vk::ImageView,
) -> Result<bool> {
    let Some(hdr_view) = ctx.hdr_color_view else {
        return Ok(false);
    };

    let buffer = WaterBuffer::new(
        ctx.instance,
        ctx.rrdevice,
        ctx.storage,
        ctx.raytracing.command_pool,
        ctx.viewport_width,
        ctx.viewport_height,
        hdr_view,
        depth_view,
    )?;

    for image in buffer.history_images {
        ctx.pass_image_states.mark_shader_read_only(image);
    }
    ctx.pass_image_states.forget(buffer.caustic_accum_image);
    ctx.world.insert_resource(WaterRenderTargets::new(buffer));
    Ok(true)
}

unsafe fn setup_water(
    instance: &Instance,
    rrdevice: &RRDevice,
    data: &mut AppData,
    rrrender: &RRRender,
) -> Result<()> {
    let mut setup_ctx = EffectContext {
        instance,
        rrdevice,
        viewport_width: data.viewport.width,
        viewport_height: data.viewport.height,
        hdr_color_view: data
            .viewport
            .hdr_buffer
            .as_ref()
            .map(|hdr| hdr.color_image_view),
        storage: &mut data.viewport.storage,
        transient: &mut data.viewport.transient,
        raytracing: &mut data.raytracing,
        pass_image_states: &mut data.pass_image_states,
        world: &mut data.ecs_world,
        frames_in_flight: MAX_FRAMES_IN_FLIGHT,
    };
    let created = create_water_render_targets(&mut setup_ctx, rrrender.gbuffer_depth_image_view)?;
    if !created {
        log!("HDR buffer not available, skipping water pipeline");
        return Ok(());
    }

    let (Some(water_targets), Some(hdr_buffer)) = (
        data.ecs_world
            .get_resource::<crate::ecs::resource::WaterRenderTargets>(),
        data.viewport.hdr_buffer.as_ref(),
    ) else {
        log!("Water buffer not available, skipping water pipeline");
        return Ok(());
    };

    data.raytracing.create_water_pipeline(
        instance,
        rrdevice,
        rrrender,
        &data.graphics_resources,
        &water_targets.buffer,
        hdr_buffer,
        MAX_FRAMES_IN_FLIGHT,
    )?;

    log!("Water pipeline created successfully");
    Ok(())
}

unsafe fn resize_water_render_targets(ctx: &mut EffectContext) -> Result<()> {
    let depth_view = ctx
        .world
        .resource::<RenderTargets>()
        .render
        .gbuffer_depth_image_view;
    if depth_view == vk::ImageView::null() {
        return Ok(());
    }
    if ctx.world.get_resource::<WaterRenderTargets>().is_none() {
        return Ok(());
    }

    destroy_water_render_targets(ctx)?;
    if !create_water_render_targets(ctx, depth_view)? {
        return Ok(());
    }

    update_water_caustic_descriptor(ctx)
}

unsafe fn destroy_water_render_targets(ctx: &mut EffectContext) -> Result<()> {
    if let Some(mut targets) = ctx.world.get_resource_mut::<WaterRenderTargets>() {
        for image in targets.buffer.history_images {
            ctx.pass_image_states.forget(image);
        }
        ctx.pass_image_states
            .forget(targets.buffer.caustic_accum_image);
        targets.buffer.destroy(&ctx.rrdevice.device);
        targets.forget_bindings();
    }
    Ok(())
}

unsafe fn update_water_caustic_descriptor(ctx: &mut EffectContext) -> Result<()> {
    let Some(caustic_accum_view) = ctx
        .world
        .get_resource::<WaterRenderTargets>()
        .map(|targets| targets.buffer.caustic_accum_view)
    else {
        return Ok(());
    };
    let Some(hdr_color_view) = ctx.hdr_color_view else {
        return Ok(());
    };

    let rrdevice = ctx.rrdevice;
    let raytracing = &mut *ctx.raytracing;
    let tlas = raytracing
        .acceleration_structure
        .as_ref()
        .and_then(|accel| accel.tlas.acceleration_structure);
    let (Some(position_image_view), Some(scene_buffer), Some(water_ubo)) = (
        raytracing
            .gbuffer
            .as_ref()
            .map(|gbuffer| gbuffer.position_image_view),
        raytracing.scene_uniform_buffer,
        raytracing.water_ubo.as_ref().map(|ubo| ubo.handle()),
    ) else {
        return Ok(());
    };
    let Some(descriptor) = raytracing.water_caustic_descriptor.as_mut() else {
        return Ok(());
    };

    descriptor.allocate_and_update(
        rrdevice,
        caustic_accum_view,
        position_image_view,
        tlas,
        scene_buffer,
        water_ubo,
        hdr_color_view,
    )
}
