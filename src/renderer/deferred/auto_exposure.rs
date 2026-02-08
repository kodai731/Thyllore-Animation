use anyhow::{anyhow, Result};
use vulkanalia::prelude::v1_0::*;

use crate::app::App;
use crate::ecs::resource::AutoExposure;
use crate::vulkanr::core::Device;
use crate::vulkanr::descriptor::{
    RRAutoExposureAverageDescriptorSet,
    RRAutoExposureHistogramDescriptorSet,
};
use crate::vulkanr::pipeline::RRPipeline;
use crate::vulkanr::resource::AutoExposureBuffers;

#[repr(C)]
#[derive(Clone, Copy)]
struct HistogramPushConstants {
    min_log_luminance: f32,
    log_luminance_range: f32,
    pixel_count: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct AveragePushConstants {
    low_percent: f32,
    high_percent: f32,
    time_delta: f32,
    adaptation_speed_up: f32,
    adaptation_speed_down: f32,
    min_ev: f32,
    max_ev: f32,
    min_log_luminance: f32,
    log_luminance_range: f32,
    pixel_count: u32,
}

pub struct AutoExposurePass<'a> {
    histogram_pipeline: &'a RRPipeline,
    average_pipeline: &'a RRPipeline,
    histogram_descriptor: &'a RRAutoExposureHistogramDescriptorSet,
    average_descriptor: &'a RRAutoExposureAverageDescriptorSet,
    buffers: &'a AutoExposureBuffers,
    histogram_push: HistogramPushConstants,
    average_push: AveragePushConstants,
    device: &'a Device,
}

impl<'a> AutoExposurePass<'a> {
    pub fn new(app: &'a App) -> Result<Self> {
        let histogram_pipeline = app
            .data
            .raytracing
            .auto_exposure_histogram_pipeline
            .as_ref()
            .ok_or_else(|| {
                anyhow!(
                    "AutoExposure histogram pipeline not initialized"
                )
            })?;

        let average_pipeline = app
            .data
            .raytracing
            .auto_exposure_average_pipeline
            .as_ref()
            .ok_or_else(|| {
                anyhow!(
                    "AutoExposure average pipeline not initialized"
                )
            })?;

        let histogram_descriptor = app
            .data
            .raytracing
            .auto_exposure_histogram_descriptor
            .as_ref()
            .ok_or_else(|| {
                anyhow!(
                    "AutoExposure histogram descriptor not initialized"
                )
            })?;

        let average_descriptor = app
            .data
            .raytracing
            .auto_exposure_average_descriptor
            .as_ref()
            .ok_or_else(|| {
                anyhow!(
                    "AutoExposure average descriptor not initialized"
                )
            })?;

        let buffers = app
            .data
            .viewport
            .auto_exposure_buffers
            .as_ref()
            .ok_or_else(|| {
                anyhow!("AutoExposure buffers not initialized")
            })?;

        let ae_settings = app
            .data
            .ecs_world
            .get_resource::<AutoExposure>();
        let (
            min_log_luminance,
            log_luminance_range,
            low_percent,
            high_percent,
            adaptation_speed_up,
            adaptation_speed_down,
            min_ev,
            max_ev,
        ) = match ae_settings {
            Some(ae) => (
                ae.min_log_luminance,
                ae.log_luminance_range,
                ae.low_percent,
                ae.high_percent,
                ae.adaptation_speed_up,
                ae.adaptation_speed_down,
                ae.min_ev,
                ae.max_ev,
            ),
            None => (-10.0, 22.0, 0.1, 0.9, 3.0, 1.0, -4.0, 16.0),
        };

        let time = app
            .data
            .ecs_world
            .get_resource::<crate::ecs::resource::TimelineState>();
        let delta_time = time
            .map(|t| 1.0 / 60.0 * t.speed.max(0.01))
            .unwrap_or(1.0 / 60.0);

        let pixel_count = buffers.width * buffers.height;

        let histogram_push = HistogramPushConstants {
            min_log_luminance,
            log_luminance_range,
            pixel_count,
        };

        let average_push = AveragePushConstants {
            low_percent,
            high_percent,
            time_delta: delta_time,
            adaptation_speed_up,
            adaptation_speed_down,
            min_ev,
            max_ev,
            min_log_luminance,
            log_luminance_range,
            pixel_count,
        };

        Ok(Self {
            histogram_pipeline,
            average_pipeline,
            histogram_descriptor,
            average_descriptor,
            buffers,
            histogram_push,
            average_push,
            device: &app.rrdevice.device,
        })
    }

    pub unsafe fn record(
        &self,
        command_buffer: vk::CommandBuffer,
    ) -> Result<()> {
        self.insert_pre_histogram_barrier(command_buffer);
        self.dispatch_histogram(command_buffer);
        self.insert_histogram_to_average_barrier(command_buffer);
        self.dispatch_average(command_buffer);
        self.insert_post_average_barrier(command_buffer);

        Ok(())
    }

    unsafe fn insert_pre_histogram_barrier(
        &self,
        command_buffer: vk::CommandBuffer,
    ) {
        let barrier = vk::MemoryBarrier::builder()
            .src_access_mask(vk::AccessFlags::SHADER_WRITE)
            .dst_access_mask(vk::AccessFlags::SHADER_WRITE)
            .build();

        self.device.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::DependencyFlags::empty(),
            &[barrier],
            &[] as &[vk::BufferMemoryBarrier],
            &[] as &[vk::ImageMemoryBarrier],
        );
    }

    unsafe fn dispatch_histogram(
        &self,
        command_buffer: vk::CommandBuffer,
    ) {
        self.device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::COMPUTE,
            self.histogram_pipeline.pipeline,
        );

        self.device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::COMPUTE,
            self.histogram_pipeline.pipeline_layout,
            0,
            &[self.histogram_descriptor.descriptor_set],
            &[],
        );

        let push_bytes = std::slice::from_raw_parts(
            &self.histogram_push
                as *const HistogramPushConstants
                as *const u8,
            std::mem::size_of::<HistogramPushConstants>(),
        );

        self.device.cmd_push_constants(
            command_buffer,
            self.histogram_pipeline.pipeline_layout,
            vk::ShaderStageFlags::COMPUTE,
            0,
            push_bytes,
        );

        let group_count_x = (self.buffers.width + 15) / 16;
        let group_count_y = (self.buffers.height + 15) / 16;
        self.device.cmd_dispatch(
            command_buffer,
            group_count_x,
            group_count_y,
            1,
        );
    }

    unsafe fn insert_histogram_to_average_barrier(
        &self,
        command_buffer: vk::CommandBuffer,
    ) {
        let barrier = vk::MemoryBarrier::builder()
            .src_access_mask(vk::AccessFlags::SHADER_WRITE)
            .dst_access_mask(vk::AccessFlags::SHADER_READ)
            .build();

        self.device.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::DependencyFlags::empty(),
            &[barrier],
            &[] as &[vk::BufferMemoryBarrier],
            &[] as &[vk::ImageMemoryBarrier],
        );
    }

    unsafe fn dispatch_average(
        &self,
        command_buffer: vk::CommandBuffer,
    ) {
        self.device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::COMPUTE,
            self.average_pipeline.pipeline,
        );

        self.device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::COMPUTE,
            self.average_pipeline.pipeline_layout,
            0,
            &[self.average_descriptor.descriptor_set],
            &[],
        );

        let push_bytes = std::slice::from_raw_parts(
            &self.average_push
                as *const AveragePushConstants
                as *const u8,
            std::mem::size_of::<AveragePushConstants>(),
        );

        self.device.cmd_push_constants(
            command_buffer,
            self.average_pipeline.pipeline_layout,
            vk::ShaderStageFlags::COMPUTE,
            0,
            push_bytes,
        );

        self.device.cmd_dispatch(command_buffer, 1, 1, 1);
    }

    unsafe fn insert_post_average_barrier(
        &self,
        command_buffer: vk::CommandBuffer,
    ) {
        let barrier = vk::MemoryBarrier::builder()
            .src_access_mask(vk::AccessFlags::SHADER_WRITE)
            .dst_access_mask(vk::AccessFlags::HOST_READ)
            .build();

        self.device.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::HOST,
            vk::DependencyFlags::empty(),
            &[barrier],
            &[] as &[vk::BufferMemoryBarrier],
            &[] as &[vk::ImageMemoryBarrier],
        );
    }
}
