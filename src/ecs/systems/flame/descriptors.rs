use anyhow::Result;

use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::pass_manifest::FLAME_RESOLVE;
use crate::vulkanr::descriptor::shader_bindings::flame_resolve;
use crate::vulkanr::descriptor::{
    DescriptorKind, ReflectedDescriptorSets, ReflectedLayoutSpec, ReflectedSetLayout,
};
use crate::vulkanr::image::create_scene_depth_sampler;
use thyllore_effect_core::FlameUBO;
use thyllore_vulkan_core::resource::{GpuResource, UniformBuffer};

const FLAME_HISTORY_SET_COUNT: usize = 2;

#[derive(Clone, Copy, Debug)]
pub struct FlameImageBindings {
    pub history_image_views: [thyllore_vulkan_core::types::ImageView; FLAME_HISTORY_SET_COUNT],
    pub flame_sampler: thyllore_vulkan_core::types::Sampler,
    pub sdf_image_view: thyllore_vulkan_core::types::ImageView,
    pub sdf_sampler: thyllore_vulkan_core::types::Sampler,
    pub scene_depth_view: thyllore_vulkan_core::types::ImageView,
}

#[derive(Clone, Debug, Default)]
pub struct RRFlameDescriptorSet {
    sets: ReflectedDescriptorSets,
    pub scene_depth_sampler: thyllore_vulkan_core::types::Sampler,
}

impl RRFlameDescriptorSet {
    pub fn layout_spec() -> ReflectedLayoutSpec {
        ReflectedLayoutSpec::local(&FLAME_RESOLVE)
            .with_override(flame_resolve::FLAME, DescriptorKind::UniformBufferDynamic)
    }

    pub fn layout(&self) -> &ReflectedSetLayout {
        self.sets.layout()
    }

    pub fn descriptor_set(
        &self,
        history_index: usize,
    ) -> thyllore_vulkan_core::types::DescriptorSet {
        self.sets.set(history_index)
    }

    pub unsafe fn new(rrdevice: &RRDevice) -> Result<Self> {
        let sets = ReflectedDescriptorSets::create(
            rrdevice,
            &Self::layout_spec(),
            FLAME_HISTORY_SET_COUNT,
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
                .writer(history_index)
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
                .writer(history_index)
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

    pub unsafe fn destroy(&mut self, rrdevice: &RRDevice) {
        self.sets.destroy(rrdevice);
        self.scene_depth_sampler.destroy_gpu(rrdevice);
    }
}

impl GpuResource for RRFlameDescriptorSet {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        self.destroy(rrdevice);
    }

    fn is_destroyed(&self) -> bool {
        self.sets.is_destroyed() && self.scene_depth_sampler.is_destroyed()
    }
}
