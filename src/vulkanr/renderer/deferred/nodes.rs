use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use super::{composite_pass::*, onion_skin_pass::*, post_process_pass::*, tonemap_pass::*};
use crate::app::post_process::{
    bloom_mip_count, is_dof_enabled, prepare_auto_exposure_input, prepare_bloom_targets,
    prepare_dof_target, prepare_tonemap_inputs, BLOOM_MIPS, DOF_OUTPUT, MAX_BLOOM_MIPS,
};
use crate::ecs::PassContext;
use crate::hooks::pass::{
    CoreTarget, PassGraph, PassStage, RenderPassNode, ShaderStage, TargetAccess, TargetRef,
    TargetUse, TransientRequest,
};

const HDR_COLOR: TargetRef = TargetRef::Core(CoreTarget::HdrColor);
const OFFSCREEN: TargetRef = TargetRef::Core(CoreTarget::Offscreen);
const ONION_GHOST: TargetRef = TargetRef::Core(CoreTarget::OnionSkinGhost);

const fn cleared_attachment() -> TargetAccess {
    TargetAccess::Attachment {
        initial_layout: vk::ImageLayout::UNDEFINED,
        final_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
    }
}

const fn loaded_attachment(layout: vk::ImageLayout) -> TargetAccess {
    TargetAccess::Attachment {
        initial_layout: layout,
        final_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
    }
}

fn tonemap_input(ctx: &PassContext) -> TargetRef {
    if is_dof_enabled(ctx) {
        TargetRef::Transient(DOF_OUTPUT)
    } else {
        HDR_COLOR
    }
}

fn bloom_mip(ctx: &PassContext, mip_index: usize) -> Option<TargetRef> {
    (mip_index < bloom_mip_count(ctx)).then(|| TargetRef::Transient(BLOOM_MIPS[mip_index]))
}

fn is_bloom_enabled(ctx: &PassContext) -> bool {
    bloom_mip_count(ctx) > 0
}

fn is_onion_skin_active(ctx: &PassContext) -> bool {
    ctx.raytracing.onion_skin_pass.is_some()
        && ctx
            .onion_skin_gpu
            .is_some_and(|gpu| gpu.source_mesh_index.is_some() && gpu.active_ghost_count() > 0)
}

fn is_auto_exposure_enabled(ctx: &PassContext) -> bool {
    ctx.world
        .get_resource::<crate::ecs::resource::AutoExposure>()
        .is_some_and(|settings| settings.enabled)
        && ctx.auto_exposure_buffers.is_some()
}

pub struct CompositeHdrNode;

impl RenderPassNode for CompositeHdrNode {
    fn name(&self) -> &'static str {
        "composite_hdr"
    }

    fn stage(&self) -> PassStage {
        PassStage::Lighting
    }

    fn writes(&self, _ctx: &PassContext) -> Vec<TargetUse> {
        vec![TargetUse::new(HDR_COLOR, cleared_attachment())]
    }

    unsafe fn record(
        &self,
        ctx: &PassContext,
        cmd: vk::CommandBuffer,
        _: usize,
        _: usize,
    ) -> Result<()> {
        record_composite_to_hdr(ctx, cmd)
    }
}

pub struct OnionSkinNode;

impl RenderPassNode for OnionSkinNode {
    fn name(&self) -> &'static str {
        "onion_skin"
    }

    fn stage(&self) -> PassStage {
        PassStage::Lighting
    }

    fn writes(&self, ctx: &PassContext) -> Vec<TargetUse> {
        if !is_onion_skin_active(ctx) {
            return Vec::new();
        }
        vec![TargetUse::new(ONION_GHOST, cleared_attachment())]
    }

    unsafe fn record(
        &self,
        ctx: &PassContext,
        cmd: vk::CommandBuffer,
        image_index: usize,
        _: usize,
    ) -> Result<()> {
        record_onion_skin_pass(ctx, cmd, image_index)
    }
}

pub struct BloomDownsampleNode {
    mip_index: usize,
}

impl RenderPassNode for BloomDownsampleNode {
    fn name(&self) -> &'static str {
        BLOOM_DOWNSAMPLE_NAMES[self.mip_index]
    }

    fn stage(&self) -> PassStage {
        PassStage::PostProcess
    }

    fn transients(&self, ctx: &PassContext) -> Vec<TransientRequest> {
        if bloom_mip(ctx, self.mip_index).is_none() {
            return Vec::new();
        }
        ctx.bloom_chain
            .and_then(|chain| chain.mip_desc(self.mip_index))
            .map(|desc| TransientRequest::new(BLOOM_MIPS[self.mip_index], desc))
            .into_iter()
            .collect()
    }

    unsafe fn prepare(&self, ctx: &mut PassContext, frame_slot: usize) -> Result<()> {
        if self.mip_index == 0 {
            prepare_bloom_targets(ctx, frame_slot)?;
        }
        Ok(())
    }

    fn reads(&self, ctx: &PassContext) -> Vec<TargetUse> {
        if bloom_mip(ctx, self.mip_index).is_none() {
            return Vec::new();
        }
        let source = match self.mip_index {
            0 => Some(HDR_COLOR),
            index => bloom_mip(ctx, index - 1),
        };
        source
            .map(|target| TargetUse::new(target, TargetAccess::Sampled(ShaderStage::Fragment)))
            .into_iter()
            .collect()
    }

    fn writes(&self, ctx: &PassContext) -> Vec<TargetUse> {
        if !is_bloom_enabled(ctx) {
            return Vec::new();
        }
        bloom_mip(ctx, self.mip_index)
            .map(|target| TargetUse::new(target, cleared_attachment()))
            .into_iter()
            .collect()
    }

    unsafe fn record(
        &self,
        ctx: &PassContext,
        cmd: vk::CommandBuffer,
        _: usize,
        frame_slot: usize,
    ) -> Result<()> {
        record_bloom_downsample(ctx, cmd, self.mip_index, frame_slot)
    }
}

pub struct BloomUpsampleNode {
    pass_index: usize,
}

impl BloomUpsampleNode {
    fn target_mip(&self, ctx: &PassContext) -> Option<usize> {
        thyllore_vulkan_core::renderer::bloom_upsample_target_mip(
            bloom_mip_count(ctx),
            self.pass_index,
        )
    }
}

impl RenderPassNode for BloomUpsampleNode {
    fn name(&self) -> &'static str {
        BLOOM_UPSAMPLE_NAMES[self.pass_index]
    }

    fn stage(&self) -> PassStage {
        PassStage::PostProcess
    }

    fn reads(&self, ctx: &PassContext) -> Vec<TargetUse> {
        if !is_bloom_enabled(ctx) {
            return Vec::new();
        }
        self.target_mip(ctx)
            .and_then(|target| bloom_mip(ctx, target + 1))
            .map(|source| TargetUse::new(source, TargetAccess::Sampled(ShaderStage::Fragment)))
            .into_iter()
            .collect()
    }

    fn writes(&self, ctx: &PassContext) -> Vec<TargetUse> {
        if !is_bloom_enabled(ctx) {
            return Vec::new();
        }
        self.target_mip(ctx)
            .and_then(|target| bloom_mip(ctx, target))
            .map(|target| {
                TargetUse::new(
                    target,
                    loaded_attachment(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL),
                )
            })
            .into_iter()
            .collect()
    }

    unsafe fn record(
        &self,
        ctx: &PassContext,
        cmd: vk::CommandBuffer,
        _: usize,
        frame_slot: usize,
    ) -> Result<()> {
        record_bloom_upsample(ctx, cmd, self.pass_index, frame_slot)
    }
}

pub struct DofNode;

impl RenderPassNode for DofNode {
    fn name(&self) -> &'static str {
        "dof"
    }

    fn stage(&self) -> PassStage {
        PassStage::PostProcess
    }

    fn transients(&self, ctx: &PassContext) -> Vec<TransientRequest> {
        if !is_dof_enabled(ctx) {
            return Vec::new();
        }
        ctx.dof_buffer
            .map(|dof_buffer| TransientRequest::new(DOF_OUTPUT, dof_buffer.output_desc()))
            .into_iter()
            .collect()
    }

    unsafe fn prepare(&self, ctx: &mut PassContext, _frame_slot: usize) -> Result<()> {
        prepare_dof_target(ctx)
    }

    fn reads(&self, ctx: &PassContext) -> Vec<TargetUse> {
        if !is_dof_enabled(ctx) {
            return Vec::new();
        }
        vec![TargetUse::new(
            HDR_COLOR,
            TargetAccess::Sampled(ShaderStage::Fragment),
        )]
    }

    fn writes(&self, ctx: &PassContext) -> Vec<TargetUse> {
        if !is_dof_enabled(ctx) {
            return Vec::new();
        }
        vec![TargetUse::new(
            TargetRef::Transient(DOF_OUTPUT),
            cleared_attachment(),
        )]
    }

    unsafe fn record(
        &self,
        ctx: &PassContext,
        cmd: vk::CommandBuffer,
        _: usize,
        _: usize,
    ) -> Result<()> {
        record_dof(ctx, cmd)
    }
}

pub struct AutoExposureNode;

impl RenderPassNode for AutoExposureNode {
    fn name(&self) -> &'static str {
        "auto_exposure"
    }

    fn stage(&self) -> PassStage {
        PassStage::PostProcess
    }

    unsafe fn prepare(&self, ctx: &mut PassContext, frame_slot: usize) -> Result<()> {
        if !is_auto_exposure_enabled(ctx) {
            return Ok(());
        }
        prepare_auto_exposure_input(ctx, frame_slot)
    }

    fn reads(&self, ctx: &PassContext) -> Vec<TargetUse> {
        if !is_auto_exposure_enabled(ctx) {
            return Vec::new();
        }
        vec![TargetUse::new(
            tonemap_input(ctx),
            TargetAccess::Sampled(ShaderStage::Compute),
        )]
    }

    unsafe fn record(
        &self,
        ctx: &PassContext,
        cmd: vk::CommandBuffer,
        _: usize,
        frame_slot: usize,
    ) -> Result<()> {
        record_auto_exposure(ctx, cmd, frame_slot)
    }
}

pub struct TonemapNode;

impl RenderPassNode for TonemapNode {
    fn name(&self) -> &'static str {
        "tonemap"
    }

    fn stage(&self) -> PassStage {
        PassStage::Final
    }

    unsafe fn prepare(&self, ctx: &mut PassContext, frame_slot: usize) -> Result<()> {
        prepare_tonemap_inputs(ctx, frame_slot)
    }

    fn reads(&self, ctx: &PassContext) -> Vec<TargetUse> {
        let sampled = TargetAccess::Sampled(ShaderStage::Fragment);
        let mut reads = vec![TargetUse::new(tonemap_input(ctx), sampled)];
        if is_bloom_enabled(ctx) {
            reads.extend(bloom_mip(ctx, 0).map(|target| TargetUse::new(target, sampled)));
        }
        reads
    }

    fn writes(&self, _ctx: &PassContext) -> Vec<TargetUse> {
        vec![TargetUse::new(OFFSCREEN, cleared_attachment())]
    }

    unsafe fn record(
        &self,
        ctx: &PassContext,
        cmd: vk::CommandBuffer,
        image_index: usize,
        _: usize,
    ) -> Result<()> {
        record_tonemap_to_offscreen(ctx, cmd, image_index)
    }
}

pub struct OnionCompositeNode;

impl RenderPassNode for OnionCompositeNode {
    fn name(&self) -> &'static str {
        "onion_composite"
    }

    fn stage(&self) -> PassStage {
        PassStage::Final
    }

    fn reads(&self, ctx: &PassContext) -> Vec<TargetUse> {
        if !is_onion_skin_active(ctx) {
            return Vec::new();
        }
        vec![TargetUse::new(
            ONION_GHOST,
            TargetAccess::Sampled(ShaderStage::Fragment),
        )]
    }

    fn writes(&self, ctx: &PassContext) -> Vec<TargetUse> {
        if !is_onion_skin_active(ctx) {
            return Vec::new();
        }
        vec![TargetUse::new(
            OFFSCREEN,
            loaded_attachment(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL),
        )]
    }

    unsafe fn record(
        &self,
        ctx: &PassContext,
        cmd: vk::CommandBuffer,
        _: usize,
        _: usize,
    ) -> Result<()> {
        record_onion_skin_composite(ctx, cmd)
    }
}

const BLOOM_DOWNSAMPLE_NAMES: [&str; MAX_BLOOM_MIPS] = [
    "bloom_downsample_0",
    "bloom_downsample_1",
    "bloom_downsample_2",
    "bloom_downsample_3",
    "bloom_downsample_4",
    "bloom_downsample_5",
    "bloom_downsample_6",
    "bloom_downsample_7",
];

const BLOOM_UPSAMPLE_NAMES: [&str; MAX_BLOOM_MIPS] = [
    "bloom_upsample_0",
    "bloom_upsample_1",
    "bloom_upsample_2",
    "bloom_upsample_3",
    "bloom_upsample_4",
    "bloom_upsample_5",
    "bloom_upsample_6",
    "bloom_upsample_7",
];

static BLOOM_DOWNSAMPLE_NODES: [BloomDownsampleNode; MAX_BLOOM_MIPS] = [
    BloomDownsampleNode { mip_index: 0 },
    BloomDownsampleNode { mip_index: 1 },
    BloomDownsampleNode { mip_index: 2 },
    BloomDownsampleNode { mip_index: 3 },
    BloomDownsampleNode { mip_index: 4 },
    BloomDownsampleNode { mip_index: 5 },
    BloomDownsampleNode { mip_index: 6 },
    BloomDownsampleNode { mip_index: 7 },
];

static BLOOM_UPSAMPLE_NODES: [BloomUpsampleNode; MAX_BLOOM_MIPS] = [
    BloomUpsampleNode { pass_index: 0 },
    BloomUpsampleNode { pass_index: 1 },
    BloomUpsampleNode { pass_index: 2 },
    BloomUpsampleNode { pass_index: 3 },
    BloomUpsampleNode { pass_index: 4 },
    BloomUpsampleNode { pass_index: 5 },
    BloomUpsampleNode { pass_index: 6 },
    BloomUpsampleNode { pass_index: 7 },
];

pub fn register_core_passes(graph: &mut PassGraph) {
    graph.register(&CompositeHdrNode);
    graph.register(&OnionSkinNode);
    for node in &BLOOM_DOWNSAMPLE_NODES {
        graph.register(node);
    }
    for node in &BLOOM_UPSAMPLE_NODES {
        graph.register(node);
    }
    graph.register(&DofNode);
    graph.register(&AutoExposureNode);
    graph.register(&TonemapNode);
    graph.register(&OnionCompositeNode);
}
