use serde::Serialize;
use vulkanalia::prelude::v1_0::*;

use crate::debugview::*;
use crate::platform::ImguiData;
use crate::scene::assets::AssetStorage;
use crate::scene::graphics_resource::GraphicsResources;
use crate::scene::raytracing::RayTracingData;
use crate::scene::world::World;
use crate::scene::Camera;

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum LightMoveTarget {
    None,
    XMin,
    XMax,
    YMin,
    YMax,
    ZMin,
    ZMax,
}

#[derive(Debug, Default)]
pub struct AppData {
    pub graphics_resources: GraphicsResources,
    pub msaa_samples: vk::SampleCountFlags,
    pub camera: Camera,
    pub imgui: ImguiData,
    pub raytracing: RayTracingData,
    pub rt_debug_state: RayTracingDebugState,
    pub debug_view_data: DebugViewData,
    pub ecs_world: World,
    pub ecs_assets: AssetStorage,
}
