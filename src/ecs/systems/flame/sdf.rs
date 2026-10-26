use anyhow::Result;

use super::FlameImageBindings;
use crate::ecs::resource::{FlameGpuState, FlameRenderTargets, FlameSdfSource};
use crate::ecs::EffectContext;
use crate::vulkanr::context::CommandState;
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
    let rgba8 = thyllore_vulkan_core::resource::create_rgba8_texture(
        instance,
        rrdevice,
        &command_pool,
        &pixels,
        width,
        height,
        thyllore_vulkan_core::resource::TextureColorSpace::Linear,
    )?;
    if ctx.world.get_resource::<FlameGpuState>().is_none() {
        ctx.world.insert_resource(FlameGpuState::default());
    }
    let mut gpu_state = ctx.world.resource_mut::<FlameGpuState>();
    gpu_state.sdf = thyllore_vulkan_core::resource::RRImage {
        image: rgba8.image,
        image_memory: rgba8.memory,
        image_view: rgba8.view,
        sampler: rgba8.sampler,
    };

    let Some(flame_targets) = ctx.world.get_resource::<FlameRenderTargets>() else {
        return Ok(());
    };
    if let Some(ref flame_descriptor) = gpu_state.descriptor {
        flame_descriptor.update_image_views(
            rrdevice,
            FlameImageBindings {
                history_image_views: flame_targets.history.views,
                flame_sampler: flame_targets.history.sampler,
                sdf_image_view: rgba8.view,
                sdf_sampler: rgba8.sampler,
                scene_depth_view: rrrender.gbuffer_depth_image_view,
            },
        )?;
    }
    Ok(())
}
