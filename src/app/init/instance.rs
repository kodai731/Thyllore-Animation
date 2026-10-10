use crate::app::{App, AppData};

use super::graphics::VulkanResources;
use crate::ecs::resource::pipeline_allocate_id;
use crate::ecs::systems::create_default_grid_scale;
use crate::ecs::PipelineManager;
use crate::hooks::startup::{run_startup_phase, StartupPhase};
use crate::vulkanr::command::*;
use crate::vulkanr::descriptor::*;
use crate::vulkanr::device::*;
use crate::vulkanr::pipeline::RRPipeline;
use crate::vulkanr::render::*;
use crate::vulkanr::swapchain::*;
use crate::vulkanr::vulkan::*;

use anyhow::{anyhow, Context, Result};
use std::rc::Rc;
use std::time::Instant;

pub use crate::ecs::MAX_FRAMES_IN_FLIGHT;
use vulkanalia::loader::{LibloadingLoader, LIBRARY};
use winit::window::Window;

// Constants
pub const PORTABILITY_MACOS_VERSION: Version = Version::new(1, 3, 216);
pub const VALIDATION_ENABLED: bool = cfg!(debug_assertions);
pub const VALIDATION_MODE: crate::vulkanr::core::device::ValidationMode = if cfg!(debug_assertions)
{
    crate::vulkanr::core::device::ValidationMode::Enabled
} else {
    crate::vulkanr::core::device::ValidationMode::Disabled
};
pub use crate::vulkanr::core::device::VALIDATION_LAYER;
pub const DEVICE_EXTENSIONS: &[vk::ExtensionName] = &[
    vk::KHR_SWAPCHAIN_EXTENSION.name,
    vk::KHR_BUFFER_DEVICE_ADDRESS_EXTENSION.name,
    vk::KHR_ACCELERATION_STRUCTURE_EXTENSION.name,
    vk::KHR_RAY_QUERY_EXTENSION.name,
    vk::KHR_RAY_TRACING_PIPELINE_EXTENSION.name,
    vk::KHR_DEFERRED_HOST_OPERATIONS_EXTENSION.name,
];

impl App {
    pub unsafe fn create(
        window: &Window,
        #[cfg(feature = "ml")] curve_copilot_mode: crate::ml::CurveCopilotMode,
    ) -> Result<Self> {
        let loader = LibloadingLoader::new(LIBRARY)?;
        let entry = Entry::new(loader).map_err(|b| anyhow!("{}", b))?;
        let mut data = AppData::default();
        crate::effect::subscription::subscribe_effects(&mut data.effect_hooks);
        crate::vulkanr::renderer::deferred::register_core_passes(&mut data.pass_graph);
        data.effect_hooks.register_passes(&mut data.pass_graph);

        Self::initialize_core_ecs_resources(&mut data)?;

        #[cfg(feature = "ml")]
        data.ecs_world.insert_resource(curve_copilot_mode);

        let (instance, messenger) = Self::create_instance_with_messenger(window, &entry)?;
        let surface = vk_window::create_surface(&instance, &window, &window)?;
        let rrdevice = RRDevice::new(
            &entry,
            &instance,
            &surface,
            VALIDATION_MODE,
            VALIDATION_LAYER,
            DEVICE_EXTENSIONS,
            PORTABILITY_MACOS_VERSION,
        )?;
        let rrswapchain = RRSwapchain::new(window, &instance, &surface, &rrdevice)?;
        let rrcommand_pool = Rc::new(RRCommandPool::new(&instance, &surface, &rrdevice));
        let rrrender = RRRender::new(&instance, &rrdevice, &rrswapchain, rrcommand_pool.as_ref());

        Self::initialize_graphics_and_ecs(
            &instance,
            &rrdevice,
            &rrswapchain,
            &rrcommand_pool,
            &rrrender,
            &mut data,
        )?;

        let render_layouts = data.graphics_resources.get_layouts();
        let mut pipeline_manager = PipelineManager::new();

        let model_pipeline = RRPipeline::new_with_graphics_resources(
            &rrdevice,
            &rrswapchain,
            &rrrender,
            &render_layouts,
            &MODEL,
            vk::PrimitiveTopology::TRIANGLE_LIST,
            vk::PolygonMode::FILL,
            vk::CullModeFlags::BACK,
        )
        .context("Failed to create model pipeline")?;
        data.pipeline_storage.register(model_pipeline.clone());
        pipeline_allocate_id(&mut pipeline_manager);

        let pipeline_ids = Self::build_gizmo_pipelines(
            &rrdevice,
            &rrswapchain,
            &rrrender,
            &render_layouts,
            &mut data.pipeline_storage,
            &mut pipeline_manager,
        )?;

        Self::initialize_gizmo_resources(
            &instance,
            &rrdevice,
            &rrcommand_pool,
            &rrswapchain,
            &rrrender,
            &pipeline_ids,
            &mut data,
            &mut pipeline_manager,
        )?;

        data.ecs_world.insert_resource(pipeline_manager);

        let grid_object_index = data.graphics_resources.objects.allocate_slot();
        data.graphics_resources.objects.seal_reserved_slots();

        let rrrender = Self::initialize_ray_tracing(
            &instance,
            &rrdevice,
            &rrswapchain,
            &rrcommand_pool,
            &rrrender,
            &mut data,
        )?;

        let (model_path, loaded_scene) = Self::determine_startup_model();
        // Self::load_startup_model(
        //     &instance,
        //     &rrdevice,
        //     &rrcommand_pool,
        //     &rrswapchain,
        //     &mut data,
        //     &model_path,
        //     loaded_scene.is_some(),
        //     );

        // The scene restores timeline, panel, curve editor and post-processing state, so those
        // resources must exist before it is applied. Registration is idempotent and runs again below.
        Self::register_editor_resources(&mut data);
        Self::register_post_processing_resources(&mut data);
        Self::apply_loaded_scene(&mut data, loaded_scene)?;
        data.raytracing.command_pool = rrcommand_pool.command_pool;
        if let Err(e) = Self::create_ray_tracing_pipelines_with_resources(
            &instance,
            &rrdevice,
            &mut data,
            &rrswapchain,
            &rrrender,
        ) {
            log_warn!("Failed to create ray tracing pipelines: {:?}", e);
        }

        if let Err(e) = Self::build_acceleration_structures_with_resources(
            &instance,
            &rrdevice,
            &mut data,
            &rrcommand_pool,
        ) {
            log_warn!("Failed to build acceleration structures: {:?}", e);
        }

        let grid_mesh_data = Self::build_grid_mesh(
            &instance,
            &rrdevice,
            &rrcommand_pool,
            &mut data,
            pipeline_ids.grid,
            grid_object_index,
        )?;

        let mut rrcommand_buffer = RRCommandBuffer::new(&rrcommand_pool);
        if let Err(e) =
            RRCommandBuffer::allocate_command_buffers(&rrdevice, &rrrender, &mut rrcommand_buffer)
        {
            eprintln!("failed to allocate command buffers: {:?}", e);
        }

        let (image_available_semaphores, render_finish_semaphores, in_flight_fences) =
            Self::create_sync_objects(&rrdevice.device)?;

        let vulkan_resources = VulkanResources {
            messenger,
            surface,
            rrswapchain,
            rrrender,
            rrcommand_pool,
            rrcommand_buffer,
            model_pipeline,
            image_available_semaphores,
            render_finish_semaphores,
            in_flight_fences,
        };

        Self::register_resources(
            &mut data,
            &vulkan_resources,
            &model_path,
            rrdevice.msaa_samples,
        );

        let grid_scale = create_default_grid_scale();
        data.ecs_world.insert_resource(grid_mesh_data);
        data.ecs_world.insert_resource(grid_scale);

        let gpu_timestamp_profiler = thyllore_vulkan_core::GpuTimestampProfiler::new(
            &rrdevice.device,
            rrdevice.timestamp_period,
            vulkan_resources.rrswapchain.swapchain_images.len(),
        );

        Ok(Self {
            entry,
            instance,
            rrdevice,
            data,
            frame: 0,
            resized: false,
            start: Instant::now(),
            last_update_time: 0.0,
            last_frame_interval: 0.0,
            gpu_timestamp_profiler,
            last_frame_instant: None,
        })
    }

    fn initialize_core_ecs_resources(data: &mut AppData) -> Result<()> {
        run_startup_phase(&mut data.ecs_world, StartupPhase::CoreResources);
        data.ecs_world
            .insert_resource(crate::hooks::scene::SceneComponentHooks::collect()?);
        data.ecs_world
            .insert_resource(crate::hooks::scene_resource::SceneResourceHooks::collect()?);
        data.ecs_world
            .insert_resource(crate::hooks::model_load::ModelLoadHooks::collect()?);
        data.ecs_world
            .insert_resource(crate::hooks::frame_prep::FramePrepHooks::collect()?);
        data.ecs_world
            .insert_resource(crate::hooks::effect_spawn::EffectSpawnHooks::collect()?);
        data.ecs_world
            .insert_resource(crate::hooks::object_pick::ObjectPickHooks::collect()?);
        data.ecs_world
            .insert_resource(crate::hooks::dispatch_prep::DispatchPrepHooks::collect()?);
        let ui_windows = crate::hooks::ui_window::UiWindows::collect()?;
        ui_windows.init_window_state(&mut data.ecs_world);
        data.ecs_world.insert_resource(ui_windows);
        Ok(())
    }

    fn register_resources(
        data: &mut AppData,
        resources: &VulkanResources,
        model_path: &str,
        msaa_samples: vk::SampleCountFlags,
    ) {
        Self::register_vulkan_resources(data, resources, model_path, msaa_samples);
        Self::register_editor_resources(data);
        Self::register_post_processing_resources(data);
        run_startup_phase(&mut data.ecs_world, StartupPhase::Ml);
    }

    fn register_editor_resources(data: &mut AppData) {
        run_startup_phase(&mut data.ecs_world, StartupPhase::Editor);
    }

    fn register_post_processing_resources(data: &mut AppData) {
        run_startup_phase(&mut data.ecs_world, StartupPhase::PostProcessing);
    }
}
