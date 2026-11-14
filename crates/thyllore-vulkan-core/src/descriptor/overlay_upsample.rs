use anyhow::Result;

use crate::core::device::RRDevice;
use crate::descriptor::pass_manifest::OVERLAY_UPSAMPLE;
use crate::descriptor::shader_bindings::overlay_upsample;
use crate::descriptor::{ReflectedDescriptorSets, ReflectedLayoutSpec, SetGrid, SetSlot};
use crate::resource::gpu_resource::{build_gpu_resource, GpuResource};
use crate::resource::image::{create_nearest_sampler, create_scene_depth_sampler};
use crate::types::{ImageView, Sampler};

/// One set per frame slot binding the reduced overlay color and the scene depth of that frame.
#[must_use]
#[derive(Debug, Default, GpuResource)]
pub struct OverlayUpsampleDescriptorSet {
    pub sets: ReflectedDescriptorSets,
    reduced_color_sampler: Sampler,
    scene_depth_sampler: Sampler,
}

impl OverlayUpsampleDescriptorSet {
    pub unsafe fn new(rrdevice: &RRDevice, frames_in_flight: usize) -> Result<Self> {
        build_gpu_resource(rrdevice, |descriptor: &mut Self| {
            descriptor.sets = ReflectedDescriptorSets::create(
                rrdevice,
                &ReflectedLayoutSpec::local(&OVERLAY_UPSAMPLE),
                SetGrid::per_frame_slot(frames_in_flight),
            )?;
            descriptor.reduced_color_sampler = Sampler(create_nearest_sampler(rrdevice)?);
            descriptor.scene_depth_sampler = create_scene_depth_sampler(rrdevice)?;
            Ok(())
        })
    }

    pub unsafe fn update_image_views_at(
        &self,
        rrdevice: &RRDevice,
        frame_slot: usize,
        reduced_color_view: ImageView,
        scene_depth_view: ImageView,
    ) -> Result<()> {
        self.sets
            .writer(SetSlot::frame_slot(frame_slot))?
            .sampled_image(
                overlay_upsample::REDUCED_COLOR_SAMPLER,
                reduced_color_view,
                self.reduced_color_sampler,
            )?
            .sampled_depth(
                overlay_upsample::SCENE_DEPTH_SAMPLER,
                scene_depth_view,
                self.scene_depth_sampler,
            )?
            .apply(rrdevice);
        Ok(())
    }
}
