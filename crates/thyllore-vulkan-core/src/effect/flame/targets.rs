use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::core::RRDevice;
use crate::resource::render_target_storage::{RenderTargetKey, RenderTargetStorage};
use crate::resource::{HistoryTargets, HistoryTargetsDesc};
use crate::types::Instance;

const FLAME_HISTORY_KEYS: [RenderTargetKey; 2] = [
    RenderTargetKey::EffectHistory(0),
    RenderTargetKey::EffectHistory(1),
];

pub const FLAME_HISTORY_FORMAT: vk::Format = vk::Format::R16G16B16A16_SFLOAT;

pub fn flame_history_desc() -> HistoryTargetsDesc {
    HistoryTargetsDesc {
        keys: FLAME_HISTORY_KEYS,
        format: FLAME_HISTORY_FORMAT,
        usage: vk::ImageUsageFlags::TRANSFER_SRC,
        sampler: nearest_clamp_sampler_info(),
        history_load_op: vk::AttachmentLoadOp::CLEAR,
        depth_view: None,
    }
}

fn nearest_clamp_sampler_info() -> vk::SamplerCreateInfo {
    vk::SamplerCreateInfo::builder()
        .mag_filter(vk::Filter::NEAREST)
        .min_filter(vk::Filter::NEAREST)
        .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .anisotropy_enable(false)
        .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
        .unnormalized_coordinates(false)
        .compare_enable(false)
        .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
        .mip_lod_bias(0.0)
        .min_lod(0.0)
        .max_lod(0.0)
        .build()
}

pub unsafe fn create_flame_history(
    instance: &Instance,
    rrdevice: &RRDevice,
    storage: &mut RenderTargetStorage,
    command_pool: vk::CommandPool,
    width: u32,
    height: u32,
    hdr_view: vk::ImageView,
) -> Result<HistoryTargets> {
    HistoryTargets::new(
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
    command_pool: vk::CommandPool,
    history: &mut HistoryTargets,
    width: u32,
    height: u32,
    hdr_view: vk::ImageView,
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
