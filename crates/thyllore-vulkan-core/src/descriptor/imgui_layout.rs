use crate::core::device::RRDevice;
use crate::descriptor::pass_manifest::IMGUI;
use crate::descriptor::reflected_layout::ReflectedLayoutSpec;
use crate::vulkan::*;

pub fn imgui_layout_spec() -> ReflectedLayoutSpec {
    ReflectedLayoutSpec::local(&IMGUI)
}

pub unsafe fn destroy_imgui_pipeline_objects(
    rrdevice: &RRDevice,
    pipeline: Option<vk::Pipeline>,
    pipeline_layout: Option<vk::PipelineLayout>,
    sampler: Option<vk::Sampler>,
) {
    if let Some(pipeline) = pipeline {
        rrdevice.device.destroy_pipeline(pipeline, None);
    }
    if let Some(pipeline_layout) = pipeline_layout {
        rrdevice
            .device
            .destroy_pipeline_layout(pipeline_layout, None);
    }
    if let Some(sampler) = sampler {
        rrdevice.device.destroy_sampler(sampler, None);
    }
}
