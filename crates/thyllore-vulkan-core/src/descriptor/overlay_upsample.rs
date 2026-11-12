use crate::core::device::*;
use crate::descriptor::pass_manifest::OVERLAY_UPSAMPLE;
use crate::descriptor::reflected_layout::{ReflectedLayoutSpec, ReflectedSetLayout};
use crate::descriptor::reflected_sets::ReflectedDescriptorSets;
use crate::descriptor::shader_bindings::overlay_upsample;
use crate::resource::gpu_resource::GpuResource;
use crate::resource::image::{create_nearest_sampler, create_scene_depth_sampler};
use crate::vulkan::*;

/// One set per frame slot binding the reduced overlay color and the scene depth of that frame.
#[derive(Clone, Debug, Default)]
pub struct OverlayUpsampleDescriptorSet {
    sets: ReflectedDescriptorSets,
    pub(crate) reduced_color_sampler: vk::Sampler,
    pub(crate) scene_depth_sampler: vk::Sampler,
}

impl OverlayUpsampleDescriptorSet {
    pub fn layout_spec() -> ReflectedLayoutSpec {
        ReflectedLayoutSpec::local(&OVERLAY_UPSAMPLE)
    }

    pub unsafe fn new(rrdevice: &RRDevice, frames_in_flight: usize) -> Result<Self> {
        let sets =
            ReflectedDescriptorSets::create(rrdevice, &Self::layout_spec(), frames_in_flight)?;
        let reduced_color_sampler = create_nearest_sampler(rrdevice)?;
        let scene_depth_sampler = create_scene_depth_sampler(rrdevice)?.raw();

        Ok(Self {
            sets,
            reduced_color_sampler,
            scene_depth_sampler,
        })
    }

    pub fn layout(&self) -> &ReflectedSetLayout {
        self.sets.layout()
    }

    pub fn descriptor_set(&self, frame_slot: usize) -> crate::types::DescriptorSet {
        self.sets.set(frame_slot)
    }

    pub unsafe fn update_image_views_at(
        &self,
        rrdevice: &RRDevice,
        frame_slot: usize,
        reduced_color_view: crate::types::ImageView,
        scene_depth_view: crate::types::ImageView,
    ) -> Result<()> {
        self.sets
            .writer(frame_slot)
            .image(
                overlay_upsample::REDUCED_COLOR_SAMPLER,
                reduced_color_view.raw(),
                self.reduced_color_sampler,
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            )?
            .image(
                overlay_upsample::SCENE_DEPTH_SAMPLER,
                scene_depth_view.raw(),
                self.scene_depth_sampler,
                vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
            )?
            .apply(rrdevice);
        Ok(())
    }

    pub unsafe fn destroy(&mut self, rrdevice: &RRDevice) {
        self.sets.destroy(rrdevice);
        self.reduced_color_sampler.destroy_gpu(rrdevice);
        self.scene_depth_sampler.destroy_gpu(rrdevice);
    }
}

impl GpuResource for OverlayUpsampleDescriptorSet {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        self.destroy(rrdevice);
    }

    fn is_destroyed(&self) -> bool {
        self.sets.is_destroyed()
            && self.reduced_color_sampler.is_destroyed()
            && self.scene_depth_sampler.is_destroyed()
    }
}
