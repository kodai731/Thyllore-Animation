use std::rc::Rc;

use cgmath::Vector3;

use crate::app::GUIData;
use crate::asset::AssetStorage;
use crate::debugview::RayTracingDebugState;
use crate::scene::camera::Camera;
use crate::scene::graphics_resource::GraphicsResources;
use crate::scene::raytracing::RayTracingData;
use crate::scene::Scene;
use crate::vulkanr::command::RRCommandPool;
use crate::vulkanr::device::RRDevice;
use crate::vulkanr::vulkan::Instance;

use super::world::World;

pub struct FrameContext<'a> {
    pub instance: &'a Instance,
    pub device: &'a RRDevice,
    pub command_pool: Rc<RRCommandPool>,

    pub time: f32,
    pub delta_time: f32,
    pub image_index: usize,
    pub swapchain_extent: (u32, u32),

    pub camera: &'a mut Camera,
    pub graphics: &'a mut GraphicsResources,
    pub raytracing: &'a mut RayTracingData,
    pub rt_debug: &'a mut RayTracingDebugState,

    pub scene: &'a Scene,

    pub world: &'a mut World,
    pub assets: &'a AssetStorage,

    pub gui_data: &'a mut GUIData,
}

impl<'a> FrameContext<'a> {
    pub fn camera_position(&self) -> Vector3<f32> {
        self.camera.position
    }

    pub fn camera_direction(&self) -> Vector3<f32> {
        self.camera.direction
    }

    pub fn camera_up(&self) -> Vector3<f32> {
        self.camera.up
    }

    pub fn light_position(&self) -> Vector3<f32> {
        self.rt_debug.light_position
    }
}
