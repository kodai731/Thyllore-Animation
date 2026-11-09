use anyhow::Result;

use crate::vulkanr::core::RRDevice;
use crate::vulkanr::image::{SamplerAddress, SamplerBorder, SamplerDesc, SamplerFilter};
use crate::vulkanr::resource::{
    GpuResource, HistoryTargets, HistoryTargetsDesc, RenderTargetKey, RenderTargetStorage,
};
use thyllore_vulkan_core::types::{CommandPool, Format, ImageView, Instance};

#[must_use]
#[derive(Debug, Default, GpuResource)]
pub struct FlameRenderTargets {
    pub history: HistoryTargets,
}

crate::gpu_resource!(FlameRenderTargets);

const FLAME_HISTORY_KEYS: [RenderTargetKey; 2] = [
    RenderTargetKey::EffectHistory(0),
    RenderTargetKey::EffectHistory(1),
];

pub const FLAME_HISTORY_FORMAT: Format = Format::R16G16B16A16_SFLOAT;

fn nearest_clamp_sampler_desc() -> SamplerDesc {
    SamplerDesc {
        filter: SamplerFilter::Nearest,
        address: SamplerAddress::ClampToEdge,
        border: SamplerBorder::OpaqueBlackInt,
    }
}

pub fn flame_history_desc() -> HistoryTargetsDesc {
    HistoryTargetsDesc::cleared_readback(
        FLAME_HISTORY_KEYS,
        FLAME_HISTORY_FORMAT,
        nearest_clamp_sampler_desc(),
    )
}

pub unsafe fn create_flame_history(
    instance: &Instance,
    rrdevice: &RRDevice,
    storage: &mut RenderTargetStorage,
    command_pool: CommandPool,
    width: u32,
    height: u32,
    hdr_view: ImageView,
) -> Result<HistoryTargets> {
    HistoryTargets::create(
        instance,
        rrdevice,
        storage,
        command_pool,
        width,
        height,
        hdr_view,
        flame_history_desc(),
    )
}

pub unsafe fn recreate_flame_history(
    instance: &Instance,
    rrdevice: &RRDevice,
    storage: &mut RenderTargetStorage,
    command_pool: CommandPool,
    history: &mut HistoryTargets,
    width: u32,
    height: u32,
    hdr_view: ImageView,
) -> Result<()> {
    history.destroy_gpu(rrdevice);
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
