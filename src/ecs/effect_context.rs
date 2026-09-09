use vulkanalia::prelude::v1_0::*;

pub const MAX_FRAMES_IN_FLIGHT: usize = 2;

use crate::ecs::World;
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::resource::{RenderTargetStorage, RenderTargetTransient};
use thyllore_vulkan_core::renderer::ImageStateTracker;
use thyllore_vulkan_core::resource::raytracing_data::RayTracingData;

pub struct EffectContext<'a> {
    pub instance: &'a Instance,
    pub rrdevice: &'a RRDevice,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub hdr_color_view: Option<vk::ImageView>,
    pub storage: &'a mut RenderTargetStorage,
    pub transient: &'a mut RenderTargetTransient,
    pub raytracing: &'a mut RayTracingData,
    pub pass_image_states: &'a mut ImageStateTracker,
    pub world: &'a mut World,
    pub frames_in_flight: usize,
}
