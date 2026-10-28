use crate::core::*;
use crate::descriptor::pass_manifest::FLAME_RESOLVE;
use crate::descriptor::shader_bindings::flame_resolve;
use crate::descriptor::{ReflectedLayoutSpec, ReflectedSetLayout};
use crate::resource::image::create_scene_depth_sampler;
use crate::resource::{GpuResource, UniformBuffer};
use crate::vulkan::*;
use thyllore_effect_core::FlameUBO;

const FLAME_HISTORY_SET_COUNT: usize = 2;

#[derive(Clone, Copy, Debug)]
pub struct FlameImageBindings {
    pub history_image_views: [crate::types::ImageView; FLAME_HISTORY_SET_COUNT],
    pub flame_sampler: crate::types::Sampler,
    pub sdf_image_view: crate::types::ImageView,
    pub sdf_sampler: crate::types::Sampler,
    pub scene_depth_view: crate::types::ImageView,
}

#[derive(Clone, Debug, Default)]
pub struct RRFlameDescriptorSet {
    pub layout: ReflectedSetLayout,
    pub(crate) descriptor_sets: [vk::DescriptorSet; FLAME_HISTORY_SET_COUNT],
    pub scene_depth_sampler: vk::Sampler,
}

impl RRFlameDescriptorSet {
    pub fn layout_spec() -> ReflectedLayoutSpec {
        ReflectedLayoutSpec::local(&FLAME_RESOLVE).with_override(
            flame_resolve::FLAME,
            crate::descriptor::reflected_layout::DescriptorKind::UniformBufferDynamic,
        )
    }

    pub unsafe fn new(rrdevice: &RRDevice) -> Result<Self> {
        let layout = ReflectedSetLayout::create(rrdevice, &Self::layout_spec())?;
        let sets = layout.allocate_sets(rrdevice, FLAME_HISTORY_SET_COUNT)?;
        let scene_depth_sampler = create_scene_depth_sampler(rrdevice)?.raw();

        Ok(Self {
            layout,
            descriptor_sets: [sets[0], sets[1]],
            scene_depth_sampler,
        })
    }

    pub fn descriptor_set(&self, history_index: usize) -> crate::types::DescriptorSet {
        crate::types::DescriptorSet(self.descriptor_sets[history_index])
    }

    pub unsafe fn write_all(
        &self,
        rrdevice: &RRDevice,
        flame_ubo: &UniformBuffer<FlameUBO>,
        images: FlameImageBindings,
    ) -> Result<()> {
        for descriptor_set in self.descriptor_sets {
            self.layout
                .writer(descriptor_set)
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
        for (history_index, descriptor_set) in self.descriptor_sets.into_iter().enumerate() {
            let previous_history_view = images.history_image_views[1 - history_index];
            self.layout
                .writer(descriptor_set)
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
                    crate::types::Sampler(self.scene_depth_sampler),
                )?
                .apply(rrdevice);
        }
        Ok(())
    }

    pub unsafe fn destroy(&mut self, rrdevice: &RRDevice) {
        self.layout.destroy(rrdevice);
        self.scene_depth_sampler.destroy_gpu(rrdevice);
    }
}

impl GpuResource for RRFlameDescriptorSet {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        self.destroy(rrdevice);
    }

    fn is_destroyed(&self) -> bool {
        self.layout.is_destroyed() && self.scene_depth_sampler.is_destroyed()
    }
}
