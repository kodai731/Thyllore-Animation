use crate::app::{App, AppData};

use crate::ecs::component::RenderInfo;
use crate::ecs::resource::pipeline_allocate_id;
use crate::ecs::resource::GridMeshData;
use crate::ecs::systems::{create_default_grid_scale, create_grid_mesh};
use crate::ecs::{
    ClipLibrary, GpuDescriptors, MaterialRegistry, MeshAssets, ModelState, NodeAssets,
    PipelineManager, SceneState, TimelineState,
};
use crate::hooks::startup::{run_startup_phase, StartupPhase};
use crate::vulkanr::command::*;
use crate::vulkanr::context::{
    CommandState, FrameSync, PipelineState, RenderConfig, RenderTargets, SurfaceState,
    SwapchainState,
};
use crate::vulkanr::descriptor::*;
use crate::vulkanr::device::*;
use crate::vulkanr::pipeline::{
    BlendConfig, DepthTestConfig, PipelineBuilder, RRPipeline, VertexInputConfig,
};
use crate::vulkanr::render::*;
use crate::vulkanr::swapchain::*;
use crate::vulkanr::vulkan::*;

use crate::vulkanr::resource::graphics_resource::GraphicsResources;

use vulkanalia::Device as VkDevice;

use anyhow::{anyhow, Context, Result};
use std::collections::HashSet;
use std::ffi::CStr;
use std::os::raw::c_void;
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

/// Clean up old screenshot files from the log directory
pub fn cleanup_old_screenshots() -> Result<()> {
    use std::fs;
    use std::path::PathBuf;

    let log_dir = PathBuf::from("log");

    if !log_dir.exists() {
        return Ok(());
    }

    let entries = fs::read_dir(&log_dir)?;

    let mut deleted_count = 0;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(filename) = path.file_name() {
                if let Some(filename_str) = filename.to_str() {
                    if filename_str.starts_with("screenshot_") {
                        fs::remove_file(&path)?;
                        deleted_count += 1;
                        log!("Deleted old screenshot: {:?}", filename_str);
                    }
                }
            }
        }
    }

    if deleted_count > 0 {
        log!("Cleaned up {} old screenshot(s)", deleted_count);
    }

    Ok(())
}

struct VulkanResources {
    messenger: vk::DebugUtilsMessengerEXT,
    surface: vk::SurfaceKHR,
    rrswapchain: RRSwapchain,
    rrrender: RRRender,
    rrcommand_pool: Rc<RRCommandPool>,
    rrcommand_buffer: RRCommandBuffer,
    model_pipeline: RRPipeline,
    image_available_semaphores: Vec<vk::Semaphore>,
    render_finish_semaphores: Vec<vk::Semaphore>,
    in_flight_fences: Vec<vk::Fence>,
}

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
        Ok(())
    }
    unsafe fn initialize_graphics_and_ecs(
        instance: &Instance,
        rrdevice: &RRDevice,
        rrswapchain: &RRSwapchain,
        rrcommand_pool: &Rc<RRCommandPool>,
        rrrender: &RRRender,
        data: &mut AppData,
    ) -> Result<()> {
        let swapchain_image_count = rrswapchain.swapchain_images.len();
        data.graphics_resources =
            GraphicsResources::new(instance, rrdevice, swapchain_image_count, 64)
                .context("Failed to create render resources")?;

        let gpu_descriptors = GpuDescriptors::new(
            data.graphics_resources.frame_set.clone(),
            data.graphics_resources.objects.clone(),
        );
        let material_registry = MaterialRegistry::new(data.graphics_resources.materials.clone());
        data.ecs_world.insert_resource(gpu_descriptors);
        data.ecs_world.insert_resource(material_registry);
        data.ecs_world.insert_resource(ClipLibrary::new());
        data.ecs_world.insert_resource(ModelState::default());
        data.ecs_world.insert_resource(MeshAssets::new());
        data.ecs_world.insert_resource(NodeAssets::new());

        let viewport_width = rrswapchain.swapchain_extent.width;
        let viewport_height = rrswapchain.swapchain_extent.height;
        data.viewport = crate::app::viewport::ViewportState::new(
            instance,
            rrdevice,
            rrcommand_pool.command_pool,
            viewport_width,
            viewport_height,
            rrdevice.msaa_samples,
            rrswapchain.swapchain_format,
        )
        .context("Failed to create viewport state")?;

        log!(
            "Created viewport state: {}x{} with MSAA {:?}, format {:?}",
            viewport_width,
            viewport_height,
            rrdevice.msaa_samples,
            rrswapchain.swapchain_format
        );

        let render_layouts = data.graphics_resources.get_layouts();
        if let Some(ref hdr_buffer) = data.viewport.hdr_buffer {
            let hdr_grid = PipelineBuilder::from_pass(&GRID)
                .vertex_input(VertexInputConfig::Gizmo)
                .topology(vk::PrimitiveTopology::LINE_LIST)
                .polygon_mode(vk::PolygonMode::LINE)
                .depth_test(DepthTestConfig {
                    test_enable: true,
                    write_enable: true,
                    compare_op: vk::CompareOp::GREATER_OR_EQUAL,
                })
                .custom_render_pass(hdr_buffer.render_pass)
                .msaa_samples(vk::SampleCountFlags::_1)
                .descriptor_layouts(&render_layouts)
                // Opaque surface inside the HDR buffer: alpha 1 marks "background fully
                // covered", which the tonemap needs to keep the grid color. The flame
                // composites over it afterwards with premultiplied blending.
                .blend(BlendConfig {
                    enable: true,
                    src_color_factor: vk::BlendFactor::SRC_ALPHA,
                    dst_color_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
                    color_op: vk::BlendOp::ADD,
                    src_alpha_factor: vk::BlendFactor::ONE,
                    dst_alpha_factor: vk::BlendFactor::ZERO,
                    alpha_op: vk::BlendOp::ADD,
                })
                .build(rrdevice, &rrrender, Some(rrswapchain.swapchain_extent))
                .context("Failed to create HDR grid pipeline")?;
            let hdr_grid_id = data.pipeline_storage.register(hdr_grid);
            data.viewport.hdr_grid_pipeline_id = Some(hdr_grid_id);
        }

        Ok(())
    }

    unsafe fn initialize_ray_tracing(
        instance: &Instance,
        rrdevice: &RRDevice,
        rrswapchain: &RRSwapchain,
        rrcommand_pool: &Rc<RRCommandPool>,
        rrrender: &RRRender,
        data: &mut AppData,
    ) -> Result<RRRender> {
        let mut rrrender_mut = rrrender.clone();
        match Self::init_ray_tracing_with_resources(
            instance,
            rrdevice,
            data,
            rrswapchain,
            rrcommand_pool.as_ref(),
            &mut rrrender_mut,
        ) {
            Ok(_) => {
                log!("init_ray_tracing succeeded");
            }
            Err(e) => {
                log_warn!("Failed to initialize ray tracing: {:?}", e);
            }
        }
        Ok(rrrender_mut)
    }

    unsafe fn load_startup_model(
        instance: &Instance,
        rrdevice: &RRDevice,
        rrcommand_pool: &Rc<RRCommandPool>,
        rrswapchain: &RRSwapchain,
        data: &mut AppData,
        model_path: &str,
        has_scene: bool,
    ) {
        if let Err(e) = Self::load_model_from_path_with_resources(
            instance,
            rrdevice,
            data,
            rrcommand_pool,
            rrswapchain,
            model_path,
            has_scene,
        ) {
            eprintln!("Failed to load model: {:?}", e);
            log_error!("Failed to load model: {:?}", e);
        }
        log!("loaded initial model: {}", model_path);
    }

    fn apply_loaded_scene(
        data: &mut AppData,
        loaded_scene: Option<(
            std::path::PathBuf,
            crate::scene::LoadedScene,
            Vec<crate::animation::editable::EditableAnimationClip>,
        )>,
    ) -> anyhow::Result<()> {
        let mut scene_state = SceneState::new();
        if let Some((scene_path, scene, clips)) = loaded_scene {
            crate::ecs::systems::clip_library_systems::clip_library_register_loaded(
                &mut data.ecs_world,
                &mut data.ecs_assets,
                clips,
            );
            crate::scene::apply_loaded_scene_to_world(
                &scene,
                &mut data.ecs_world,
                &mut data.ecs_assets,
            );

            let active_clip_id = {
                let timeline = data.ecs_world.resource::<TimelineState>();
                timeline.current_clip_id
            };

            if let Some(clip_id) = active_clip_id {
                let schedule =
                    crate::app::model::build_initial_clip_schedule(Some(clip_id), &data.ecs_world);
                for (_, existing) in data
                    .ecs_world
                    .iter_components_mut::<crate::ecs::component::ClipSchedule>()
                {
                    *existing = schedule.clone();
                }
            }

            scene_state.set_from_loaded(scene_path, scene.scene.metadata.clone());
        } else {
            crate::hooks::effect_spawn::spawn_empty_scene_defaults(
                &mut data.ecs_world,
                &mut data.ecs_assets,
            );
        }
        data.ecs_world.insert_resource(scene_state);
        Ok(())
    }

    unsafe fn build_grid_mesh(
        instance: &Instance,
        rrdevice: &RRDevice,
        rrcommand_pool: &Rc<RRCommandPool>,
        data: &mut AppData,
        grid_pipeline_id: usize,
        grid_object_index: usize,
    ) -> Result<GridMeshData> {
        let (mut grid_mesh, xz_only_index_count) = create_grid_mesh();
        let grid_scale = create_default_grid_scale();

        grid_mesh.vertex_buffer_handles[0] = data.buffer_registry.create_vertex_buffer(
            instance,
            rrdevice,
            rrcommand_pool,
            &grid_mesh.vertices,
            crate::render::BufferMemoryType::DeviceLocal,
        )?;
        grid_mesh.last_written_slot = 0;

        grid_mesh.index_buffer_handles[0] = data.buffer_registry.create_index_buffer(
            instance,
            rrdevice,
            rrcommand_pool,
            &grid_mesh.indices,
        )?;

        Ok(GridMeshData {
            mesh: grid_mesh,
            render_info: RenderInfo::new(Some(grid_pipeline_id), grid_object_index),
            scale: grid_scale,
            show_y_axis_grid: false,
            xz_only_index_count,
        })
    }

    extern "system" fn debug_callback(
        severity: vk::DebugUtilsMessageSeverityFlagsEXT,
        type_: vk::DebugUtilsMessageTypeFlagsEXT,
        data: *const vk::DebugUtilsMessengerCallbackDataEXT,
        _: *mut c_void,
    ) -> vk::Bool32 {
        let data = unsafe { *data };
        let message = if data.message.is_null() {
            std::borrow::Cow::Borrowed("(no message)")
        } else {
            unsafe { CStr::from_ptr(data.message) }.to_string_lossy()
        };

        // コンソール（色付き）とログファイルの両方に出力
        use log::{debug, error, trace, warn};
        if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::ERROR {
            error!("({:?}) {}", type_, message);
            log_error!("({:?}) {}", type_, message);
        } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::WARNING {
            warn!("({:?}) {}", type_, message);
            log_warn!("({:?}) {}", type_, message);
        } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::INFO {
            debug!("({:?}) {}", type_, message);
            log!("({:?}) {}", type_, message);
        } else {
            trace!("({:?}) {}", type_, message);
            log!("DEBUG ({:?}) {}", type_, message);
        }

        vk::FALSE
    }

    unsafe fn create_instance_with_messenger(
        window: &Window,
        entry: &Entry,
    ) -> Result<(Instance, vk::DebugUtilsMessengerEXT)> {
        let application_info = vk::ApplicationInfo::builder()
            .application_name(b"Vulkan Tutorial\0")
            .application_version(vk::make_version(1, 0, 0))
            .engine_name(b"No Engine\0")
            .engine_version(vk::make_version(1, 0, 0))
            .api_version(vk::make_version(1, 2, 0));

        let mut extensions = vk_window::get_required_instance_extensions(window)
            .iter()
            .map(|e| e.as_ptr())
            .collect::<Vec<_>>();

        if VALIDATION_ENABLED {
            extensions.push(vk::EXT_DEBUG_UTILS_EXTENSION.name.as_ptr());
        }

        let flags = if cfg!(target_os = "macos") && entry.version()? >= PORTABILITY_MACOS_VERSION {
            log!("Enabling extensions for macOS portability.");
            extensions.push(
                vk::KHR_GET_PHYSICAL_DEVICE_PROPERTIES2_EXTENSION
                    .name
                    .as_ptr(),
            );
            extensions.push(vk::KHR_PORTABILITY_ENUMERATION_EXTENSION.name.as_ptr());
            vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
        } else {
            vk::InstanceCreateFlags::empty()
        };

        let available_layers = entry
            .enumerate_instance_layer_properties()?
            .iter()
            .map(|l| l.layer_name)
            .collect::<HashSet<_>>();

        if VALIDATION_ENABLED && !available_layers.contains(&VALIDATION_LAYER) {
            return Err(anyhow!("Validation layer requested but not supported"));
        }

        let layers = if VALIDATION_ENABLED {
            vec![VALIDATION_LAYER.as_ptr()]
        } else {
            Vec::new()
        };

        let mut info = vk::InstanceCreateInfo::builder()
            .application_info(&application_info)
            .enabled_layer_names(&layers)
            .enabled_extension_names(&extensions)
            .flags(flags);

        let mut debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::all())
            .message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
            .user_callback(Some(Self::debug_callback));

        if VALIDATION_ENABLED {
            info = info.push_next(&mut debug_info);
        }

        let instance = entry.create_instance(&info, None)?;

        let messenger = if VALIDATION_ENABLED {
            instance.create_debug_utils_messenger_ext(&debug_info, None)?
        } else {
            vk::DebugUtilsMessengerEXT::null()
        };

        Ok((instance, messenger))
    }

    unsafe fn create_sync_objects(
        device: &VkDevice,
    ) -> Result<(Vec<vk::Semaphore>, Vec<vk::Semaphore>, Vec<vk::Fence>)> {
        let semaphore_info = vk::SemaphoreCreateInfo::builder();
        let fence_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);

        let mut image_available = Vec::new();
        let mut render_finished = Vec::new();
        let mut in_flight = Vec::new();

        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            image_available.push(device.create_semaphore(&semaphore_info, None)?);
            render_finished.push(device.create_semaphore(&semaphore_info, None)?);
            in_flight.push(device.create_fence(&fence_info, None)?);
        }

        Ok((image_available, render_finished, in_flight))
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

    fn register_vulkan_resources(
        data: &mut AppData,
        resources: &VulkanResources,
        model_path: &str,
        msaa_samples: vk::SampleCountFlags,
    ) {
        data.ecs_world.insert_resource(FrameSync::new(
            resources.image_available_semaphores.clone(),
            resources.render_finish_semaphores.clone(),
            resources.in_flight_fences.clone(),
        ));

        data.ecs_world.insert_resource(SwapchainState::new(
            resources.rrswapchain.clone(),
            resources.rrswapchain.swapchain_images.len(),
        ));

        data.ecs_world
            .insert_resource(RenderTargets::new(resources.rrrender.clone()));

        data.ecs_world.insert_resource(CommandState::new(
            resources.rrcommand_pool.clone(),
            resources.rrcommand_buffer.clone(),
        ));

        data.ecs_world
            .insert_resource(PipelineState::new(resources.model_pipeline.clone()));

        data.ecs_world
            .insert_resource(SurfaceState::new(resources.surface, resources.messenger));

        {
            let mut model_state = data.ecs_world.resource_mut::<ModelState>();
            if model_state.model_path.is_empty() {
                model_state.model_path = model_path.to_string();
            }
        }

        if !data.ecs_world.contains_resource::<RenderConfig>() {
            data.ecs_world
                .insert_resource(RenderConfig::new(msaa_samples));
        }
    }

    fn register_editor_resources(data: &mut AppData) {
        run_startup_phase(&mut data.ecs_world, StartupPhase::Editor);
    }

    fn register_post_processing_resources(data: &mut AppData) {
        run_startup_phase(&mut data.ecs_world, StartupPhase::PostProcessing);
    }

    fn determine_startup_model() -> (
        String,
        Option<(
            std::path::PathBuf,
            crate::scene::LoadedScene,
            Vec<crate::animation::editable::EditableAnimationClip>,
        )>,
    ) {
        use crate::scene::{find_default_scene, load_scene};

        // let default_model_path = "assets/models/stickman/stickman.glb".to_string();

        // Check for --batch-scene flag in command line arguments
        let args: Vec<String> = std::env::args().collect();
        if let Some(pos) = args.iter().position(|a| a == "--batch-scene") {
            if let Some(path_str) = args.get(pos + 1) {
                let scene_path = std::path::PathBuf::from(path_str);
                if scene_path.exists() {
                    match load_scene(&scene_path) {
                        Ok(loaded) => {
                            let model_path = loaded
                                .model_path
                                .as_ref()
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or_default();
                            let clips = loaded.clips.clone();
                            log!("Loaded batch scene from: {}", scene_path.display());
                            return (model_path, Some((scene_path, loaded, clips)));
                        }
                        Err(e) => {
                            log_error!("Failed to load batch scene: {:?}", e);
                        }
                    }
                } else {
                    log_error!("Batch scene path does not exist: {}", scene_path.display());
                }
            } else {
                log_error!("--batch-scene flag is missing the path argument");
            }
        }

        if let Some(scene_path) = find_default_scene() {
            match load_scene(&scene_path) {
                Ok(loaded) => {
                    // Loading uses the resolved assets path from LoadedScene.model_path (None for
                    // a generated mesh with no file). Saving still uses loaded.scene.model.path.
                    let model_path = loaded
                        .model_path
                        .as_ref()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let clips = loaded.clips.clone();
                    log!("Loaded default scene from: {}", scene_path.display());
                    return (model_path, Some((scene_path, loaded, clips)));
                }
                Err(e) => {
                    log_error!("Failed to load default scene: {:?}", e);
                }
            }
        }

        // (default_model_path, None)
        ("".to_string(), None)
    }
}
