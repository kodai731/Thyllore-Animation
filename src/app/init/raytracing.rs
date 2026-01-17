use crate::app::{App, AppData};
use crate::renderer::deferred::create_gbuffer_framebuffer;
use crate::scene::Scene;
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::render::create_gbuffer_render_pass;

use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

impl App {
    pub(crate) unsafe fn init_ray_tracing(
        instance: &Instance,
        rrdevice: &RRDevice,
        data: &mut AppData,
    ) -> Result<()> {
        log::info!("Initializing Ray Tracing resources...");

        data.raytracing.init_gbuffer(
            instance,
            rrdevice,
            &data.rrswapchain,
            &data.rrcommand_pool,
        )?;

        create_gbuffer_render_pass(instance, rrdevice, &mut data.rrrender)?;

        if let Some(ref gbuffer) = data.raytracing.gbuffer {
            create_gbuffer_framebuffer(instance, rrdevice, &mut data.rrrender, gbuffer)?;
        }
        log::info!("Created G-Buffer render pass and framebuffer");

        log::info!("Ray Tracing initialization complete");
        Ok(())
    }

    pub(crate) unsafe fn build_acceleration_structures(
        instance: &Instance,
        rrdevice: &RRDevice,
        data: &mut AppData,
    ) -> Result<()> {
        data.raytracing.build_acceleration_structures(
            instance,
            rrdevice,
            &data.rrcommand_pool,
            &data.graphics_resources.meshes,
        )
    }

    pub(crate) unsafe fn create_ray_tracing_pipelines(
        instance: &Instance,
        rrdevice: &RRDevice,
        data: &mut AppData,
        scene: &Scene,
    ) -> Result<()> {
        let mut billboard = scene.billboard_mut();
        data.raytracing.create_pipelines(
            instance,
            rrdevice,
            &data.rrswapchain,
            &data.rrrender,
            &data.graphics_resources,
            &mut billboard.descriptor_set,
        )
    }
}
