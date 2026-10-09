use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::app::{App, AppData};
use crate::ecs::EffectContext;
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::render::RRRender;

pub fn build_effect_context<'a>(
    instance: &'a Instance,
    rrdevice: &'a RRDevice,
    data: &'a mut AppData,
) -> EffectContext<'a> {
    EffectContext {
        instance,
        rrdevice,
        graphics: &data.graphics_resources,
        viewport_width: data.viewport.width,
        viewport_height: data.viewport.height,
        hdr_color_view: data
            .viewport
            .hdr_buffer
            .as_ref()
            .map(|hdr| hdr.color_image_view),
        storage: &mut data.viewport.storage,
        raytracing: &mut data.raytracing,
        pass_image_states: &mut data.pass_image_states,
        world: &mut data.ecs_world,
    }
}

pub unsafe fn run_effect_setup(
    instance: &Instance,
    rrdevice: &RRDevice,
    data: &mut AppData,
    rrrender: &RRRender,
) -> Result<()> {
    let hooks = data.effect_hooks.snapshot();
    let mut ctx = build_effect_context(instance, rrdevice, data);
    for hook in hooks {
        if let Some(setup) = hook.setup {
            setup(&mut ctx, rrrender)?;
        }
    }
    Ok(())
}

/// Runs the GPU work that depends on the startup overrides, once they are applied.
pub unsafe fn run_effect_after_overrides(
    instance: &Instance,
    rrdevice: &RRDevice,
    data: &mut AppData,
    rrrender: &RRRender,
) -> Result<()> {
    let hooks = data.effect_hooks.snapshot();
    let mut ctx = build_effect_context(instance, rrdevice, data);
    for hook in hooks {
        if let Some(after_overrides) = hook.after_overrides {
            after_overrides(&mut ctx, rrrender)?;
        }
    }
    Ok(())
}

impl App {
    pub unsafe fn run_effect_viewport_resize(&mut self) -> Result<()> {
        let hooks = self.data.effect_hooks.snapshot();
        let mut ctx = build_effect_context(&self.instance, &self.rrdevice, &mut self.data);
        for hook in hooks {
            if let Some(on_viewport_resize) = hook.on_viewport_resize {
                on_viewport_resize(&mut ctx)?;
            }
        }
        Ok(())
    }
}
