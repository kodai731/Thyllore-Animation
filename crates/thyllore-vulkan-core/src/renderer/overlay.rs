use crate::descriptor::PassShaders;
use crate::pipeline::{BlendConfig, PipelineBuilder, VertexInputConfig};
use vulkanalia::prelude::v1_0::*;

fn premultiplied_blend() -> BlendConfig {
    BlendConfig {
        enable: true,
        src_color_factor: vk::BlendFactor::ONE,
        dst_color_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
        color_op: vk::BlendOp::ADD,
        src_alpha_factor: vk::BlendFactor::ONE,
        dst_alpha_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
        alpha_op: vk::BlendOp::ADD,
    }
}

pub fn overlay_pipeline(
    pass: &'static PassShaders,
    render_pass: vk::RenderPass,
) -> PipelineBuilder {
    PipelineBuilder::from_pass(pass)
        .vertex_input(VertexInputConfig::Custom {
            bindings: vec![],
            attributes: vec![],
        })
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .no_depth_test()
        .custom_render_pass(render_pass)
        .msaa_samples(vk::SampleCountFlags::_1)
        .blend(premultiplied_blend())
        .dynamic_states(vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR])
}

/// What a fullscreen overlay pass finds in its attachments, one clear color per attachment.
#[derive(Clone, Copy, Debug)]
pub enum OverlayAttachmentLoad<'a> {
    Keep,
    Clear(&'a [[f32; 4]]),
}

pub unsafe fn begin_overlay_render_pass(
    device: &Device,
    cmd: vk::CommandBuffer,
    render_pass: vk::RenderPass,
    framebuffer: vk::Framebuffer,
    render_area: vk::Rect2D,
    load: OverlayAttachmentLoad,
) {
    let clear_values = match load {
        OverlayAttachmentLoad::Keep => vec![],
        OverlayAttachmentLoad::Clear(colors) => colors
            .iter()
            .map(|&color| vk::ClearValue {
                color: vk::ClearColorValue { float32: color },
            })
            .collect::<Vec<_>>(),
    };
    let render_pass_info = vk::RenderPassBeginInfo::builder()
        .render_pass(render_pass)
        .framebuffer(framebuffer)
        .render_area(render_area)
        .clear_values(&clear_values);
    device.cmd_begin_render_pass(cmd, &render_pass_info, vk::SubpassContents::INLINE);
}

pub unsafe fn set_full_viewport(device: &Device, cmd: vk::CommandBuffer, extent: vk::Extent2D) {
    let viewport = vk::Viewport::builder()
        .x(0.0)
        .y(0.0)
        .width(extent.width as f32)
        .height(extent.height as f32)
        .min_depth(0.0)
        .max_depth(1.0);
    device.cmd_set_viewport(cmd, 0, &[viewport]);
}

pub unsafe fn draw_fullscreen_triangle(device: &Device, cmd: vk::CommandBuffer) {
    device.cmd_draw(cmd, 3, 1, 0, 0);
}

#[derive(Clone, Copy, Debug)]
pub struct OverlayPass {
    pub render_pass: vk::RenderPass,
    pub framebuffer: vk::Framebuffer,
    pub extent: vk::Extent2D,
}

pub struct OverlayDraw<'a> {
    pub descriptor_sets: &'a [vk::DescriptorSet],
    pub dynamic_offsets: &'a [u32],
    pub scissor: vk::Rect2D,
}

pub unsafe fn record_overlay_draws(
    device: &Device,
    cmd: vk::CommandBuffer,
    pass: &OverlayPass,
    render_area: vk::Rect2D,
    load: OverlayAttachmentLoad,
    pipeline: &crate::pipeline::RRPipeline,
    push_constants: Option<&[u8]>,
    draws: &[OverlayDraw],
) -> anyhow::Result<()> {
    begin_overlay_render_pass(
        device,
        cmd,
        pass.render_pass,
        pass.framebuffer,
        render_area,
        load,
    );

    device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, pipeline.pipeline);
    set_full_viewport(device, cmd, pass.extent);

    if let Some(pc) = push_constants {
        device.cmd_push_constants(
            cmd,
            pipeline.pipeline_layout,
            vk::ShaderStageFlags::FRAGMENT,
            0,
            pc,
        );
    }

    for draw in draws {
        device.cmd_set_scissor(cmd, 0, &[draw.scissor]);
        device.cmd_bind_descriptor_sets(
            cmd,
            vk::PipelineBindPoint::GRAPHICS,
            pipeline.pipeline_layout,
            0,
            draw.descriptor_sets,
            draw.dynamic_offsets,
        );
        draw_fullscreen_triangle(device, cmd);
    }

    device.cmd_end_render_pass(cmd);
    Ok(())
}
