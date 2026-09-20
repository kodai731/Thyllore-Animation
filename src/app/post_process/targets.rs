use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::ecs::resource::PostProcessFrameTargets;
use crate::ecs::PassContext;
use crate::hooks::pass::TransientSlot;
use crate::vulkanr::resource::BloomMipTarget;

pub const DOF_OUTPUT: TransientSlot = TransientSlot("dof.output");
pub const MAX_BLOOM_MIPS: usize = 8;
pub const BLOOM_MIPS: [TransientSlot; MAX_BLOOM_MIPS] = [
    TransientSlot("bloom.mip0"),
    TransientSlot("bloom.mip1"),
    TransientSlot("bloom.mip2"),
    TransientSlot("bloom.mip3"),
    TransientSlot("bloom.mip4"),
    TransientSlot("bloom.mip5"),
    TransientSlot("bloom.mip6"),
    TransientSlot("bloom.mip7"),
];

const HDR_DIRECT_GENERATION: u64 = 0;

struct InputBinding {
    view: vk::ImageView,
    sampler: vk::Sampler,
    generation: u64,
}

pub fn is_dof_enabled(ctx: &PassContext) -> bool {
    ctx.raytracing.dof_pipeline.is_some() && ctx.dof_buffer.is_some()
}

pub fn bloom_mip_count(ctx: &PassContext) -> usize {
    let enabled = ctx
        .world
        .get_resource::<crate::ecs::resource::BloomSettings>()
        .is_some_and(|settings| settings.enabled)
        && ctx.raytracing.bloom_downsample_pipeline.is_some();
    if !enabled {
        return 0;
    }
    ctx.bloom_chain
        .map(|chain| chain.mip_count().min(MAX_BLOOM_MIPS))
        .unwrap_or(0)
}

fn hdr_binding(ctx: &PassContext) -> Result<InputBinding> {
    ctx.hdr_buffer
        .map(|hdr| InputBinding {
            view: hdr.color_image_view,
            sampler: hdr.sampler,
            generation: HDR_DIRECT_GENERATION,
        })
        .ok_or_else(|| anyhow::anyhow!("HDR buffer not initialized"))
}

fn transient_binding(
    ctx: &PassContext,
    slot: TransientSlot,
    sampler: vk::Sampler,
) -> Result<InputBinding> {
    let image = ctx.transient_image(slot)?;
    Ok(InputBinding {
        view: image.view,
        sampler,
        generation: image.generation,
    })
}

fn tonemap_input_binding(ctx: &PassContext) -> Result<InputBinding> {
    match ctx.dof_buffer {
        Some(dof_buffer) if is_dof_enabled(ctx) => {
            transient_binding(ctx, DOF_OUTPUT, dof_buffer.sampler)
        }
        _ => hdr_binding(ctx),
    }
}

fn bloom_input_binding(ctx: &PassContext) -> Result<InputBinding> {
    match ctx.bloom_chain {
        Some(chain) if bloom_mip_count(ctx) > 0 => {
            transient_binding(ctx, BLOOM_MIPS[0], chain.sampler)
        }
        _ => hdr_binding(ctx),
    }
}

pub unsafe fn prepare_dof_target(ctx: &mut PassContext) -> Result<()> {
    let Some(dof_buffer) = ctx.dof_buffer.filter(|_| is_dof_enabled(ctx)) else {
        return Ok(());
    };
    let desc = dof_buffer.output_desc();
    let output = ctx.transient_image(DOF_OUTPUT)?;
    ctx.transient.framebuffer(
        &ctx.rrdevice.device,
        dof_buffer.render_pass,
        &[output.view],
        desc.width,
        desc.height,
    )?;
    Ok(())
}

pub unsafe fn prepare_bloom_targets(ctx: &mut PassContext, frame_slot: usize) -> Result<()> {
    let mut targets = ctx.world.resource_mut::<PostProcessFrameTargets>();
    let mip_count = bloom_mip_count(ctx);
    let Some(bloom_chain) = ctx.bloom_chain.filter(|_| mip_count > 0) else {
        targets.bloom_mips.clear();
        return Ok(());
    };
    let sampler = bloom_chain.sampler;
    let render_pass = bloom_chain.downsample_render_pass;

    let mut mips = Vec::with_capacity(mip_count);
    let mut generations = Vec::with_capacity(mip_count);
    for slot in &BLOOM_MIPS[..mip_count] {
        let handle = ctx.frame_transients.handle(*slot)?;
        let image = ctx.transient.get(handle)?;
        let extent = vk::Extent2D {
            width: image.desc.width,
            height: image.desc.height,
        };
        let framebuffer = ctx.transient.framebuffer(
            &ctx.rrdevice.device,
            render_pass,
            &[image.view],
            extent.width,
            extent.height,
        )?;
        mips.push(BloomMipTarget {
            image: image.image,
            view: image.view,
            framebuffer,
            extent,
        });
        generations.push(image.generation);
    }
    let views: Vec<vk::ImageView> = mips.iter().map(|mip| mip.view).collect();
    targets.bloom_mips = mips;

    if targets.bloom_bound.is_bound(frame_slot, &generations) {
        return Ok(());
    }
    if let (Some(bloom_descriptors), Some(hdr_buffer)) =
        (ctx.raytracing.bloom_descriptors.as_ref(), ctx.hdr_buffer)
    {
        bloom_descriptors.update_image_views_at(
            ctx.rrdevice,
            frame_slot,
            hdr_buffer.color_image_view,
            &views,
            sampler,
        )?;
    }
    targets.bloom_bound.mark_bound(frame_slot, generations);
    Ok(())
}

pub unsafe fn prepare_auto_exposure_input(ctx: &mut PassContext, frame_slot: usize) -> Result<()> {
    let mut targets = ctx.world.resource_mut::<PostProcessFrameTargets>();
    let input = tonemap_input_binding(ctx)?;
    let generations = vec![input.generation];
    if targets.histogram_bound.is_bound(frame_slot, &generations) {
        return Ok(());
    }

    if let Some(histogram_descriptor) = ctx.raytracing.auto_exposure_histogram_descriptor.as_ref() {
        histogram_descriptor.update_hdr_image_at(
            ctx.rrdevice,
            frame_slot,
            input.view,
            input.sampler,
        )?;
    }
    targets.histogram_bound.mark_bound(frame_slot, generations);
    Ok(())
}

pub unsafe fn prepare_tonemap_inputs(ctx: &mut PassContext, frame_slot: usize) -> Result<()> {
    let mut targets = ctx.world.resource_mut::<PostProcessFrameTargets>();
    let tonemap_input = tonemap_input_binding(ctx)?;
    let bloom_input = bloom_input_binding(ctx)?;
    let generations = vec![tonemap_input.generation, bloom_input.generation];
    if targets.tonemap_bound.is_bound(frame_slot, &generations) {
        return Ok(());
    }

    if let Some(tonemap_descriptor) = ctx.raytracing.tonemap_descriptor.as_ref() {
        tonemap_descriptor.update_hdr_sampler_at(
            ctx.rrdevice,
            frame_slot,
            tonemap_input.view,
            tonemap_input.sampler,
        )?;
        tonemap_descriptor.update_bloom_sampler_at(
            ctx.rrdevice,
            frame_slot,
            bloom_input.view,
            bloom_input.sampler,
        )?;
    }
    targets.tonemap_bound.mark_bound(frame_slot, generations);
    Ok(())
}
