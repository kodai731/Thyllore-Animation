use serde::Serialize;

use crate::asset::AssetStorage;
use crate::debugview::*;
use crate::ecs::World;
use crate::platform::ImguiData;
use crate::scene::graphics_resource::GraphicsResources;
use crate::scene::raytracing::RayTracingData;

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
    pub imgui: ImguiData,
    pub raytracing: RayTracingData,
    pub debug_view_data: DebugViewData,
    pub ecs_world: World,
    pub ecs_assets: AssetStorage,
}
