use crate::app::{App, AppData};

use crate::vulkanr::context::RenderConfig;
use crate::vulkanr::descriptor::*;
use crate::vulkanr::device::*;
use crate::vulkanr::pipeline::RRPipeline;
use crate::vulkanr::render::*;

use anyhow::{anyhow, Result};

impl App {
    pub unsafe fn init_imgui_rendering(
        rrdevice: &RRDevice,
        data: &mut AppData,
        imgui: &mut imgui::Context,
        rrrender: &RRRender,
    ) -> Result<()> {
        log!("Initializing ImGui Vulkan rendering resources");

        let descriptor_set_layout = ReflectedSetLayout::create(rrdevice, &imgui_layout_spec())?;
        let sampler = crate::vulkanr::image::create_linear_clamp_sampler(rrdevice)?;

        let msaa_samples = data.ecs_world.resource::<RenderConfig>().msaa_samples;
        let imgui_pipeline = RRPipeline::new_imgui(
            rrdevice,
            rrrender,
            &descriptor_set_layout,
            &IMGUI,
            msaa_samples,
        )?;

        let renderer_consumer = imgui
            .create_synchronous_renderer_consumer()
            .map_err(|error| anyhow!("ImGui renderer consumer: {error}"))?;

        data.imgui.pipeline = Some(imgui_pipeline.pipeline());
        data.imgui.pipeline_layout = Some(imgui_pipeline.pipeline_layout());
        data.imgui.descriptor_set_layout = Some(descriptor_set_layout);
        data.imgui.sampler = Some(sampler);
        data.imgui.renderer_consumer = Some(renderer_consumer);
        data.ecs_world
            .insert_resource(crate::ecs::resource::ImguiTextures::default());
        log!("ImGui rendering resources initialized successfully");
        log!("  Pipeline: {:?}", imgui_pipeline.pipeline());
        Ok(())
    }
}
