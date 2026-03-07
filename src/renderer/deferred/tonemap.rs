use anyhow::{anyhow, Result};
use vulkanalia::prelude::v1_0::*;

use super::OverlayRenderer;
use crate::app::App;
use crate::ecs::resource::{BloomSettings, LensEffects, ToneMapping};
use crate::vulkanr::core::Device;
use crate::vulkanr::descriptor::RRToneMapDescriptorSet;
use crate::vulkanr::pipeline::RRPipeline;

#[repr(C)]
#[derive(Clone, Copy)]
struct ToneMapPushConstants {
    tone_map_operator: i32,
    gamma: f32,
    exposure_value: f32,
    vignette_intensity: f32,
    chromatic_aberration_intensity: f32,
    bloom_intensity: f32,
}

pub struct ToneMapPass<'a> {
    app: &'a App,
    tonemap_pipeline: &'a RRPipeline,
    tonemap_descriptor: &'a RRToneMapDescriptorSet,
    device: &'a Device,
    extent: vk::Extent2D,
}

impl<'a> ToneMapPass<'a> {
    pub fn new(app: &'a App, extent: vk::Extent2D) -> Result<Self> {
        let tonemap_pipeline = app
            .data
            .raytracing
            .tonemap_pipeline
            .as_ref()
            .ok_or_else(|| anyhow!("ToneMap pipeline not initialized"))?;
        let tonemap_descriptor = app
            .data
            .raytracing
            .tonemap_descriptor
            .as_ref()
            .ok_or_else(|| anyhow!("ToneMap descriptor not initialized"))?;

        Ok(Self {
            app,
            tonemap_pipeline,
            tonemap_descriptor,
            device: &app.rrdevice.device,
            extent,
        })
    }

    pub unsafe fn record_to_offscreen(
        &self,
        command_buffer: vk::CommandBuffer,
        render_pass: vk::RenderPass,
        framebuffer: vk::Framebuffer,
        image_index: usize,
    ) -> Result<()> {
        self.begin_render_pass(command_buffer, render_pass, framebuffer);
        self.draw_tonemap(command_buffer)?;
        OverlayRenderer::new(self.app).draw_all_overlays(command_buffer, image_index)?;
        self.device.cmd_end_render_pass(command_buffer);

        Ok(())
    }

    unsafe fn begin_render_pass(
        &self,
        command_buffer: vk::CommandBuffer,
        render_pass: vk::RenderPass,
        framebuffer: vk::Framebuffer,
    ) {
        let render_area = vk::Rect2D::builder()
            .offset(vk::Offset2D::default())
            .extent(self.extent);

        let color_clear_value = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        };
        let depth_clear_value = vk::ClearValue {
            depth_stencil: vk::ClearDepthStencilValue {
                depth: 0.0,
                stencil: 0,
            },
        };
        let resolve_clear_value = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        };

        let clear_values = vec![color_clear_value, depth_clear_value, resolve_clear_value];

        let render_pass_info = vk::RenderPassBeginInfo::builder()
            .render_pass(render_pass)
            .framebuffer(framebuffer)
            .render_area(render_area)
            .clear_values(&clear_values);

        self.device.cmd_begin_render_pass(
            command_buffer,
            &render_pass_info,
            vk::SubpassContents::INLINE,
        );
    }

    unsafe fn draw_tonemap(&self, command_buffer: vk::CommandBuffer) -> Result<()> {
        self.device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.tonemap_pipeline.pipeline,
        );

        let viewport = vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(self.extent.width as f32)
            .height(self.extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0);

        let scissor = vk::Rect2D::builder()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(self.extent);

        self.device.cmd_set_viewport(command_buffer, 0, &[viewport]);
        self.device.cmd_set_scissor(command_buffer, 0, &[scissor]);

        self.device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.tonemap_pipeline.pipeline_layout,
            0,
            &[self.tonemap_descriptor.descriptor_set],
            &[],
        );

        let (operator, gamma) = match self.app.data.ecs_world.get_resource::<ToneMapping>() {
            Some(tm) => {
                let op = if tm.enabled { tm.operator as i32 } else { 0 };
                (op, tm.gamma)
            }
            None => (0, 2.2),
        };

        let exposure_value = self
            .app
            .data
            .ecs_world
            .get_resource::<crate::ecs::resource::Exposure>()
            .map(|e| e.exposure_value)
            .unwrap_or(1.0);

        let (vignette_intensity, ca_intensity) =
            match self.app.data.ecs_world.get_resource::<LensEffects>() {
                Some(le) => {
                    let vi = if le.vignette_enabled {
                        le.vignette_intensity
                    } else {
                        0.0
                    };
                    let ca = if le.chromatic_aberration_enabled {
                        le.chromatic_aberration_intensity
                    } else {
                        0.0
                    };
                    (vi, ca)
                }
                None => (0.0, 0.0),
            };

        let bloom_intensity = self
            .app
            .data
            .ecs_world
            .get_resource::<BloomSettings>()
            .map(|bs| if bs.enabled { bs.intensity } else { 0.0 })
            .unwrap_or(0.0);

        let push_constants = ToneMapPushConstants {
            tone_map_operator: operator,
            gamma,
            exposure_value,
            vignette_intensity,
            chromatic_aberration_intensity: ca_intensity,
            bloom_intensity,
        };

        let push_constant_bytes = std::slice::from_raw_parts(
            &push_constants as *const ToneMapPushConstants as *const u8,
            std::mem::size_of::<ToneMapPushConstants>(),
        );

        self.device.cmd_push_constants(
            command_buffer,
            self.tonemap_pipeline.pipeline_layout,
            vk::ShaderStageFlags::FRAGMENT,
            0,
            push_constant_bytes,
        );

        self.device.cmd_draw(command_buffer, 3, 1, 0, 0);

        Ok(())
    }
}
