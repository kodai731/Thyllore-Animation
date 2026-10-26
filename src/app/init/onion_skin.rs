use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::app::{App, AppData};
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::{ONION_SKIN_COMPOSITE, ONION_SKIN_GHOST};
use crate::vulkanr::pipeline::{
    BlendConfig, CullFace, DepthCompare, DepthTestConfig, FillMode, PipelineBuilder,
    PushConstantConfig, SampleCount, ShaderStages, Topology, VertexInputConfig,
};
use crate::vulkanr::render::RRRender;
use crate::vulkanr::resource::{GraphicsResources, OnionSkinPassResources};
use thyllore_vulkan_core::renderer::{OnionSkinPushConstants, OverlayBlend};
use thyllore_vulkan_core::types::{Extent2D, ImageView};

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
            Some(ref offscreen) => (offscreen.resolve_color_image_view, offscreen.format),
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
    offscreen_format: vk::Format,
    width: u32,
    height: u32,
) -> Result<OnionSkinPassResources> {
    let extent = Extent2D { width, height };
    let (ghost_image, ghost_image_memory, ghost_image_view, ghost_sampler) =
        OnionSkinPassResources::create_ghost_buffer(instance, rrdevice, width, height)?;
    let ghost_render_pass = OnionSkinPassResources::create_ghost_render_pass(rrdevice)?;

    let ghost_pipeline = PipelineBuilder::from_pass(&ONION_SKIN_GHOST)
        .vertex_input(VertexInputConfig::Standard)
        .topology(Topology::TriangleList)
        .polygon_mode(FillMode::Fill)
        .cull_mode(CullFace::Back)
        .custom_render_pass(ghost_render_pass)
        .mrt_attachments(1)
        .msaa_samples(SampleCount::One)
        .depth_test(DepthTestConfig {
            test_enable: false,
            write_enable: false,
            compare_op: DepthCompare::Always.into(),
        })
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
    let ghost_framebuffer = OnionSkinPassResources::create_single_framebuffer(
        rrdevice,
        ghost_render_pass,
        ghost_image_view,
        width,
        height,
    )?;

    let composite_render_pass =
        OnionSkinPassResources::create_composite_render_pass(rrdevice, offscreen_format)?;
    let (composite_descriptor_layout, composite_descriptor_set) =
        OnionSkinPassResources::create_composite_descriptor(
            rrdevice,
            ghost_image_view,
            ghost_sampler,
        )?;
    let composite_pipeline = PipelineBuilder::from_pass(&ONION_SKIN_COMPOSITE)
        .vertex_input(VertexInputConfig::Custom {
            bindings: vec![],
            attributes: vec![],
        })
        .topology(Topology::TriangleList)
        .polygon_mode(FillMode::Fill)
        .no_depth_test()
        .custom_render_pass(composite_render_pass)
        .msaa_samples(SampleCount::One)
        .blend(OverlayBlend::Premultiplied.blend_config())
        .descriptor_layouts(&[&composite_descriptor_layout])
        .build(rrdevice, rrrender, Some(extent))?;
    let composite_framebuffer = OnionSkinPassResources::create_single_framebuffer(
        rrdevice,
        composite_render_pass,
        offscreen_resolve_image_view,
        width,
        height,
    )?;

    log!("Created onion skin pass: {}x{}", width, height);
    Ok(OnionSkinPassResources {
        ghost_image,
        ghost_image_memory,
        ghost_image_view,
        ghost_sampler,
        ghost_render_pass,
        ghost_framebuffer,
        ghost_pipeline,
        composite_render_pass,
        composite_framebuffer,
        composite_pipeline,
        composite_descriptor_layout,
        composite_descriptor_set,
        width,
        height,
    })
}
