use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::ecs::resource::{FlameRenderTargets, FlameSdfSource};
use crate::ecs::EffectContext;
use crate::vulkanr::context::CommandState;
use crate::vulkanr::descriptor::FlameImageBindings;
use crate::vulkanr::render::RRRender;

fn load_sdf_pixels(source: Option<&FlameSdfSource>) -> (Vec<u8>, u32, u32) {
    let white_pixel = (vec![255u8; 4], 1, 1);
    let Some(source) = source else {
        return white_pixel;
    };
    match thyllore_effect_core::flame_sdf::load_flame_sdf(&source.path) {
        Ok(sdf) => {
            let pixels = thyllore_effect_core::flame_sdf::encode_sdf_rgba8(&sdf);
            (pixels, sdf.width, sdf.height)
        }
        Err(e) => {
            eprintln!("flame_sdf load failed: {e}");
            white_pixel
        }
    }
}

/// Builds the SDF texture from the file requested at startup (a white pixel without one) and
/// binds it into the flame descriptor.
pub(super) unsafe fn init_sdf_texture(ctx: &mut EffectContext, rrrender: &RRRender) -> Result<()> {
    let (pixels, width, height) =
        load_sdf_pixels(ctx.world.get_resource::<FlameSdfSource>().as_deref());

    let (instance, rrdevice) = (ctx.instance, ctx.rrdevice);
    let command_pool = ctx.world.resource::<CommandState>().pool.clone();
    let (image, image_memory, mips) =
        thyllore_vulkan_core::resource::create_texture_image_pixel_with_format(
            instance,
            rrdevice,
            &command_pool,
            &pixels,
            width,
            height,
            vk::Format::R8G8B8A8_UNORM,
        )?;
    let image_view = thyllore_vulkan_core::resource::create_image_view(
        rrdevice,
        image,
        vk::Format::R8G8B8A8_UNORM,
        vk::ImageAspectFlags::COLOR,
        mips,
    )?;
    let sampler = thyllore_vulkan_core::resource::create_texture_sampler(rrdevice, mips)?;
    ctx.raytracing.flame_sdf = thyllore_vulkan_core::resource::RRImage {
        image,
        image_memory,
        image_view,
        sampler,
    };

    let Some(flame_targets) = ctx.world.get_resource::<FlameRenderTargets>() else {
        return Ok(());
    };
    if let Some(ref flame_descriptor) = ctx.raytracing.flame_descriptor {
        flame_descriptor.update_image_views(
            rrdevice,
            FlameImageBindings {
                history_image_views: flame_targets.buffer.history_image_views,
                flame_sampler: flame_targets.buffer.sampler,
                sdf_image_view: image_view,
                sdf_sampler: sampler,
                scene_depth_view: rrrender.gbuffer_depth_image_view,
            },
        )?;
    }
    Ok(())
}
