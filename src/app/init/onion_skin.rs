use anyhow::Result;

use crate::app::{App, AppData};
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::{ONION_SKIN_COMPOSITE, ONION_SKIN_GHOST};
use crate::vulkanr::pipeline::{
    BlendConfig, CullFace, DepthTestConfig, FillMode, PipelineBuilder, PushConstantConfig,
    SampleCount, ShaderStages, Topology, VertexInputConfig,
};
use crate::vulkanr::render::RRRender;
use crate::vulkanr::resource::{GraphicsResources, OnionSkinPassResources};
use thyllore_vulkan_core::renderer::{OnionSkinPushConstants, OverlayBlend};
use thyllore_vulkan_core::types::{Extent2D, Format, ImageView, Instance};

impl App {
    pub(crate) unsafe fn create_onion_skin_pipeline_with_resources(
        instance: &Instance,
        rrdevice: &RRDevice,
        data: &mut AppData,
        rrrender: &RRRender,
    ) -> Result<()> {
        let (width, height) = match data.viewport.hdr_buffer {
            Some(ref hdr) => (hdr.width, hdr.height),
            None => {
                log!("HDR buffer not available, skipping onion skin pipeline");
                return Ok(());
            }
        };

        let (resolve_image_view, offscreen_format) = match data.viewport.offscreen {
            Some(ref offscreen) => (offscreen.resolve_color_image_view(), offscreen.format()),
            None => {
                log!("Offscreen not available, skipping onion skin pipeline");
                return Ok(());
            }
        };

        let onion_skin_pass = create_onion_skin_pass(
            instance,
            rrdevice,
            rrrender,
            &data.graphics_resources,
            resolve_image_view,
            offscreen_format,
            width,
            height,
        )?;
        data.raytracing.onion_skin_pass = Some(onion_skin_pass);

        log!("Onion skin pipeline created successfully");
        Ok(())
    }
}

unsafe fn create_onion_skin_pass(
    instance: &Instance,
    rrdevice: &RRDevice,
    rrrender: &RRRender,
    graphics_resources: &GraphicsResources,
    offscreen_resolve_image_view: ImageView,
    offscreen_format: Format,
    width: u32,
    height: u32,
) -> Result<OnionSkinPassResources> {
    let extent = Extent2D { width, height };
    let mut onion_skin_pass = OnionSkinPassResources::create(
        instance,
        rrdevice,
        offscreen_resolve_image_view,
        offscreen_format,
        width,
        height,
    )?;

    onion_skin_pass.ghost_pipeline = PipelineBuilder::from_pass(&ONION_SKIN_GHOST)
        .vertex_input(VertexInputConfig::Standard)
        .topology(Topology::TriangleList)
        .polygon_mode(FillMode::Fill)
        .cull_mode(CullFace::Back)
        .custom_render_pass(onion_skin_pass.ghost_render_pass())
        .mrt_attachments(1)
        .msaa_samples(SampleCount::One)
        .depth_test(DepthTestConfig::disabled())
        .blend(BlendConfig::alpha())
        .descriptor_layouts(&[
            &graphics_resources.frame_set.layout,
            &graphics_resources.materials.layout,
            &graphics_resources.objects.layout,
        ])
        .push_constants(PushConstantConfig::new(
            ShaderStages::Fragment,
            0,
            std::mem::size_of::<OnionSkinPushConstants>() as u32,
        ))
        .build(rrdevice, rrrender, Some(extent))?;

    onion_skin_pass.composite_pipeline = PipelineBuilder::from_pass(&ONION_SKIN_COMPOSITE)
        .vertex_input(VertexInputConfig::Custom {
            bindings: vec![],
            attributes: vec![],
        })
        .topology(Topology::TriangleList)
        .polygon_mode(FillMode::Fill)
        .no_depth_test()
        .custom_render_pass(onion_skin_pass.composite_render_pass())
        .msaa_samples(SampleCount::One)
        .blend(OverlayBlend::Premultiplied.blend_config())
        .descriptor_layouts(&[&onion_skin_pass.composite_descriptor_layout])
        .build(rrdevice, rrrender, Some(extent))?;

    log!("Created onion skin pass: {}x{}", width, height);
    Ok(onion_skin_pass)
}
