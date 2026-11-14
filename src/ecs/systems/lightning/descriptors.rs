use anyhow::Result;

use crate::vulkanr::core::RRDevice;
use crate::vulkanr::descriptor::shader_bindings::lightning_resolve;
use crate::vulkanr::descriptor::{
    DescriptorKind, ReflectedDescriptorSets, ReflectedLayoutSpec, SetGrid, SetSlot,
    LIGHTNING_RESOLVE,
};
use crate::vulkanr::image::create_scene_depth_sampler;
use thyllore_effect_core::{LightningSegmentsUBO, LightningUBO};
use thyllore_vulkan_core::resource::{build_gpu_resource, GpuResource, UniformBuffer};
use thyllore_vulkan_core::types::{ImageView, Sampler};

#[must_use]
#[derive(Debug, Default, GpuResource)]
pub struct LightningResolveDescriptorSet {
    pub sets: ReflectedDescriptorSets,
    scene_depth_sampler: Sampler,
}

impl LightningResolveDescriptorSet {
    pub unsafe fn new(rrdevice: &RRDevice) -> Result<Self> {
        let layout_spec = ReflectedLayoutSpec::local(&LIGHTNING_RESOLVE)
            .with_override(
                lightning_resolve::LIGHTNING,
                DescriptorKind::UniformBufferDynamic,
            )
            .with_override(
                lightning_resolve::SEGMENTS,
                DescriptorKind::UniformBufferDynamic,
            );

        build_gpu_resource(rrdevice, |descriptor: &mut Self| {
            descriptor.sets =
                ReflectedDescriptorSets::create(rrdevice, &layout_spec, SetGrid::SINGLE)?;
            descriptor.scene_depth_sampler = create_scene_depth_sampler(rrdevice)?;
            Ok(())
        })
    }

    pub unsafe fn write_all(
        &self,
        rrdevice: &RRDevice,
        lightning_ubo: &UniformBuffer<LightningUBO>,
        segments_ubo: &UniformBuffer<LightningSegmentsUBO>,
        scene_depth_view: ImageView,
    ) -> Result<()> {
        self.sets
            .writer(SetSlot::SINGLE)?
            .uniform_dynamic(lightning_resolve::LIGHTNING, lightning_ubo)?
            .uniform_dynamic(lightning_resolve::SEGMENTS, segments_ubo)?
            .apply(rrdevice);
        self.update_scene_depth(rrdevice, scene_depth_view)
    }

    pub unsafe fn update_scene_depth(
        &self,
        rrdevice: &RRDevice,
        scene_depth_view: ImageView,
    ) -> Result<()> {
        self.sets
            .writer(SetSlot::SINGLE)?
            .sampled_depth(
                lightning_resolve::SCENE_DEPTH_SAMPLER,
                scene_depth_view,
                self.scene_depth_sampler,
            )?
            .apply(rrdevice);
        Ok(())
    }
}
