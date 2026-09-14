use crate::app::post_process::PostProcessFrameTargets;
use crate::app::viewport::ViewportState;
use crate::asset::AssetStorage;
use crate::ecs::World;
use crate::hooks::effect::EffectHooks;
use crate::hooks::pass::PassGraph;
use crate::platform::ImguiData;
use crate::vulkanr::renderer::onion_skin_buffers::OnionSkinGpuState;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;
use crate::vulkanr::resource::{GpuBufferRegistry, GpuResource, PipelineStorage};
use thyllore_vulkan_core::renderer::{FrameTransients, ImageStateTracker};
use thyllore_vulkan_core::resource::raytracing_data::RayTracingData;

#[derive(Debug, Default, GpuResource)]
pub struct AppData {
    pub graphics_resources: GraphicsResources,
    pub imgui: ImguiData,
    pub raytracing: RayTracingData,
    #[gpu_resource(skip)]
    pub ecs_world: World,
    #[gpu_resource(skip)]
    pub ecs_assets: AssetStorage,
    pub buffer_registry: GpuBufferRegistry,
    pub pipeline_storage: PipelineStorage,
    pub viewport: ViewportState,
    #[gpu_resource(skip)]
    pub effect_hooks: EffectHooks,
    #[gpu_resource(skip)]
    pub pass_graph: PassGraph,
    #[gpu_resource(skip)]
    pub pass_image_states: ImageStateTracker,
    #[gpu_resource(skip)]
    pub frame_transients: FrameTransients,
    #[gpu_resource(skip)]
    pub post_process: PostProcessFrameTargets,
    pub onion_skin_gpu: Option<OnionSkinGpuState>,
}
