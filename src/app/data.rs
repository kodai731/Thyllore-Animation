use serde::Serialize;

use crate::app::graphics_resource::GraphicsResources;
use crate::app::raytracing::RayTracingData;
use crate::app::viewport::ViewportState;
use crate::asset::AssetStorage;
use crate::debugview::*;
use crate::ecs::World;
use crate::platform::ImguiData;
use crate::renderer::onion_skin_buffers::OnionSkinGpuState;
use crate::vulkanr::resource::{GpuBufferRegistry, PipelineStorage};

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
    pub buffer_registry: GpuBufferRegistry,
    pub pipeline_storage: PipelineStorage,
    pub viewport: ViewportState,
    pub onion_skin_gpu: Option<OnionSkinGpuState>,
}
