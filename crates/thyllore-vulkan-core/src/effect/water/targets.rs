use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::core::RRDevice;
use crate::resource::hdr_buffer::HDR_FORMAT;
use crate::resource::render_target_storage::{RenderTargetKey, RenderTargetStorage};
use crate::resource::{AccumulationTarget, GpuResource, HistoryTargets, HistoryTargetsDesc};
use crate::types::Instance;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WaterBindingKey {
    pub tlas: crate::types::AccelerationStructure,
    pub hit_table: crate::types::Buffer,
    pub history_views: [crate::types::ImageView; 2],
    pub scene_color_generation: u64,
    pub trace_generation: u64,
}

#[must_use]
#[derive(Debug, Default, GpuResource)]
pub struct WaterRenderTargets {
    pub history: HistoryTargets,
    pub caustic_accum: AccumulationTarget,
    #[gpu_resource(skip)]
    pub frame_instances: Vec<(thyllore_effect_core::WaterUBO, u32)>,
    #[gpu_resource(skip)]
    bound: Vec<Option<WaterBindingKey>>,
}

const WATER_HISTORY_KEYS: [RenderTargetKey; 2] = [
    RenderTargetKey::EffectHistory(2),
    RenderTargetKey::EffectHistory(3),
];
const WATER_CAUSTIC_ACCUM_KEY: RenderTargetKey = RenderTargetKey::EffectAccumulation(0);
const WATER_CAUSTIC_ACCUM_FORMAT: vk::Format = vk::Format::R32_UINT;

fn water_history_desc(depth_view: vk::ImageView) -> HistoryTargetsDesc {
    HistoryTargetsDesc {
        keys: WATER_HISTORY_KEYS,
        format: HDR_FORMAT,
        usage: vk::ImageUsageFlags::empty(),
        sampler: linear_clamp_sampler_info(),
        history_load_op: vk::AttachmentLoadOp::DONT_CARE,
        depth_view: Some(depth_view),
    }
}

fn linear_clamp_sampler_info() -> vk::SamplerCreateInfo {
    let address_mode = vk::SamplerAddressMode::CLAMP_TO_EDGE;
    vk::SamplerCreateInfo::builder()
        .mag_filter(vk::Filter::LINEAR)
        .min_filter(vk::Filter::LINEAR)
        .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
        .address_mode_u(address_mode)
        .address_mode_v(address_mode)
        .address_mode_w(address_mode)
        .border_color(vk::BorderColor::FLOAT_OPAQUE_BLACK)
        .anisotropy_enable(false)
        .max_anisotropy(1.0)
        .build()
}

pub unsafe fn create_water_gpu_targets(
    instance: &Instance,
    rrdevice: &RRDevice,
    storage: &mut RenderTargetStorage,
    command_pool: crate::types::CommandPool,
    width: u32,
    height: u32,
    hdr_view: crate::types::ImageView,
    depth_view: crate::types::ImageView,
) -> Result<WaterRenderTargets> {
    let command_pool = command_pool.raw();
    let history = HistoryTargets::new(
        instance,
        rrdevice,
        storage,
        command_pool,
        width,
        height,
        hdr_view.raw(),
        water_history_desc(depth_view.raw()),
    )?;
    let caustic_accum = AccumulationTarget::new(
        instance,
        rrdevice,
        storage,
        command_pool,
        WATER_CAUSTIC_ACCUM_KEY,
        WATER_CAUSTIC_ACCUM_FORMAT,
    )?;

    Ok(WaterRenderTargets {
        history,
        caustic_accum,
        frame_instances: Vec::new(),
        bound: Vec::new(),
    })
}

pub fn is_depth_view_ready(view: crate::types::ImageView) -> bool {
    view.raw() != vk::ImageView::null()
}

impl WaterRenderTargets {
    pub fn extent(&self) -> crate::types::Extent2D {
        self.history.extent()
    }

    pub fn scene_color_desc(&self) -> crate::resource::render_target_transient::TransientDesc {
        crate::resource::render_target_transient::TransientDesc {
            width: self.history.width,
            height: self.history.height,
            format: HDR_FORMAT,
            usage: vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
        }
    }

    pub fn trace_desc(&self) -> crate::resource::render_target_transient::TransientDesc {
        crate::resource::render_target_transient::TransientDesc {
            width: self.history.width,
            height: self.history.height,
            format: vk::Format::R16G16B16A16_SFLOAT,
            usage: vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
        }
    }

    pub fn forget_bindings(&mut self) {
        self.bound.clear();
    }

    pub fn is_bound(&self, frame_slot: usize, key: WaterBindingKey) -> bool {
        self.bound.get(frame_slot).copied().flatten() == Some(key)
    }

    pub fn mark_bound(&mut self, frame_slot: usize, key: WaterBindingKey) {
        if self.bound.len() <= frame_slot {
            self.bound.resize(frame_slot + 1, None);
        }
        self.bound[frame_slot] = Some(key);
    }
}
