use anyhow::{anyhow, Result};
use vulkanalia::prelude::v1_0::*;

use crate::app::App;
use crate::ecs::resource::{DepthOfField, PhysicalCameraParameters};
use crate::vulkanr::core::Device;
use crate::vulkanr::descriptor::RRDofDescriptorSet;
use crate::vulkanr::pipeline::RRPipeline;
use crate::vulkanr::resource::DofBuffer;

#[repr(C)]
#[derive(Clone, Copy)]
struct DofPushConstants {
    focal_length_mm: f32,
    aperture_f_stops: f32,
    sensor_height_mm: f32,
    focus_distance: f32,
    near_plane: f32,
    max_blur_radius: f32,
    viewport_height: f32,
    enabled: i32,
}

pub struct DofPass<'a> {
    pipeline: &'a RRPipeline,
    dof_descriptor: &'a RRDofDescriptorSet,
    dof_buffer: &'a DofBuffer,
    push_constants: DofPushConstants,
    device: &'a Device,
}

impl<'a> DofPass<'a> {
    pub fn new(app: &'a App) -> Result<Self> {
        let pipeline = app
            .data
            .raytracing
            .dof_pipeline
            .as_ref()
            .ok_or_else(|| anyhow!("DOF pipeline not initialized"))?;

        let dof_descriptor = app
            .data
            .raytracing
            .dof_descriptor
            .as_ref()
            .ok_or_else(|| anyhow!("DOF descriptor not initialized"))?;

        let dof_buffer = app
            .data
            .viewport
            .dof_buffer
            .as_ref()
            .ok_or_else(|| anyhow!("DOF buffer not initialized"))?;

        let dof_settings = app.data.ecs_world.get_resource::<DepthOfField>();
        let camera_params = app.data.ecs_world.get_resource::<PhysicalCameraParameters>();
        let camera = app.camera();

        let (enabled, focus_distance, max_blur_radius) = match dof_settings {
            Some(dof) => (dof.enabled, dof.focus_distance, dof.max_blur_radius),
            None => (false, 10.0, 8.0),
        };

        let (focal_length_mm, aperture_f_stops, sensor_height_mm) = match camera_params {
            Some(params) => (
                params.focal_length_mm,
                params.aperture_f_stops,
                params.sensor_height_mm,
            ),
            None => (35.0, 16.0, 18.66),
        };

        let push_constants = DofPushConstants {
            focal_length_mm,
            aperture_f_stops,
            sensor_height_mm,
            focus_distance,
            near_plane: camera.near_plane,
            max_blur_radius,
            viewport_height: dof_buffer.height as f32,
            enabled: if enabled { 1 } else { 0 },
        };

        Ok(Self {
            pipeline,
            dof_descriptor,
            dof_buffer,
            push_constants,
            device: &app.rrdevice.device,
        })
    }

    pub unsafe fn record(&self, command_buffer: vk::CommandBuffer) -> Result<()> {
        let extent = self.dof_buffer.extent();

        self.begin_render_pass(command_buffer, extent);

        self.device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.pipeline.pipeline,
        );

        self.set_viewport_and_scissor(command_buffer, extent);

        self.device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.pipeline.pipeline_layout,
            0,
            &[self.dof_descriptor.descriptor_set],
            &[],
        );

        let push_bytes = std::slice::from_raw_parts(
            &self.push_constants as *const DofPushConstants as *const u8,
            std::mem::size_of::<DofPushConstants>(),
        );

        self.device.cmd_push_constants(
            command_buffer,
            self.pipeline.pipeline_layout,
            vk::ShaderStageFlags::FRAGMENT,
            0,
            push_bytes,
        );

        self.device.cmd_draw(command_buffer, 3, 1, 0, 0);

        self.device.cmd_end_render_pass(command_buffer);

        Ok(())
    }

    unsafe fn begin_render_pass(
        &self,
        command_buffer: vk::CommandBuffer,
        extent: vk::Extent2D,
    ) {
        let clear_value = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        };

        let render_pass_info = vk::RenderPassBeginInfo::builder()
            .render_pass(self.dof_buffer.render_pass)
            .framebuffer(self.dof_buffer.framebuffer)
            .render_area(vk::Rect2D {
                offset: vk::Offset2D::default(),
                extent,
            })
            .clear_values(std::slice::from_ref(&clear_value));

        self.device.cmd_begin_render_pass(
            command_buffer,
            &render_pass_info,
            vk::SubpassContents::INLINE,
        );
    }

    unsafe fn set_viewport_and_scissor(
        &self,
        command_buffer: vk::CommandBuffer,
        extent: vk::Extent2D,
    ) {
        let viewport = vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(extent.width as f32)
            .height(extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0);

        let scissor = vk::Rect2D::builder()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(extent);

        self.device.cmd_set_viewport(command_buffer, 0, &[viewport]);
        self.device.cmd_set_scissor(command_buffer, 0, &[scissor]);
    }
}
