use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::core::RRDevice;
use crate::resource::image::{SamplerAddress, SamplerBorder, SamplerDesc, SamplerFilter};
use crate::resource::render_target_storage::{RenderTargetKey, RenderTargetStorage};
use crate::resource::{HistoryTargets, HistoryTargetsDesc};
use crate::types::Instance;

const FLAME_HISTORY_KEYS: [RenderTargetKey; 2] = [
    RenderTargetKey::EffectHistory(0),
    RenderTargetKey::EffectHistory(1),
];

pub const FLAME_HISTORY_FORMAT: crate::types::Format = crate::types::Format::R16G16B16A16_SFLOAT;

fn nearest_clamp_sampler_desc() -> SamplerDesc {
    SamplerDesc {
        filter: SamplerFilter::Nearest,
        address: SamplerAddress::ClampToEdge,
        border: SamplerBorder::OpaqueBlackInt,
    }
}

pub fn flame_history_desc() -> HistoryTargetsDesc {
    HistoryTargetsDesc {
        keys: FLAME_HISTORY_KEYS,
        format: FLAME_HISTORY_FORMAT.raw(),
        usage: vk::ImageUsageFlags::TRANSFER_SRC,
        sampler: nearest_clamp_sampler_desc(),
        history_load_op: vk::AttachmentLoadOp::CLEAR,
        depth_view: None,
    }
}

pub unsafe fn create_flame_history(
    instance: &Instance,
    rrdevice: &RRDevice,
    storage: &mut RenderTargetStorage,
    command_pool: crate::types::CommandPool,
    width: u32,
    height: u32,
    hdr_view: crate::types::ImageView,
) -> Result<HistoryTargets> {
    HistoryTargets::new(
        instance,
        rrdevice,
        storage,
        command_pool.raw(),
        width,
        height,
        hdr_view.raw(),
        flame_history_desc(),
    )
}

pub unsafe fn recreate_flame_history(
    instance: &Instance,
    rrdevice: &RRDevice,
    storage: &mut RenderTargetStorage,
    command_pool: crate::types::CommandPool,
    history: &mut HistoryTargets,
    width: u32,
    height: u32,
    hdr_view: crate::types::ImageView,
) -> Result<()> {
    history.destroy(&rrdevice.device());
    *history = create_flame_history(
        instance,
        rrdevice,
        storage,
        command_pool,
        width,
        height,
        hdr_view,
    )?;
    Ok(())
}
