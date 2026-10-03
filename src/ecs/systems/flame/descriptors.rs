use crate::vulkanr::core::*;
use crate::vulkanr::descriptor::pass_manifest::FLAME_RESOLVE;
use crate::vulkanr::descriptor::shader_bindings::flame_resolve;
use crate::vulkanr::descriptor::{
    ReflectedDescriptorSets, ReflectedLayoutSpec, ReflectedSetLayout,
};
use crate::vulkanr::image::create_scene_depth_sampler;
use crate::vulkanr::resource::{GpuResource, UniformBuffer};
use thyllore_effect_core::FlameUBO;
use thyllore_vulkan_core::vulkan::*;

const FLAME_HISTORY_SET_COUNT: usize = 2;

#[derive(Clone, Copy, Debug)]
pub struct FlameImageBindings {
    pub history_image_views: [vk::ImageView; FLAME_HISTORY_SET_COUNT],
    pub flame_sampler: vk::Sampler,
    pub sdf_image_view: vk::ImageView,
    pub sdf_sampler: vk::Sampler,
    pub scene_depth_view: vk::ImageView,
}

#[derive(Clone, Debug, Default)]
pub struct RRFlameDescriptorSet {
    sets: ReflectedDescriptorSets,
    pub scene_depth_sampler: vk::Sampler,
}

impl RRFlameDescriptorSet {
    pub fn layout_spec() -> ReflectedLayoutSpec {
        ReflectedLayoutSpec::local(&FLAME_RESOLVE).with_override(
            flame_resolve::FLAME,
            vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC,
        )
    }

    pub fn layout(&self) -> &ReflectedSetLayout {
        self.sets.layout()
    }

    pub fn descriptor_set(&self, history_index: usize) -> vk::DescriptorSet {
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
                .image(
                    flame_resolve::FLAME_HISTORY_SAMPLER,
                    previous_history_view,
                    images.flame_sampler,
                    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                )?
                .image(
                    flame_resolve::FLAME_SDF_SAMPLER,
                    images.sdf_image_view,
                    images.sdf_sampler,
                    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                )?
                .image(
                    flame_resolve::SCENE_DEPTH_SAMPLER,
                    images.scene_depth_view,
                    self.scene_depth_sampler,
                    vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
                )?
                .apply(rrdevice);
        }
        Ok(())
    }

    pub unsafe fn destroy(&mut self, device: &vulkanalia::Device) {
        self.sets.destroy(device);
        device.destroy_sampler(self.scene_depth_sampler, None);
    }
}

impl GpuResource for RRFlameDescriptorSet {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        self.destroy(&rrdevice.device);
    }
}
