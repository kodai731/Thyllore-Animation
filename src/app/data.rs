use std::rc::Rc;

use crate::app::viewport::ViewportState;
use crate::asset::AssetStorage;
use crate::ecs::world::World;
use crate::ecs::GpuSceneContext;
use crate::hooks::effect::EffectHooks;
use crate::hooks::pass::PassGraph;
use crate::platform::ImguiData;
use crate::vulkanr::renderer::onion_skin_buffers::OnionSkinGpuState;
use thyllore_vulkan_core::command::RRCommandPool;
use thyllore_vulkan_core::core::RRDevice;
use thyllore_vulkan_core::renderer::{FrameTransients, ImageStateTracker};
use thyllore_vulkan_core::resource::GraphicsResources;
use thyllore_vulkan_core::resource::RayTracingData;
use thyllore_vulkan_core::resource::{GpuBufferRegistry, GpuResource, PipelineStorage};
use thyllore_vulkan_core::types::Instance;

#[must_use]
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
    pub onion_skin_gpu: Option<OnionSkinGpuState>,
}

impl AppData {
    /// Splits the aggregate into what a scene rebuild borrows: GPU side, `World` and assets.
    pub fn scene_parts<'a>(
        &'a mut self,
        instance: &'a Instance,
        device: &'a RRDevice,
        command_pool: &'a Rc<RRCommandPool>,
    ) -> (GpuSceneContext<'a>, &'a mut World, &'a mut AssetStorage) {
        let gpu = GpuSceneContext {
            instance,
            device,
            command_pool,
            graphics: &mut self.graphics_resources,
            raytracing: &mut self.raytracing,
            onion_skin: &mut self.onion_skin_gpu,
        };
        (gpu, &mut self.ecs_world, &mut self.ecs_assets)
    }
}
