use anyhow::Result;

use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::pass_manifest::FLAME_RESOLVE;
use crate::vulkanr::descriptor::shader_bindings::flame_resolve;
use crate::vulkanr::descriptor::{
    DescriptorKind, ReflectedDescriptorSets, ReflectedLayoutSpec, SetGrid, SetSlot,
};
use crate::vulkanr::image::create_scene_depth_sampler;
use thyllore_effect_core::FlameUBO;
use thyllore_vulkan_core::resource::{GpuResource, UniformBuffer};
use thyllore_vulkan_core::types::{ImageView, Sampler};

const FLAME_HISTORY_SET_COUNT: usize = 2;

#[derive(Clone, Copy, Debug)]
pub struct FlameImageBindings {
    pub history_image_views: [ImageView; FLAME_HISTORY_SET_COUNT],
    pub flame_sampler: Sampler,
    pub sdf_image_view: ImageView,
    pub sdf_sampler: Sampler,
    pub scene_depth_view: ImageView,
}

/// One set per history index; each samples the history image the other index wrote.
#[must_use]
#[derive(Debug, Default, GpuResource)]
pub struct RRFlameDescriptorSet {
    pub sets: ReflectedDescriptorSets,
    scene_depth_sampler: Sampler,
}

impl RRFlameDescriptorSet {
    pub unsafe fn new(rrdevice: &RRDevice) -> Result<Self> {
        let layout_spec = ReflectedLayoutSpec::local(&FLAME_RESOLVE)
            .with_override(flame_resolve::FLAME, DescriptorKind::UniformBufferDynamic);
        let sets = ReflectedDescriptorSets::create(
            rrdevice,
            &layout_spec,
            SetGrid::per_history(FLAME_HISTORY_SET_COUNT),
        )?;
        let scene_depth_sampler = create_scene_depth_sampler(rrdevice)?;

        Ok(Self {
            sets,
            scene_depth_sampler,
        })
    }

    pub unsafe fn write_all(
        &self,
        rrdevice: &RRDevice,
        flame_ubo: &UniformBuffer<FlameUBO>,
        images: FlameImageBindings,
    ) -> Result<()> {
        for history_index in 0..FLAME_HISTORY_SET_COUNT {
            self.sets
                .writer(SetSlot::history(history_index))?
                .uniform_dynamic(flame_resolve::FLAME, flame_ubo)?
                .apply(rrdevice);
        }
        self.update_image_views(rrdevice, images)
    }

    pub unsafe fn update_image_views(
        &self,
        rrdevice: &RRDevice,
        images: FlameImageBindings,
    ) -> Result<()> {
        for history_index in 0..FLAME_HISTORY_SET_COUNT {
            let previous_history_view = images.history_image_views[1 - history_index];
            self.sets
                .writer(SetSlot::history(history_index))?
                .sampled_image(
                    flame_resolve::FLAME_HISTORY_SAMPLER,
                    previous_history_view,
                    images.flame_sampler,
                )?
                .sampled_image(
                    flame_resolve::FLAME_SDF_SAMPLER,
                    images.sdf_image_view,
                    images.sdf_sampler,
                )?
                .sampled_depth(
                    flame_resolve::SCENE_DEPTH_SAMPLER,
                    images.scene_depth_view,
                    self.scene_depth_sampler,
                )?
                .apply(rrdevice);
        }
        Ok(())
    }
}
