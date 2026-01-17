use crate::app::{App, AppData};
use crate::renderer::deferred::create_gbuffer_framebuffer;
use crate::scene::Scene;
use crate::vulkanr::command::RRCommandPool;
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::render::{create_gbuffer_render_pass, RRRender};
use crate::vulkanr::swapchain::RRSwapchain;

use anyhow::Result;
use std::rc::Rc;
use vulkanalia::prelude::v1_0::*;

impl App {
    pub(crate) unsafe fn init_ray_tracing_with_resources(
        instance: &Instance,
        rrdevice: &RRDevice,
        data: &mut AppData,
        rrswapchain: &RRSwapchain,
        rrcommand_pool: &RRCommandPool,
        rrrender: &mut RRRender,
    ) -> Result<()> {
        log::info!("Initializing Ray Tracing resources...");

        data.raytracing
            .init_gbuffer(instance, rrdevice, rrswapchain, rrcommand_pool)?;

        create_gbuffer_render_pass(instance, rrdevice, rrrender)?;

        if let Some(ref gbuffer) = data.raytracing.gbuffer {
            create_gbuffer_framebuffer(instance, rrdevice, rrrender, gbuffer)?;
        }
        log::info!("Created G-Buffer render pass and framebuffer");

        log::info!("Ray Tracing initialization complete");
        Ok(())
    }

    pub(crate) unsafe fn build_acceleration_structures_with_resources(
        instance: &Instance,
        rrdevice: &RRDevice,
        data: &mut AppData,
        rrcommand_pool: &Rc<RRCommandPool>,
    ) -> Result<()> {
        data.raytracing.build_acceleration_structures(
            instance,
            rrdevice,
            rrcommand_pool,
            &data.graphics_resources.meshes,
        )
    }

    pub(crate) unsafe fn create_ray_tracing_pipelines_with_resources(
        instance: &Instance,
        rrdevice: &RRDevice,
        data: &mut AppData,
        scene: &Scene,
        rrswapchain: &RRSwapchain,
        rrrender: &RRRender,
    ) -> Result<()> {
        let mut billboard = scene.billboard_mut();
        data.raytracing.create_pipelines(
            instance,
            rrdevice,
            rrswapchain,
            rrrender,
            &data.graphics_resources,
            &mut billboard.descriptor_set,
        )
    }
}
