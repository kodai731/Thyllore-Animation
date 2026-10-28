use anyhow::Result;

use crate::core::device::RRDevice;
use crate::descriptor::pass_manifest::IMGUI;
use crate::descriptor::reflected_layout::{ReflectedLayoutSpec, ReflectedSetLayout};
use crate::descriptor::shader_bindings::imgui;
use crate::types::{DescriptorSet, ImageView, Pipeline, PipelineLayout, Sampler};
use crate::vulkan::*;

pub fn imgui_layout_spec() -> ReflectedLayoutSpec {
    ReflectedLayoutSpec::local(&IMGUI)
}

pub unsafe fn destroy_imgui_pipeline_objects(
    rrdevice: &RRDevice,
    pipeline: Option<Pipeline>,
    pipeline_layout: Option<PipelineLayout>,
    sampler: Option<Sampler>,
) {
    if let Some(pipeline) = pipeline {
        rrdevice.device().destroy_pipeline(pipeline.raw(), None);
    }
    if let Some(pipeline_layout) = pipeline_layout {
        rrdevice
            .device()
            .destroy_pipeline_layout(pipeline_layout.raw(), None);
    }
    if let Some(sampler) = sampler {
        rrdevice.device().destroy_sampler(sampler.raw(), None);
    }
}

pub unsafe fn allocate_imgui_texture_set(
    rrdevice: &RRDevice,
    layout: &ReflectedSetLayout,
) -> Result<DescriptorSet> {
    layout.allocate_set(rrdevice).map(DescriptorSet)
}

pub unsafe fn write_imgui_texture_set(
    rrdevice: &RRDevice,
    layout: &ReflectedSetLayout,
    descriptor_set: DescriptorSet,
    image_view: ImageView,
    sampler: Sampler,
) -> Result<()> {
    layout
        .writer(descriptor_set.raw())
        .sampled_image(imgui::TEX_SAMPLER, image_view.raw(), sampler.raw())?
        .apply(rrdevice);
    Ok(())
}

pub fn imgui_texture_id(descriptor_set: DescriptorSet) -> u64 {
    descriptor_set.raw().as_raw()
}
