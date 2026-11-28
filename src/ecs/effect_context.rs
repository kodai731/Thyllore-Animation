use crate::ecs::world::World;
use thyllore_vulkan_core::core::RRDevice;
use thyllore_vulkan_core::renderer::ImageStateTracker;
use thyllore_vulkan_core::resource::GraphicsResources;
use thyllore_vulkan_core::resource::RayTracingData;
use thyllore_vulkan_core::resource::RenderTargetStorage;
use thyllore_vulkan_core::types::{ImageView, Instance};

pub struct EffectContext<'a> {
    pub instance: &'a Instance,
    pub rrdevice: &'a RRDevice,
    pub graphics: &'a GraphicsResources,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub hdr_color_view: Option<ImageView>,
    pub storage: &'a mut RenderTargetStorage,
    pub raytracing: &'a mut RayTracingData,
    pub pass_image_states: &'a mut ImageStateTracker,
    pub world: &'a mut World,
}
