use crate::core::RRDevice;
use crate::descriptor::{PassShaders, ReflectedSetLayout};
use crate::pipeline::{
    BlendConfig, DepthTestConfig, PipelineBuilder, PushConstantConfig, RRPipeline,
    VertexInputConfig,
};
use crate::render::RRRender;
use anyhow::{ensure, Result};
use vulkanalia::prelude::v1_0::*;

#[derive(Clone, Copy, Debug)]
pub enum OverlayBlend {
    Premultiplied,
    Opaque,
    Additive,
}

impl OverlayBlend {
    pub fn blend_config(self) -> BlendConfig {
        match self {
            OverlayBlend::Premultiplied => premultiplied_blend(),
            OverlayBlend::Opaque => opaque_blend(),
            OverlayBlend::Additive => additive_blend(),
        }
    }
}

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

fn opaque_blend() -> BlendConfig {
    BlendConfig {
        enable: false,
        src_color_factor: vk::BlendFactor::ONE,
        dst_color_factor: vk::BlendFactor::ZERO,
        color_op: vk::BlendOp::ADD,
        src_alpha_factor: vk::BlendFactor::ONE,
        dst_alpha_factor: vk::BlendFactor::ZERO,
        alpha_op: vk::BlendOp::ADD,
    }
}

fn additive_blend() -> BlendConfig {
    BlendConfig {
        enable: true,
        src_color_factor: vk::BlendFactor::ONE,
        dst_color_factor: vk::BlendFactor::ONE,
        color_op: vk::BlendOp::ADD,
        src_alpha_factor: vk::BlendFactor::ZERO,
        dst_alpha_factor: vk::BlendFactor::ONE,
        alpha_op: vk::BlendOp::ADD,
    }
}

/// What a fullscreen overlay pass finds in its attachments: their previous content, or one
/// clear color for every attachment whose render pass clears it.
#[derive(Clone, Copy, Debug)]
pub enum OverlayAttachmentLoad {
    Keep,
    Clear([f32; 4]),
}

/// The fragment push constant block of an overlay pass, or none.
#[derive(Clone, Copy, Debug)]
pub enum OverlayPushConstants {
    None,
    Fragment { size: u32 },
}

impl OverlayPushConstants {
    pub const fn of<T>() -> Self {
        OverlayPushConstants::Fragment {
            size: std::mem::size_of::<T>() as u32,
        }
    }
}

/// Everything a fullscreen overlay pass declares once: its shaders, one blend per color
/// attachment, the depth test and its push constant block. The pipeline it builds has no
/// vertex input, draws one fullscreen triangle and takes the viewport and scissor dynamically,
/// so it survives a viewport resize.
#[derive(Clone, Copy)]
pub struct OverlayNodeSpec {
    pub shaders: &'static PassShaders,
    pub blends: &'static [OverlayBlend],
    pub depth_test: Option<DepthTestConfig>,
    pub push_constants: OverlayPushConstants,
}

/// One instance drawn by an overlay pass: its dynamic UBO offsets and its screen rectangle.
#[derive(Clone, Debug)]
pub struct OverlayInstanceDraw {
    pub dynamic_offsets: Vec<u32>,
    pub scissor: vk::Rect2D,
}

/// The render target an overlay pass draws into and what it finds there.
#[derive(Clone, Copy, Debug)]
pub struct OverlayPass {
    pub render_pass: vk::RenderPass,
    pub framebuffer: vk::Framebuffer,
    pub extent: vk::Extent2D,
    pub load: OverlayAttachmentLoad,
}

impl OverlayPass {
    pub fn full_area(&self) -> vk::Rect2D {
        vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: self.extent,
        }
    }
}

impl OverlayNodeSpec {
    pub unsafe fn build_pipeline(
        &self,
        rrdevice: &RRDevice,
        rrrender: &RRRender,
        render_pass: vk::RenderPass,
        descriptor_layouts: &[&ReflectedSetLayout],
        extent: vk::Extent2D,
    ) -> Result<RRPipeline> {
        let mut builder = PipelineBuilder::from_pass(self.shaders)
            .vertex_input(VertexInputConfig::Custom {
                bindings: vec![],
                attributes: vec![],
            })
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .custom_render_pass(render_pass)
            .msaa_samples(vk::SampleCountFlags::_1)
            .mrt_attachments(self.blends.len() as u32)
            .descriptor_layouts(descriptor_layouts)
            .dynamic_states(vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);

        builder = match self.depth_test {
            Some(config) => builder.depth_test(config),
            None => builder.no_depth_test(),
        };
        builder = match self.push_constants {
            OverlayPushConstants::None => builder,
            OverlayPushConstants::Fragment { size } => builder.push_constants(PushConstantConfig {
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                offset: 0,
                size,
            }),
        };
        for (attachment_index, blend) in self.blends.iter().enumerate() {
            builder = builder.attachment_blend(attachment_index as u32, blend.blend_config());
        }

        builder.build(rrdevice, rrrender, Some(extent))
    }

    /// Records one render pass: every instance binds the same descriptor sets with its own
    /// dynamic offsets and draws one fullscreen triangle inside its scissor.
    pub unsafe fn record(
        &self,
        device: &Device,
        cmd: vk::CommandBuffer,
        pass: &OverlayPass,
        render_area: vk::Rect2D,
        pipeline: &RRPipeline,
        push_constants: Option<&[u8]>,
        descriptor_sets: &[vk::DescriptorSet],
        draws: &[OverlayInstanceDraw],
    ) -> Result<()> {
        self.ensure_push_constants_match(push_constants)?;
        self.begin_render_pass(device, cmd, pass, render_area);

        device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, pipeline.pipeline);
        set_full_viewport(device, cmd, pass.extent);
        if let Some(bytes) = push_constants {
            device.cmd_push_constants(
                cmd,
                pipeline.pipeline_layout,
                vk::ShaderStageFlags::FRAGMENT,
                0,
                bytes,
            );
        }

        for draw in draws {
            device.cmd_set_scissor(cmd, 0, &[draw.scissor]);
            device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.pipeline_layout,
                0,
                descriptor_sets,
                &draw.dynamic_offsets,
            );
            draw_fullscreen_triangle(device, cmd);
        }

        device.cmd_end_render_pass(cmd);
        Ok(())
    }

    unsafe fn begin_render_pass(
        &self,
        device: &Device,
        cmd: vk::CommandBuffer,
        pass: &OverlayPass,
        render_area: vk::Rect2D,
    ) {
        let clear_values = match pass.load {
            OverlayAttachmentLoad::Keep => vec![],
            OverlayAttachmentLoad::Clear(color) => vec![
                vk::ClearValue {
                    color: vk::ClearColorValue { float32: color },
                };
                self.blends.len()
            ],
        };
        let render_pass_info = vk::RenderPassBeginInfo::builder()
            .render_pass(pass.render_pass)
            .framebuffer(pass.framebuffer)
            .render_area(render_area)
            .clear_values(&clear_values);
        device.cmd_begin_render_pass(cmd, &render_pass_info, vk::SubpassContents::INLINE);
    }

    fn ensure_push_constants_match(&self, push_constants: Option<&[u8]>) -> Result<()> {
        match (self.push_constants, push_constants) {
            (OverlayPushConstants::None, None) => Ok(()),
            (OverlayPushConstants::Fragment { size }, Some(bytes)) => {
                ensure!(
                    bytes.len() == size as usize,
                    "overlay pass `{}` declares {} push constant bytes, got {}",
                    self.shaders.name(),
                    size,
                    bytes.len()
                );
                Ok(())
            }
            (OverlayPushConstants::None, Some(_)) => anyhow::bail!(
                "overlay pass `{}` declares no push constants",
                self.shaders.name()
            ),
            (OverlayPushConstants::Fragment { .. }, None) => anyhow::bail!(
                "overlay pass `{}` needs its push constants",
                self.shaders.name()
            ),
        }
    }
}

unsafe fn set_full_viewport(device: &Device, cmd: vk::CommandBuffer, extent: vk::Extent2D) {
    let viewport = vk::Viewport::builder()
        .x(0.0)
        .y(0.0)
        .width(extent.width as f32)
        .height(extent.height as f32)
        .min_depth(0.0)
        .max_depth(1.0);
    device.cmd_set_viewport(cmd, 0, &[viewport]);
}

unsafe fn draw_fullscreen_triangle(device: &Device, cmd: vk::CommandBuffer) {
    device.cmd_draw(cmd, 3, 1, 0, 0);
}
