use anyhow::Result;
use cgmath::SquareMatrix;
use vulkanalia::prelude::v1_0::*;
use vulkanalia::vk::KhrRayTracingPipelineExtension;

use crate::ecs::component::WaterTorusEffect;
use crate::ecs::resource::{
    EffectTraceGpuState, WaterBindingKey, WaterGpuState, WaterRenderTargets,
};
use crate::ecs::world::Entity;
use crate::ecs::PassContext;
use crate::hooks::pass::{
    CoreTarget, PassStage, RenderPassNode, ShaderStage, TargetAccess, TargetRef, TargetUse,
    TransientRequest, TransientSlot,
};
use crate::vulkanr::renderer::deferred::full_extent_scissor;
use thyllore_vulkan_core::descriptor::shader_bindings::effect_trace;
use thyllore_vulkan_core::descriptor::{push_constant_range, GpuBlock};
use thyllore_vulkan_core::renderer::TracePush;
use thyllore_vulkan_core::resource::RenderTargetKey;

/// Pass nodes in record order. Subscription order inside the effect stage is this order.
pub const WATER_PASS_NODES: &[&dyn RenderPassNode] = &[
    &WaterFrameNode,
    &WaterTraceNode,
    &WaterCausticClearNode,
    &WaterCausticSplatNode,
    &WaterCausticApplyNode,
    &WaterSceneColorCopyNode,
    &WaterHistoryClearNode,
    &WaterShadingNode,
];

const HDR_COLOR: TargetRef = TargetRef::Core(CoreTarget::HdrColor);
const SCENE_COLOR_SLOT: TransientSlot = TransientSlot("water.scene_color");
const TRACE_SLOT: TransientSlot = TransientSlot("water.trace");
const SCENE_COLOR: TargetRef = TargetRef::Transient(SCENE_COLOR_SLOT);
const TRACE: TargetRef = TargetRef::Transient(TRACE_SLOT);
const CAUSTIC_ACCUM: TargetRef = TargetRef::Storage(RenderTargetKey::EffectAccumulation(0));
const HISTORY_KEYS: [RenderTargetKey; 2] = [
    RenderTargetKey::EffectHistory(2),
    RenderTargetKey::EffectHistory(3),
];

/// Must match CAUSTIC_GRID_SIZE and local_size in causticSplat.comp
const CAUSTIC_GRID_SIZE: u32 = 512;
const CAUSTIC_WORKGROUP_SIZE: u32 = 16;

const COLOR_SUBRESOURCE_RANGE: vk::ImageSubresourceRange = vk::ImageSubresourceRange {
    aspect_mask: vk::ImageAspectFlags::COLOR,
    base_mip_level: 0,
    level_count: 1,
    base_array_layer: 0,
    layer_count: 1,
};

/// Everything the water nodes agree on for one frame. `None` means no water pass records this frame.
struct WaterFrame {
    waters: Vec<Entity>,
    scissors: Vec<Option<vk::Rect2D>>,
    history_index: usize,
    settings: crate::ecs::resource::WaterRenderSettings,
}

fn water_frame(ctx: &PassContext) -> Option<WaterFrame> {
    ctx.world.get_resource::<WaterRenderTargets>()?;
    let gpu_state = ctx.world.get_resource::<WaterGpuState>()?;
    gpu_state.shading_pipeline.as_ref()?;
    gpu_state.descriptor.as_ref()?;
    gpu_state.ubo.as_ref()?;
    ctx.hdr_buffer?;
    water_scene_bindings(ctx)?;

    let mut waters: Vec<Entity> = ctx.world.entities_with::<WaterTorusEffect>();
    waters.truncate(thyllore_effect_core::WATER_MAX_INSTANCES);
    if waters.is_empty() {
        return None;
    }

    let history_index = first_water_accum(ctx, &waters)
        .map(|accum| (accum.frame_index & 1) as usize)
        .unwrap_or(0);
    let settings = ctx
        .world
        .get_resource::<crate::ecs::resource::WaterRenderSettings>()
        .map(|settings| *settings)
        .unwrap_or_default();

    let scissors = instance_scissors(ctx, &waters);

    Some(WaterFrame {
        waters,
        scissors,
        history_index,
        settings,
    })
}

/// Screen rectangle of every instance, `None` when the instance is off-screen. Declarations and
/// recording share this so the graph only expects the shading render pass when it really records.
fn instance_scissors(ctx: &PassContext, waters: &[Entity]) -> Vec<Option<vk::Rect2D>> {
    let Some(extent) = ctx
        .world
        .get_resource::<WaterRenderTargets>()
        .map(|targets| targets.buffer.extent())
    else {
        return vec![None; waters.len()];
    };
    waters
        .iter()
        .map(|water| {
            let effect = ctx
                .world
                .get_component::<crate::ecs::component::WaterTorusEffect>(*water)?;
            let frame_index = ctx
                .world
                .get_component::<crate::ecs::component::WaterTemporalAccum>(*water)
                .map(|accum| accum.frame_index as u32)
                .unwrap_or(0);
            let model = thyllore_effect_core::build_water_ubo(&effect, frame_index).model;
            compute_water_scissor(
                ctx,
                extent,
                &model,
                effect.major_radius,
                effect.minor_radius,
            )
        })
        .collect()
}

fn water_scene_bindings(ctx: &PassContext) -> Option<(vk::AccelerationStructureKHR, vk::Buffer)> {
    if !ctx.raytracing.has_valid_tlas() {
        return None;
    }
    let accel = ctx.raytracing.acceleration_structure.as_ref()?;
    let tlas = accel.tlas.acceleration_structure?;
    let hit_table = accel.hit_shading_table.as_ref()?.buffer;
    Some((tlas, hit_table))
}

fn first_water_accum(
    ctx: &PassContext,
    waters: &[Entity],
) -> Option<crate::ecs::component::WaterTemporalAccum> {
    waters.first().and_then(|water| {
        ctx.world
            .get_component::<crate::ecs::component::WaterTemporalAccum>(*water)
            .cloned()
    })
}

impl WaterFrame {
    /// The requested secondary ray mode, or ray query when the shared trace pipeline is unavailable.
    fn secondary_rays(&self, ctx: &PassContext) -> thyllore_effect_core::WaterSecondaryRays {
        let trace_available = ctx
            .world
            .get_resource::<EffectTraceGpuState>()
            .is_some_and(|state| state.pipeline.is_some() && state.descriptor.is_some());
        match self.settings.secondary_rays {
            thyllore_effect_core::WaterSecondaryRays::RayTracingPipeline if !trace_available => {
                thyllore_effect_core::WaterSecondaryRays::RayQuery
            }
            requested => requested,
        }
    }

    fn is_trace_enabled(&self, ctx: &PassContext) -> bool {
        self.secondary_rays(ctx) == thyllore_effect_core::WaterSecondaryRays::RayTracingPipeline
            && ctx
                .world
                .get_component::<crate::ecs::component::WaterTorusEffect>(self.waters[0])
                .is_some()
    }

    fn is_caustic_enabled(&self, ctx: &PassContext) -> bool {
        let caustic_strength = ctx
            .world
            .get_component::<crate::ecs::component::WaterTorusEffect>(self.waters[0])
            .map(|effect| effect.caustic_strength)
            .unwrap_or(0.0);
        let Some(gpu_state) = ctx.world.get_resource::<WaterGpuState>() else {
            return false;
        };
        caustic_strength > 0.0
            && gpu_state.caustic_splat_pipeline.is_some()
            && gpu_state.caustic_apply_pipeline.is_some()
            && gpu_state
                .caustic_descriptor
                .as_ref()
                .is_some_and(|descriptor| {
                    descriptor.splat_descriptor_set != vk::DescriptorSet::null()
                })
    }

    fn has_visible_instance(&self) -> bool {
        self.scissors.iter().any(Option::is_some)
    }

    fn is_history_invalidated(&self, ctx: &PassContext) -> bool {
        first_water_accum(ctx, &self.waters).is_some_and(|accum| accum.history_invalidated)
    }

    fn written_history(&self) -> TargetRef {
        TargetRef::Storage(HISTORY_KEYS[self.history_index])
    }

    fn read_history(&self) -> TargetRef {
        TargetRef::Storage(HISTORY_KEYS[1 - self.history_index])
    }
}

fn frame_uses(
    ctx: &PassContext,
    select: impl FnOnce(&WaterFrame, &PassContext) -> Vec<TargetUse>,
) -> Vec<TargetUse> {
    water_frame(ctx)
        .map(|frame| select(&frame, ctx))
        .unwrap_or_default()
}

pub struct WaterTraceNode;

impl RenderPassNode for WaterTraceNode {
    fn name(&self) -> &'static str {
        "water_trace"
    }

    fn stage(&self) -> PassStage {
        PassStage::Effect
    }

    fn writes(&self, ctx: &PassContext) -> Vec<TargetUse> {
        frame_uses(ctx, |frame, ctx| {
            if !frame.is_trace_enabled(ctx) {
                return Vec::new();
            }
            vec![TargetUse::new(
                TRACE,
                TargetAccess::StorageReadWrite(ShaderStage::RayTracing),
            )]
        })
    }

    unsafe fn record(
        &self,
        ctx: &PassContext,
        command_buffer: vk::CommandBuffer,
        image_index: usize,
        frame_slot: usize,
    ) -> Result<()> {
        let Some(frame) = water_frame(ctx).filter(|frame| frame.is_trace_enabled(ctx)) else {
            return Ok(());
        };
        let Some(trace_state) = ctx.world.get_resource::<EffectTraceGpuState>() else {
            return Ok(());
        };
        let (Some(trace_pipeline), Some(trace_descriptor)) = (
            trace_state.pipeline.as_ref(),
            trace_state.descriptor.as_ref(),
        ) else {
            return Ok(());
        };
        let Some(targets) = ctx.world.get_resource::<WaterRenderTargets>() else {
            return Ok(());
        };
        let water_buffer = &targets.buffer;
        let render = ctx.frame_render_context(image_index);

        let device = &render.device.device;
        device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::RAY_TRACING_KHR,
            trace_pipeline.pipeline,
        );
        device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::RAY_TRACING_KHR,
            trace_pipeline.pipeline_layout,
            0,
            &[trace_descriptor.descriptor_set(frame_slot)?],
            &[],
        );
        let projection = ctx.world.resource::<crate::ecs::resource::ProjectionData>();
        let inv_view_proj = crate::ecs::systems::water::probe::inverse_view_proj_f64(
            projection.proj,
            projection.view,
        );
        let view_inverse = projection
            .view
            .invert()
            .unwrap_or_else(cgmath::Matrix4::identity);
        let light_position = ctx
            .world
            .resource::<crate::ecs::resource::LightState>()
            .light_position;
        let trace_push = TracePush {
            inv_view_proj,
            camera_pos: [
                view_inverse[3][0],
                view_inverse[3][1],
                view_inverse[3][2],
                1.0,
            ],
            light_pos: [light_position.x, light_position.y, light_position.z, 1.0],
            light_color: [1.0; 4],
        };
        let push_range = push_constant_range(&effect_trace::PUSH_CONSTANT);
        device.cmd_push_constants(
            command_buffer,
            trace_pipeline.pipeline_layout,
            push_range.stage_flags,
            push_range.offset,
            trace_push.as_bytes(),
        );
        let extent = water_buffer.extent();
        device.cmd_trace_rays_khr(
            command_buffer,
            &trace_pipeline.raygen_region,
            &trace_pipeline.miss_region,
            &trace_pipeline.hit_region,
            &trace_pipeline.callable_region,
            extent.width,
            extent.height,
            1,
        );
        Ok(())
    }
}

/// Borrows the frame's scene color / trace images, rebinds the water descriptors of this frame slot,
/// and uploads the per-instance UBOs before any water pass reads them.
pub struct WaterFrameNode;

impl RenderPassNode for WaterFrameNode {
    fn name(&self) -> &'static str {
        "water_frame"
    }

    fn stage(&self) -> PassStage {
        PassStage::Effect
    }

    fn transients(&self, ctx: &PassContext) -> Vec<TransientRequest> {
        if water_frame(ctx).is_none() {
            return Vec::new();
        }
        let Some(targets) = ctx.world.get_resource::<WaterRenderTargets>() else {
            return Vec::new();
        };
        vec![
            TransientRequest::new(SCENE_COLOR_SLOT, targets.buffer.scene_color_desc()),
            TransientRequest::new(TRACE_SLOT, targets.buffer.trace_desc()),
        ]
    }

    unsafe fn prepare(&self, ctx: &mut PassContext, frame_slot: usize) -> Result<()> {
        if water_frame(ctx).is_none() {
            return Ok(());
        }
        let Some((tlas, hit_table)) = water_scene_bindings(ctx) else {
            return Ok(());
        };
        let scene_color_image = ctx.transient_image(SCENE_COLOR_SLOT)?;
        let trace_image = ctx.transient_image(TRACE_SLOT)?;
        let Some(mut targets) = ctx.world.get_resource_mut::<WaterRenderTargets>() else {
            return Ok(());
        };

        let history_views = targets.buffer.history_image_views;
        let history_sampler = targets.buffer.history_sampler;
        let key = WaterBindingKey {
            tlas,
            hit_table,
            history_views,
            scene_color_generation: scene_color_image.generation,
            trace_generation: trace_image.generation,
        };
        if targets.is_bound(frame_slot, key) {
            return Ok(());
        }

        let Some(gpu_state) = ctx.world.get_resource::<WaterGpuState>() else {
            return Ok(());
        };
        let Some(water_ubo) = gpu_state.ubo.as_ref() else {
            return Ok(());
        };
        if let Some(descriptor) = gpu_state.descriptor.as_ref() {
            descriptor.write_all_at(
                ctx.rrdevice,
                frame_slot,
                water_ubo,
                scene_color_image.view,
                history_sampler,
                history_views,
                history_sampler,
                trace_image.view,
                history_sampler,
                tlas,
                hit_table,
            )?;
        }
        let trace_state = ctx.world.get_resource::<EffectTraceGpuState>();
        if let Some(trace_descriptor) = trace_state
            .as_ref()
            .and_then(|state| state.descriptor.as_ref())
        {
            trace_descriptor.write_all_at(
                ctx.rrdevice,
                frame_slot,
                tlas,
                trace_image.view,
                hit_table,
            )?;
        }
        targets.mark_bound(frame_slot, key);
        Ok(())
    }

    unsafe fn record(
        &self,
        ctx: &PassContext,
        command_buffer: vk::CommandBuffer,
        image_index: usize,
        _frame_slot: usize,
    ) -> Result<()> {
        let Some(frame) = water_frame(ctx) else {
            return Ok(());
        };
        let Some(gpu_state) = ctx.world.get_resource::<WaterGpuState>() else {
            return Ok(());
        };
        let Some(water_ubo) = gpu_state.ubo.as_ref() else {
            return Ok(());
        };
        let render = ctx.frame_render_context(image_index);
        let instance_ubos =
            record_water_ubo_updates(ctx, &render, water_ubo, &frame.waters, command_buffer)?;

        if let Some(mut targets) = ctx.world.get_resource_mut::<WaterRenderTargets>() {
            targets.frame_instances = instance_ubos;
        }
        Ok(())
    }
}

pub struct WaterCausticClearNode;

impl RenderPassNode for WaterCausticClearNode {
    fn name(&self) -> &'static str {
        "water_caustic_clear"
    }

    fn stage(&self) -> PassStage {
        PassStage::Effect
    }

    fn writes(&self, ctx: &PassContext) -> Vec<TargetUse> {
        frame_uses(ctx, |frame, ctx| {
            if !frame.is_caustic_enabled(ctx) {
                return Vec::new();
            }
            vec![TargetUse::new(CAUSTIC_ACCUM, TargetAccess::TransferDst)]
        })
    }

    unsafe fn record(
        &self,
        ctx: &PassContext,
        command_buffer: vk::CommandBuffer,
        _: usize,
        _: usize,
    ) -> Result<()> {
        if water_frame(ctx).is_none_or(|frame| !frame.is_caustic_enabled(ctx)) {
            return Ok(());
        }
        let Some(targets) = ctx.world.get_resource::<WaterRenderTargets>() else {
            return Ok(());
        };

        ctx.rrdevice.device.cmd_clear_color_image(
            command_buffer,
            targets.buffer.caustic_accum_image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &vk::ClearColorValue { uint32: [0; 4] },
            &[COLOR_SUBRESOURCE_RANGE],
        );
        Ok(())
    }
}

/// Splats refracted light into the accumulation image.
/// The G-buffer position image stays in GENERAL from the ray query pass and is not declared here.
pub struct WaterCausticSplatNode;

impl RenderPassNode for WaterCausticSplatNode {
    fn name(&self) -> &'static str {
        "water_caustic_splat"
    }

    fn stage(&self) -> PassStage {
        PassStage::Effect
    }

    fn writes(&self, ctx: &PassContext) -> Vec<TargetUse> {
        frame_uses(ctx, |frame, ctx| {
            if !frame.is_caustic_enabled(ctx) {
                return Vec::new();
            }
            vec![TargetUse::new(
                CAUSTIC_ACCUM,
                TargetAccess::StorageReadWrite(ShaderStage::Compute),
            )]
        })
    }

    unsafe fn prepare(&self, ctx: &mut PassContext, _: usize) -> Result<()> {
        let Some((tlas, _)) = water_scene_bindings(ctx) else {
            return Ok(());
        };
        let Some(mut gpu_state) = ctx.world.get_resource_mut::<WaterGpuState>() else {
            return Ok(());
        };
        if gpu_state.caustic_bound_tlas == tlas {
            return Ok(());
        }
        let Some(caustic_descriptor) = gpu_state.caustic_descriptor.as_mut() else {
            return Ok(());
        };

        caustic_descriptor.update_tlas(ctx.rrdevice, tlas)?;
        gpu_state.caustic_bound_tlas = tlas;
        Ok(())
    }

    unsafe fn record(
        &self,
        ctx: &PassContext,
        command_buffer: vk::CommandBuffer,
        _: usize,
        _: usize,
    ) -> Result<()> {
        if water_frame(ctx).is_none_or(|frame| !frame.is_caustic_enabled(ctx)) {
            return Ok(());
        }
        let Some(gpu_state) = ctx.world.get_resource::<WaterGpuState>() else {
            return Ok(());
        };
        let (Some(splat_pipeline), Some(descriptor)) = (
            gpu_state.caustic_splat_pipeline.as_ref(),
            gpu_state.caustic_descriptor.as_ref(),
        ) else {
            return Ok(());
        };

        let device = &ctx.rrdevice.device;
        device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::COMPUTE,
            splat_pipeline.pipeline,
        );
        device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::COMPUTE,
            splat_pipeline.pipeline_layout,
            0,
            &[descriptor.splat_descriptor_set],
            &[],
        );
        let splat_group_count = CAUSTIC_GRID_SIZE / CAUSTIC_WORKGROUP_SIZE;
        device.cmd_dispatch(command_buffer, splat_group_count, splat_group_count, 1);
        Ok(())
    }
}

/// Adds the accumulated caustic light to the HDR color.
pub struct WaterCausticApplyNode;

impl RenderPassNode for WaterCausticApplyNode {
    fn name(&self) -> &'static str {
        "water_caustic_apply"
    }

    fn stage(&self) -> PassStage {
        PassStage::Effect
    }

    fn reads(&self, ctx: &PassContext) -> Vec<TargetUse> {
        frame_uses(ctx, |frame, ctx| {
            if !frame.is_caustic_enabled(ctx) {
                return Vec::new();
            }
            vec![TargetUse::new(
                CAUSTIC_ACCUM,
                TargetAccess::StorageRead(ShaderStage::Compute),
            )]
        })
    }

    fn writes(&self, ctx: &PassContext) -> Vec<TargetUse> {
        frame_uses(ctx, |frame, ctx| {
            if !frame.is_caustic_enabled(ctx) {
                return Vec::new();
            }
            vec![TargetUse::new(
                HDR_COLOR,
                TargetAccess::StorageReadWrite(ShaderStage::Compute),
            )]
        })
    }

    unsafe fn record(
        &self,
        ctx: &PassContext,
        command_buffer: vk::CommandBuffer,
        _: usize,
        _: usize,
    ) -> Result<()> {
        if water_frame(ctx).is_none_or(|frame| !frame.is_caustic_enabled(ctx)) {
            return Ok(());
        }
        let Some(gpu_state) = ctx.world.get_resource::<WaterGpuState>() else {
            return Ok(());
        };
        let (Some(apply_pipeline), Some(descriptor), Some(hdr_buffer)) = (
            gpu_state.caustic_apply_pipeline.as_ref(),
            gpu_state.caustic_descriptor.as_ref(),
            ctx.hdr_buffer,
        ) else {
            return Ok(());
        };

        let device = &ctx.rrdevice.device;
        device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::COMPUTE,
            apply_pipeline.pipeline,
        );
        device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::COMPUTE,
            apply_pipeline.pipeline_layout,
            0,
            &[descriptor.apply_descriptor_set],
            &[],
        );
        device.cmd_dispatch(
            command_buffer,
            hdr_buffer.width.div_ceil(CAUSTIC_WORKGROUP_SIZE),
            hdr_buffer.height.div_ceil(CAUSTIC_WORKGROUP_SIZE),
            1,
        );
        Ok(())
    }
}

pub struct WaterSceneColorCopyNode;

impl RenderPassNode for WaterSceneColorCopyNode {
    fn name(&self) -> &'static str {
        "water_scene_color_copy"
    }

    fn stage(&self) -> PassStage {
        PassStage::Effect
    }

    fn reads(&self, ctx: &PassContext) -> Vec<TargetUse> {
        frame_uses(ctx, |_, _| {
            vec![TargetUse::new(HDR_COLOR, TargetAccess::TransferSrc)]
        })
    }

    fn writes(&self, ctx: &PassContext) -> Vec<TargetUse> {
        frame_uses(ctx, |frame, _| {
            vec![TargetUse::new(SCENE_COLOR, TargetAccess::TransferDst)]
        })
    }

    unsafe fn record(
        &self,
        ctx: &PassContext,
        command_buffer: vk::CommandBuffer,
        image_index: usize,
        _: usize,
    ) -> Result<()> {
        if water_frame(ctx).is_none() {
            return Ok(());
        }
        let (Some(targets), Some(hdr_buffer)) = (
            ctx.world.get_resource::<WaterRenderTargets>(),
            ctx.hdr_buffer,
        ) else {
            return Ok(());
        };
        let scene_color_image = ctx.transient_image(SCENE_COLOR_SLOT)?;
        let render = ctx.frame_render_context(image_index);

        super::record_water_scene_color_copy(
            &render,
            hdr_buffer.color_image,
            scene_color_image.image,
            targets.buffer.extent(),
            command_buffer,
        );
        Ok(())
    }
}

pub struct WaterHistoryClearNode;

impl RenderPassNode for WaterHistoryClearNode {
    fn name(&self) -> &'static str {
        "water_history_clear"
    }

    fn stage(&self) -> PassStage {
        PassStage::Effect
    }

    fn writes(&self, ctx: &PassContext) -> Vec<TargetUse> {
        frame_uses(ctx, |frame, ctx| {
            if !frame.is_history_invalidated(ctx) {
                return Vec::new();
            }
            HISTORY_KEYS
                .iter()
                .map(|key| TargetUse::new(TargetRef::Storage(*key), TargetAccess::TransferDst))
                .collect()
        })
    }

    unsafe fn record(
        &self,
        ctx: &PassContext,
        command_buffer: vk::CommandBuffer,
        _: usize,
        _: usize,
    ) -> Result<()> {
        if water_frame(ctx).is_none_or(|frame| !frame.is_history_invalidated(ctx)) {
            return Ok(());
        }
        let Some(targets) = ctx.world.get_resource::<WaterRenderTargets>() else {
            return Ok(());
        };

        let black = vk::ClearColorValue {
            float32: [0.0, 0.0, 0.0, 1.0],
        };
        for &image in &targets.buffer.history_images {
            ctx.rrdevice.device.cmd_clear_color_image(
                command_buffer,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &black,
                &[COLOR_SUBRESOURCE_RANGE],
            );
        }
        Ok(())
    }
}

pub struct WaterShadingNode;

impl RenderPassNode for WaterShadingNode {
    fn name(&self) -> &'static str {
        "water_shading"
    }

    fn stage(&self) -> PassStage {
        PassStage::Effect
    }

    fn reads(&self, ctx: &PassContext) -> Vec<TargetUse> {
        frame_uses(ctx, |frame, _| {
            vec![
                TargetUse::new(SCENE_COLOR, TargetAccess::Sampled(ShaderStage::Fragment)),
                TargetUse::new(TRACE, TargetAccess::StorageRead(ShaderStage::Fragment)),
                TargetUse::new(
                    frame.read_history(),
                    TargetAccess::Sampled(ShaderStage::Fragment),
                ),
            ]
        })
    }

    fn writes(&self, ctx: &PassContext) -> Vec<TargetUse> {
        frame_uses(ctx, |frame, _| {
            if !frame.has_visible_instance() {
                return Vec::new();
            }
            vec![
                TargetUse::new(
                    HDR_COLOR,
                    TargetAccess::Attachment {
                        initial_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                        final_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    },
                ),
                TargetUse::new(
                    frame.written_history(),
                    TargetAccess::Attachment {
                        initial_layout: vk::ImageLayout::UNDEFINED,
                        final_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    },
                ),
            ]
        })
    }

    unsafe fn record(
        &self,
        ctx: &PassContext,
        command_buffer: vk::CommandBuffer,
        image_index: usize,
        frame_slot: usize,
    ) -> Result<()> {
        let Some(frame) = water_frame(ctx) else {
            return Ok(());
        };
        let (Some(targets), Some(gpu_state)) = (
            ctx.world.get_resource::<WaterRenderTargets>(),
            ctx.world.get_resource::<WaterGpuState>(),
        ) else {
            return Ok(());
        };
        let (Some(shading_pipeline), Some(descriptor)) = (
            gpu_state.shading_pipeline.as_ref(),
            gpu_state.descriptor.as_ref(),
        ) else {
            return Ok(());
        };
        let water_buffer = &targets.buffer;
        let render = ctx.frame_render_context(image_index);

        for (i, (_, ubo_dynamic_offset)) in targets.frame_instances.iter().enumerate() {
            let Some(scissor) = frame.scissors.get(i).copied().flatten() else {
                continue;
            };

            let push_constants = super::WaterPushConstants::new(
                frame.secondary_rays(ctx).as_shader_value(),
                frame.settings.debug_view,
            );

            super::record_water_shading_pass(
                &render,
                water_buffer,
                shading_pipeline,
                descriptor,
                *ubo_dynamic_offset,
                scissor,
                push_constants,
                image_index,
                frame_slot,
                frame.history_index,
                command_buffer,
            )?;
        }
        Ok(())
    }
}

unsafe fn record_water_ubo_updates(
    ctx: &PassContext,
    render: &thyllore_vulkan_core::FrameRenderContext,
    water_ubo: &thyllore_vulkan_core::resource::UniformBuffer<thyllore_effect_core::WaterUBO>,
    waters: &[crate::ecs::world::Entity],
    command_buffer: vk::CommandBuffer,
) -> Result<Vec<(thyllore_effect_core::WaterUBO, u32)>> {
    let projection = ctx.world.resource::<crate::ecs::resource::ProjectionData>();
    let inv_view_proj =
        crate::ecs::systems::water::probe::inverse_view_proj_f64(projection.proj, projection.view);
    let settings = ctx
        .world
        .get_resource::<crate::ecs::resource::WaterRenderSettings>()
        .map(|settings| *settings)
        .unwrap_or_default();

    let mut instance_ubos = Vec::with_capacity(waters.len());
    for (i, water) in waters.iter().enumerate() {
        let effect = ctx
            .world
            .get_component::<crate::ecs::component::WaterTorusEffect>(*water)
            .ok_or_else(|| anyhow::anyhow!("Missing WaterTorusEffect for instance {}", i))?;
        let accum = ctx
            .world
            .get_component::<crate::ecs::component::WaterTemporalAccum>(*water)
            .cloned()
            .unwrap_or_default();

        let mut ubo = thyllore_effect_core::build_water_ubo(effect, accum.frame_index as u32);
        ubo.inv_view_proj = inv_view_proj;
        ubo.temporal = [accum.weight, accum.frame_index as f32, 0.0, 0.0];
        ubo.composite[3] = settings.caustic_debug as f32;

        water_ubo.record_update(
            &render.device.device,
            command_buffer,
            i,
            &ubo,
            vk::PipelineStageFlags::FRAGMENT_SHADER | vk::PipelineStageFlags::COMPUTE_SHADER,
        )?;
        instance_ubos.push((ubo, water_ubo.slot_offset(i)? as u32));
    }
    Ok(instance_ubos)
}

fn compute_water_scissor(
    ctx: &PassContext,
    extent: vk::Extent2D,
    model: &cgmath::Matrix4<f32>,
    major_radius: f32,
    minor_radius: f32,
) -> Option<vk::Rect2D> {
    use crate::ecs::resource::ProjectionData;
    const SCISSOR_MARGIN_PX: f32 = 2.0;

    let Some(projection) = ctx.world.get_resource::<ProjectionData>() else {
        return Some(full_extent_scissor(extent));
    };
    let view_proj = projection.proj * projection.view;

    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    let corners = thyllore_math_core::torus_local_bounds_corners(major_radius, minor_radius);
    let corners_behind_camera = corners
        .iter()
        .filter(|corner| {
            (view_proj * model * cgmath::vec4(corner.x, corner.y, corner.z, 1.0)).w <= 0.0
        })
        .count();
    if corners_behind_camera == corners.len() {
        return None;
    }
    if corners_behind_camera > 0 {
        return Some(full_extent_scissor(extent));
    }
    for corner in corners {
        let clip = view_proj * model * cgmath::vec4(corner.x, corner.y, corner.z, 1.0);
        let screen_x = (clip.x / clip.w + 1.0) * 0.5 * extent.width as f32;
        let screen_y = (clip.y / clip.w + 1.0) * 0.5 * extent.height as f32;
        min_x = min_x.min(screen_x);
        min_y = min_y.min(screen_y);
        max_x = max_x.max(screen_x);
        max_y = max_y.max(screen_y);
    }

    let min_x = (min_x - SCISSOR_MARGIN_PX).clamp(0.0, extent.width as f32);
    let min_y = (min_y - SCISSOR_MARGIN_PX).clamp(0.0, extent.height as f32);
    let max_x = (max_x + SCISSOR_MARGIN_PX).clamp(0.0, extent.width as f32);
    let max_y = (max_y + SCISSOR_MARGIN_PX).clamp(0.0, extent.height as f32);
    if max_x - min_x < 1.0 || max_y - min_y < 1.0 {
        return None;
    }

    Some(
        vk::Rect2D::builder()
            .offset(vk::Offset2D {
                x: min_x as i32,
                y: min_y as i32,
            })
            .extent(vk::Extent2D {
                width: (max_x - min_x).ceil() as u32,
                height: (max_y - min_y).ceil() as u32,
            })
            .build(),
    )
}
