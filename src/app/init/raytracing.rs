use crate::app::{App, AppData};
use crate::renderer::deferred::create_gbuffer_framebuffer;
use crate::app::billboard::BillboardData;
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
        rrswapchain: &RRSwapchain,
        rrrender: &RRRender,
    ) -> Result<()> {
        let (offscreen_render_pass, offscreen_extent) =
            if let Some(ref offscreen) = data.viewport.offscreen {
                (Some(offscreen.render_pass), Some(offscreen.extent()))
            } else {
                (None, None)
            };

        let hdr_render_pass = data
            .viewport
            .hdr_buffer
            .as_ref()
            .map(|hdr| hdr.render_pass);

        {
            let mut billboard = data.ecs_world.resource_mut::<BillboardData>();
            data.raytracing.create_pipelines(
                instance,
                rrdevice,
                rrswapchain,
                rrrender,
                &data.graphics_resources,
                &mut billboard.render_state.descriptor_set,
                offscreen_render_pass,
                offscreen_extent,
                hdr_render_pass,
            )?;
        }

        Self::create_tonemap_pipeline_with_resources(rrdevice, data, rrrender)?;
        Self::create_bloom_pipelines_with_resources(rrdevice, data, rrrender)?;

        Ok(())
    }

    pub(crate) unsafe fn create_tonemap_pipeline_with_resources(
        rrdevice: &RRDevice,
        data: &mut AppData,
        rrrender: &RRRender,
    ) -> Result<()> {
        let (hdr_image_view, hdr_sampler) =
            match data.viewport.hdr_buffer {
                Some(ref hdr) => (hdr.color_image_view, hdr.sampler),
                None => {
                    crate::log!("HDR buffer not available, skipping tonemap pipeline");
                    return Ok(());
                }
            };

        let (offscreen_render_pass, offscreen_extent) =
            match data.viewport.offscreen {
                Some(ref offscreen) => (offscreen.render_pass, offscreen.extent()),
                None => {
                    crate::log!("Offscreen not available, skipping tonemap pipeline");
                    return Ok(());
                }
            };

        data.raytracing.create_tonemap_pipeline(
            rrdevice,
            rrrender,
            hdr_image_view,
            hdr_sampler,
            offscreen_render_pass,
            offscreen_extent,
        )?;

        crate::log!("Tonemap pipeline created successfully");
        Ok(())
    }

    pub(crate) unsafe fn create_bloom_pipelines_with_resources(
        rrdevice: &RRDevice,
        data: &mut AppData,
        rrrender: &RRRender,
    ) -> Result<()> {
        let hdr_image_view = match data.viewport.hdr_buffer {
            Some(ref hdr) => hdr.color_image_view,
            None => {
                crate::log!("HDR buffer not available, skipping bloom pipelines");
                return Ok(());
            }
        };

        let bloom_chain = match data.viewport.bloom_chain {
            Some(ref chain) => chain,
            None => {
                crate::log!("Bloom chain not available, skipping bloom pipelines");
                return Ok(());
            }
        };

        data.raytracing.create_bloom_pipelines(
            rrdevice,
            rrrender,
            hdr_image_view,
            bloom_chain,
        )?;

        if let (Some(ref bloom_chain), Some(ref tonemap_desc)) =
            (&data.viewport.bloom_chain, &data.raytracing.tonemap_descriptor)
        {
            if let Some(ref first_mip) = bloom_chain.mip_levels.first() {
                tonemap_desc.update_bloom_sampler(
                    rrdevice,
                    first_mip.image_view,
                    bloom_chain.sampler,
                )?;
                crate::log!("Updated tonemap descriptor with bloom texture");
            }
        }

        crate::log!("Bloom pipelines created successfully");
        Ok(())
    }
}
