use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::core::RRDevice;
use crate::descriptor::shader_bindings::lightning_resolve;
use crate::descriptor::{ReflectedLayoutSpec, ReflectedSetLayout, LIGHTNING_RESOLVE};
use crate::resource::image::create_scene_depth_sampler;
use crate::resource::GpuResource;
use crate::resource::UniformBuffer;
use thyllore_effect_core::{LightningSegmentsUBO, LightningUBO};

#[derive(Clone, Debug, Default)]
pub struct LightningResolveDescriptorSet {
    pub layout: ReflectedSetLayout,
    pub(crate) descriptor_set: vk::DescriptorSet,
    pub scene_depth_sampler: vk::Sampler,
}

impl LightningResolveDescriptorSet {
    pub fn layout_spec() -> ReflectedLayoutSpec {
        ReflectedLayoutSpec::local(&LIGHTNING_RESOLVE)
            .with_override(
                lightning_resolve::LIGHTNING,
                crate::descriptor::reflected_layout::DescriptorKind::UniformBufferDynamic,
            )
            .with_override(
                lightning_resolve::SEGMENTS,
                crate::descriptor::reflected_layout::DescriptorKind::UniformBufferDynamic,
            )
    }

    pub unsafe fn new(rrdevice: &RRDevice) -> Result<Self> {
        let layout = ReflectedSetLayout::create(rrdevice, &Self::layout_spec())?;
        let descriptor_set = layout.allocate_set(rrdevice)?;
        let scene_depth_sampler = create_scene_depth_sampler(rrdevice)?;

        Ok(Self {
            layout,
            descriptor_set,
            scene_depth_sampler,
        })
    }

    pub fn descriptor_set(&self) -> crate::types::DescriptorSet {
        crate::types::DescriptorSet(self.descriptor_set)
    }

    pub unsafe fn write_all(
        &self,
        rrdevice: &RRDevice,
        lightning_ubo: &UniformBuffer<LightningUBO>,
        segments_ubo: &UniformBuffer<LightningSegmentsUBO>,
        scene_depth_view: crate::types::ImageView,
    ) -> Result<()> {
        self.layout
            .writer(self.descriptor_set)
            .uniform_dynamic(lightning_resolve::LIGHTNING, lightning_ubo)?
            .uniform_dynamic(lightning_resolve::SEGMENTS, segments_ubo)?
            .apply(rrdevice);
        self.update_scene_depth(rrdevice, scene_depth_view)
    }

    pub unsafe fn update_scene_depth(
        &self,
        rrdevice: &RRDevice,
        scene_depth_view: crate::types::ImageView,
    ) -> Result<()> {
        self.layout
            .writer(self.descriptor_set)
            .sampled_depth(
                lightning_resolve::SCENE_DEPTH_SAMPLER,
                scene_depth_view,
                crate::types::Sampler(self.scene_depth_sampler),
            )?
            .apply(rrdevice);
        Ok(())
    }

    pub unsafe fn destroy(&mut self, rrdevice: &RRDevice) {
        self.layout.destroy(rrdevice);
        self.scene_depth_sampler.destroy_gpu(rrdevice);
    }
}

impl GpuResource for LightningResolveDescriptorSet {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        self.destroy(rrdevice);
    }

    fn is_destroyed(&self) -> bool {
        self.layout.is_destroyed() && self.scene_depth_sampler.is_destroyed()
    }
}
