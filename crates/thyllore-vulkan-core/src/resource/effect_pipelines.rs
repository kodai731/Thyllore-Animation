use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::core::device::RRDevice;
use crate::descriptor::{
    FlameImageBindings, RRFlameDescriptorSet, RRWaterCausticDescriptorSet, RRWaterDescriptorSet,
    RRWaterTraceDescriptorSet, FLAME_RESOLVE, WATER_CAUSTIC_APPLY, WATER_CAUSTIC_SPLAT,
    WATER_RESOLVE, WATER_TRACE,
};
use crate::pipeline::{
    BlendConfig, DepthTestConfig, PipelineBuilder, PushConstantConfig, RRPipeline,
    RRRayTracingPipeline, VertexInputConfig,
};
use crate::render::RRRender;
use crate::resource::graphics_resource::GraphicsResources;
use crate::resource::raytracing_data::{MAX_FLAME_INSTANCES, MAX_WATER_INSTANCES};
use crate::resource::uniform_buffer::{Placement, UniformBuffer};
use crate::resource::{FlameBuffer, HdrBuffer, RayTracingData, WaterBuffer};
use thyllore_effect_core::{FlameUBO, WaterUBO};

impl RayTracingData {
    pub unsafe fn create_flame_pipeline(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        rrrender: &RRRender,
        graphics_resources: &GraphicsResources,
        flame_buffer: &FlameBuffer,
        position_image_view: vk::ImageView,
        position_sampler: vk::Sampler,
        scene_depth_view: vk::ImageView,
    ) -> Result<()> {
        let flame_ubo = UniformBuffer::new(
            instance,
            rrdevice,
            MAX_FLAME_INSTANCES,
            Placement::DeviceUpdated,
        )?;
        flame_ubo.write_slot(rrdevice, 0, &FlameUBO::default())?;

        let flame_descriptor = RRFlameDescriptorSet::new(rrdevice)?;
        flame_descriptor.write_all(
            rrdevice,
            &flame_ubo,
            FlameImageBindings {
                history_image_views: flame_buffer.history_image_views,
                flame_sampler: flame_buffer.sampler,
                sdf_image_view: position_image_view,
                sdf_sampler: position_sampler,
                scene_depth_view,
            },
        )?;

        let flame_shading_pipeline = PipelineBuilder::from_pass(&FLAME_RESOLVE)
            .vertex_input(VertexInputConfig::Custom {
                bindings: vec![],
                attributes: vec![],
            })
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .no_depth_test()
            .custom_render_pass(flame_buffer.shading_render_pass)
            .msaa_samples(vk::SampleCountFlags::_1)
            .mrt_attachments(2)
            .blend(BlendConfig {
                enable: true,
                src_color_factor: vk::BlendFactor::ONE,
                dst_color_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
                color_op: vk::BlendOp::ADD,
                src_alpha_factor: vk::BlendFactor::ONE,
                dst_alpha_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
                alpha_op: vk::BlendOp::ADD,
            })
            .attachment_blend(
                1,
                BlendConfig {
                    enable: false,
                    src_color_factor: vk::BlendFactor::ONE,
                    dst_color_factor: vk::BlendFactor::ZERO,
                    color_op: vk::BlendOp::ADD,
                    src_alpha_factor: vk::BlendFactor::ONE,
                    dst_alpha_factor: vk::BlendFactor::ZERO,
                    alpha_op: vk::BlendOp::ADD,
                },
            )
            .push_constants(PushConstantConfig {
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                offset: 0,
                size: std::mem::size_of::<crate::renderer::FlamePushConstants>() as u32,
            })
            .dynamic_states(vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR])
            .descriptor_layouts(&[
                &graphics_resources.frame_set.layout,
                &flame_descriptor.layout,
            ])
            .build(rrdevice, rrrender, Some(flame_buffer.extent()))?;

        self.flame_shading_pipeline = Some(flame_shading_pipeline);
        self.flame_descriptor = Some(flame_descriptor);
        self.flame_ubo = Some(flame_ubo);

        log!("Created flame pipelines");
        Ok(())
    }

    pub unsafe fn create_water_pipeline(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        rrrender: &RRRender,
        graphics_resources: &GraphicsResources,
        water_buffer: &WaterBuffer,
        hdr_buffer: &HdrBuffer,
        frames_in_flight: usize,
    ) -> Result<()> {
        let water_ubo = UniformBuffer::new(
            instance,
            rrdevice,
            MAX_WATER_INSTANCES,
            Placement::DeviceUpdated,
        )?;
        water_ubo.write_slot(rrdevice, 0, &WaterUBO::default())?;

        let water_descriptor = RRWaterDescriptorSet::new(rrdevice, frames_in_flight)?;

        let water_shading_pipeline = PipelineBuilder::from_pass(&WATER_RESOLVE)
            .vertex_input(VertexInputConfig::Custom {
                bindings: vec![],
                attributes: vec![],
            })
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .depth_test(DepthTestConfig {
                test_enable: true,
                write_enable: true,
                compare_op: vk::CompareOp::GREATER_OR_EQUAL,
            })
            .custom_render_pass(water_buffer.render_pass)
            .msaa_samples(vk::SampleCountFlags::_1)
            .mrt_attachments(2)
            .blend(BlendConfig {
                enable: false,
                src_color_factor: vk::BlendFactor::ONE,
                dst_color_factor: vk::BlendFactor::ZERO,
                color_op: vk::BlendOp::ADD,
                src_alpha_factor: vk::BlendFactor::ONE,
                dst_alpha_factor: vk::BlendFactor::ZERO,
                alpha_op: vk::BlendOp::ADD,
            })
            .push_constants(PushConstantConfig {
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                offset: 0,
                size: std::mem::size_of::<crate::renderer::WaterPushConstants>() as u32,
            })
            .dynamic_states(vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR])
            .descriptor_layouts(&[
                &graphics_resources.frame_set.layout,
                &water_descriptor.layout,
            ])
            .build(rrdevice, rrrender, Some(water_buffer.extent()))?;

        self.water_shading_pipeline = Some(water_shading_pipeline);
        self.water_descriptor = Some(water_descriptor);

        let water_trace_descriptor = RRWaterTraceDescriptorSet::new(rrdevice, frames_in_flight)?;

        self.water_ubo = Some(water_ubo);
        let intersection_range = vk::PushConstantRange::builder()
            .stage_flags(vk::ShaderStageFlags::INTERSECTION_KHR)
            .offset(0)
            .size(8)
            .build();
        let raygen_range = vk::PushConstantRange::builder()
            .stage_flags(vk::ShaderStageFlags::RAYGEN_KHR)
            .offset(16)
            .size(112)
            .build();
        let closest_hit_range = vk::PushConstantRange::builder()
            .stage_flags(vk::ShaderStageFlags::CLOSEST_HIT_KHR)
            .offset(96)
            .size(32)
            .build();
        let water_trace_pipeline = RRRayTracingPipeline::new(
            instance,
            rrdevice,
            &WATER_TRACE,
            &[water_trace_descriptor.layout.handle],
            &[intersection_range, raygen_range, closest_hit_range],
        )?;

        self.water_trace_descriptor = Some(water_trace_descriptor);
        self.water_trace_pipeline = Some(water_trace_pipeline);

        self.create_water_caustic_pipelines(rrdevice, water_buffer, hdr_buffer)?;

        log!("Created water trace pipeline");
        Ok(())
    }
    /// Caustic splat/apply need the water UBO and the water buffer, so they are built
    /// once the water pipeline has produced them; the TLAS is bound later if missing.
    unsafe fn create_water_caustic_pipelines(
        &mut self,
        rrdevice: &RRDevice,
        water_buffer: &WaterBuffer,
        hdr_buffer: &HdrBuffer,
    ) -> Result<()> {
        let (Some(gbuffer), Some(scene_buffer), Some(water_ubo)) = (
            self.gbuffer.as_ref(),
            self.scene_uniform_buffer,
            self.water_ubo.as_ref(),
        ) else {
            log!("Water caustic inputs are not ready, skipping caustic pipelines");
            return Ok(());
        };
        let tlas = self
            .acceleration_structure
            .as_ref()
            .and_then(|accel| accel.tlas.acceleration_structure);

        let mut descriptor = RRWaterCausticDescriptorSet::new(rrdevice)?;

        descriptor.allocate_and_update(
            rrdevice,
            water_buffer.caustic_accum_view,
            gbuffer.position_image_view,
            tlas,
            scene_buffer,
            water_ubo.handle(),
            hdr_buffer.color_image_view,
        )?;

        let splat_pipeline =
            RRPipeline::new_compute(rrdevice, &WATER_CAUSTIC_SPLAT, &[&descriptor.splat_layout])?;
        let apply_pipeline =
            RRPipeline::new_compute(rrdevice, &WATER_CAUSTIC_APPLY, &[&descriptor.apply_layout])?;

        self.water_caustic_splat_pipeline = Some(splat_pipeline);
        self.water_caustic_apply_pipeline = Some(apply_pipeline);
        self.water_caustic_descriptor = Some(descriptor);

        log!("Created water caustic pipelines");
        Ok(())
    }
}
