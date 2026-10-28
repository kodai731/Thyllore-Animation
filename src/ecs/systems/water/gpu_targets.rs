use anyhow::Result;

use thyllore_vulkan_core::core::RRDevice;
use thyllore_vulkan_core::resource::image::{
    SamplerAddress, SamplerBorder, SamplerDesc, SamplerFilter,
};
use thyllore_vulkan_core::resource::render_target_storage::{RenderTargetKey, RenderTargetStorage};
use thyllore_vulkan_core::resource::volume_image::VolumeUsage;
use thyllore_vulkan_core::resource::{
    AccumulationTarget, GpuResource, HistoryTargets, HistoryTargetsDesc, TransientDesc,
};
use thyllore_vulkan_core::types::{CommandPool, Extent2D, Format, ImageView, Instance, HDR_FORMAT};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WaterBindingKey {
    pub tlas: thyllore_vulkan_core::types::AccelerationStructure,
    pub hit_table: thyllore_vulkan_core::types::Buffer,
    pub history_views: [ImageView; 2],
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

crate::gpu_resource!(WaterRenderTargets);

const WATER_HISTORY_KEYS: [RenderTargetKey; 2] = [
    RenderTargetKey::EffectHistory(2),
    RenderTargetKey::EffectHistory(3),
];
const WATER_CAUSTIC_ACCUM_KEY: RenderTargetKey = RenderTargetKey::EffectAccumulation(0);
const WATER_CAUSTIC_ACCUM_FORMAT: Format = Format::R32_UINT;

fn water_history_desc(depth_view: ImageView) -> HistoryTargetsDesc {
    HistoryTargetsDesc::with_depth(
        WATER_HISTORY_KEYS,
        HDR_FORMAT,
        linear_clamp_sampler_desc(),
        depth_view,
    )
}

fn linear_clamp_sampler_desc() -> SamplerDesc {
    SamplerDesc {
        filter: SamplerFilter::Linear,
        address: SamplerAddress::ClampToEdge,
        border: SamplerBorder::OpaqueBlackFloat,
    }
}

pub unsafe fn create_water_gpu_targets(
    instance: &Instance,
    rrdevice: &RRDevice,
    storage: &mut RenderTargetStorage,
    command_pool: CommandPool,
    width: u32,
    height: u32,
    hdr_view: ImageView,
    depth_view: ImageView,
) -> Result<WaterRenderTargets> {
    let history = HistoryTargets::create(
        instance,
        rrdevice,
        storage,
        command_pool,
        width,
        height,
        hdr_view,
        water_history_desc(depth_view),
    )?;
    let caustic_accum = AccumulationTarget::create(
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

pub fn is_depth_view_ready(view: ImageView) -> bool {
    view != ImageView::default()
}

impl WaterRenderTargets {
    pub fn extent(&self) -> Extent2D {
        self.history.extent()
    }

    pub fn scene_color_desc(&self) -> TransientDesc {
        TransientDesc::new(
            self.history.width,
            self.history.height,
            HDR_FORMAT,
            VolumeUsage::TransferDstSampled,
        )
    }

    pub fn trace_desc(&self) -> TransientDesc {
        TransientDesc::new(
            self.history.width,
            self.history.height,
            Format::R16G16B16A16_SFLOAT,
            VolumeUsage::StorageSampled,
        )
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
