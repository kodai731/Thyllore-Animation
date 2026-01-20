use crate::app::{App, AppData};

use crate::ecs::systems::{
    billboard_create_buffers, create_billboard, create_grid_gizmo, create_light_gizmo,
    gizmo_create_buffers,
};
use crate::ecs::{
    AnimationPlayback, AnimationRegistry, GpuDescriptors, MaterialRegistry, MeshAssets, ModelState,
    NodeAssets, PipelineManager,
};
use crate::vulkanr::command::*;
use crate::vulkanr::context::{
    CommandState, FrameSync, PipelineState, RenderConfig, RenderTargets, SurfaceState,
    SwapchainState,
};
use crate::vulkanr::data::*;
use crate::vulkanr::descriptor::*;
use crate::vulkanr::device::*;
use crate::vulkanr::pipeline::{PipelineBuilder, RRPipeline, VertexInputConfig};
use crate::vulkanr::render::*;
use crate::vulkanr::swapchain::*;
use crate::vulkanr::vulkan::*;

use crate::debugview::*;
use crate::math::*;
use crate::scene::graphics_resource::GraphicsResources;
use crate::scene::grid::GridData;
use crate::scene::Camera;

use vulkanalia::Device as VkDevice;

use anyhow::{anyhow, Result};
use std::collections::HashSet;
use std::ffi::CStr;
use std::os::raw::c_void;
use std::ptr::copy_nonoverlapping as memcpy;
use std::rc::Rc;
use std::time::Instant;

use vulkanalia::loader::{LibloadingLoader, LIBRARY};
use winit::window::Window;

// Constants
pub const PORTABILITY_MACOS_VERSION: Version = Version::new(1, 3, 216);
pub const VALIDATION_ENABLED: bool = cfg!(debug_assertions);
pub const VALIDATION_LAYER: vk::ExtensionName =
    vk::ExtensionName::from_bytes(b"VK_LAYER_KHRONOS_validation");
pub const DEVICE_EXTENSIONS: &[vk::ExtensionName] = &[
    vk::KHR_SWAPCHAIN_EXTENSION.name,
    vk::KHR_BUFFER_DEVICE_ADDRESS_EXTENSION.name,
    vk::KHR_ACCELERATION_STRUCTURE_EXTENSION.name,
    vk::KHR_RAY_QUERY_EXTENSION.name,
    vk::KHR_DEFERRED_HOST_OPERATIONS_EXTENSION.name,
];
pub const MAX_FRAMES_IN_FLIGHT: usize = 2;

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
                        crate::log!("Deleted old screenshot: {:?}", filename_str);
                    }
                }
            }
        }
    }

    if deleted_count > 0 {
        crate::log!("Cleaned up {} old screenshot(s)", deleted_count);
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
    pub unsafe fn create(window: &Window) -> Result<Self> {
        let loader = LibloadingLoader::new(LIBRARY)?;
        let entry = Entry::new(loader).map_err(|b| anyhow!("{}", b))?;
        let mut data = AppData::default();

        data.ecs_world.insert_resource(Camera::default());
        data.ecs_world
            .insert_resource(RayTracingDebugState::default());

        let (instance, messenger) = Self::create_instance_with_messenger(window, &entry)?;
        let surface = vk_window::create_surface(&instance, &window, &window)?;
        let rrdevice = RRDevice::new(
            &entry,
            &instance,
            &surface,
            VALIDATION_ENABLED,
            VALIDATION_LAYER,
            DEVICE_EXTENSIONS,
            PORTABILITY_MACOS_VERSION,
        )?;
        let rrswapchain = RRSwapchain::new(window, &instance, &surface, &rrdevice);
        let rrcommand_pool = Rc::new(RRCommandPool::new(&instance, &surface, &rrdevice));
        let rrrender = RRRender::new(&instance, &rrdevice, &rrswapchain, rrcommand_pool.as_ref());
        let swapchain_image_count = rrswapchain.swapchain_images.len();
        let max_materials = 32;
        let max_objects = 64;
        data.graphics_resources = GraphicsResources::new(
            &instance,
            &rrdevice,
            swapchain_image_count,
            max_materials,
            max_objects,
        )
        .expect("Failed to create render resources");

        let gpu_descriptors = GpuDescriptors::new(
            data.graphics_resources.frame_set.clone(),
            data.graphics_resources.objects.clone(),
        );
        let material_registry = MaterialRegistry::new(data.graphics_resources.materials.clone());
        data.ecs_world.insert_resource(gpu_descriptors);
        data.ecs_world.insert_resource(material_registry);
        data.ecs_world.insert_resource(AnimationRegistry::new());
        data.ecs_world.insert_resource(ModelState::default());
        data.ecs_world.insert_resource(MeshAssets::new());
        data.ecs_world.insert_resource(NodeAssets::new());

        let render_layouts = data.graphics_resources.get_layouts();
        let mut pipeline_manager = PipelineManager::new();

        let model_pipeline = RRPipeline::new_with_graphics_resources(
            &rrdevice,
            &rrswapchain,
            &rrrender,
            &render_layouts,
            "assets/shaders/vert.spv",
            "assets/shaders/frag.spv",
            vk::PrimitiveTopology::TRIANGLE_LIST,
            vk::PolygonMode::FILL,
            vk::CullModeFlags::BACK,
        );
        let model_pipeline_id = pipeline_manager.register(model_pipeline.clone());
        crate::log!("Registered model pipeline with id {}", model_pipeline_id);

        let grid_pipeline = RRPipeline::new_with_graphics_resources(
            &rrdevice,
            &rrswapchain,
            &rrrender,
            &render_layouts,
            "assets/shaders/gridVert.spv",
            "assets/shaders/gridFrag.spv",
            vk::PrimitiveTopology::LINE_LIST,
            vk::PolygonMode::LINE,
            vk::CullModeFlags::NONE,
        );
        let grid_pipeline_id = pipeline_manager.register(grid_pipeline);
        crate::log!("Registered grid pipeline with id {}", grid_pipeline_id);

        let gizmo_pipeline = PipelineBuilder::new(
            "assets/shaders/gizmoVert.spv",
            "assets/shaders/gizmoFrag.spv",
        )
        .vertex_input(VertexInputConfig::Gizmo)
        .topology(vk::PrimitiveTopology::LINE_LIST)
        .polygon_mode(vk::PolygonMode::LINE)
        .no_depth_test()
        .dynamic_states(vec![vk::DynamicState::LINE_WIDTH])
        .descriptor_layouts(render_layouts.to_vec())
        .build(&rrdevice, &rrrender, Some(rrswapchain.swapchain_extent))
        .expect("Failed to create gizmo pipeline");
        let gizmo_pipeline_id = pipeline_manager.register(gizmo_pipeline);
        crate::log!("Registered gizmo pipeline with id {}", gizmo_pipeline_id);

        let mut gizmo_data = create_grid_gizmo();
        gizmo_data.mesh.object_index = data.graphics_resources.objects.allocate_slot();
        gizmo_data.mesh.pipeline_id = Some(gizmo_pipeline_id);
        crate::log!(
            "Allocated object_index {} for Gizmo",
            gizmo_data.mesh.object_index
        );
        println!("allocated gizmo object_index");

        gizmo_create_buffers(
            &mut gizmo_data.mesh,
            &mut data.buffer_registry,
            &instance,
            &rrdevice,
            rrcommand_pool.as_ref(),
            true,
        )
        .expect("Failed to create gizmo buffers");

        let light_position = data
            .ecs_world
            .resource::<RayTracingDebugState>()
            .light_position;
        let mut light_gizmo_data = create_light_gizmo(light_position);
        light_gizmo_data.mesh.pipeline_id = Some(gizmo_pipeline_id);
        light_gizmo_data.mesh.object_index = data.graphics_resources.objects.allocate_slot();
        crate::log!(
            "Allocated object_index {} for LightGizmo",
            light_gizmo_data.mesh.object_index
        );
        gizmo_create_buffers(
            &mut light_gizmo_data.mesh,
            &mut data.buffer_registry,
            &instance,
            &rrdevice,
            rrcommand_pool.as_ref(),
            false,
        )
        .expect("Failed to create light gizmo buffers");

        let mut billboard_data = create_billboard();
        billboard_data.object_index = data.graphics_resources.objects.allocate_slot();
        crate::log!(
            "Allocated object_index {} for Billboard",
            billboard_data.object_index
        );

        billboard_create_buffers(
            &mut billboard_data,
            &mut data.buffer_registry,
            &instance,
            &rrdevice,
            rrcommand_pool.as_ref(),
        )
        .expect("Failed to create billboard buffers");

        billboard_data.descriptor_set = RRBillboardDescriptorSet::new(&rrdevice, &rrswapchain)
            .expect("Failed to create billboard descriptor set");
        billboard_data.descriptor_set.rrdata.push(RRData::new(
            &instance,
            &rrdevice,
            &rrswapchain,
            "billboard",
        ));

        billboard_data
            .descriptor_set
            .allocate_descriptor_sets(&rrdevice, &rrswapchain)
            .expect("Failed to allocate billboard descriptor sets");

        if let Some(ref billboard_texture) = billboard_data.texture {
            billboard_data
                .descriptor_set
                .update_descriptor_sets(&rrdevice, &rrswapchain, billboard_texture)
                .expect("Failed to update billboard descriptor sets");
        }

        let billboard_pipeline = RRPipeline::new_billboard(
            &rrdevice,
            &rrrender,
            &rrswapchain,
            billboard_data.descriptor_set.descriptor_set_layout,
            "assets/shaders/billboardVert.spv",
            "assets/shaders/billboardFrag.spv",
        )
        .expect("Failed to create billboard pipeline");
        let billboard_pipeline_id = pipeline_manager.register(billboard_pipeline);
        billboard_data.pipeline_id = Some(billboard_pipeline_id);
        crate::log!(
            "Registered billboard pipeline with id {}",
            billboard_pipeline_id
        );

        println!("created pipeline");

        data.ecs_world.insert_resource(pipeline_manager);
        data.ecs_world.insert_resource(gizmo_data);
        data.ecs_world.insert_resource(light_gizmo_data);
        data.ecs_world.insert_resource(billboard_data);

        crate::log!("Starting ray tracing initialization...");
        crate::log!(
            "swapchain extent: {}x{}",
            rrswapchain.swapchain_extent.width,
            rrswapchain.swapchain_extent.height
        );

        let mut rrrender_mut = rrrender.clone();
        match Self::init_ray_tracing_with_resources(
            &instance,
            &rrdevice,
            &mut data,
            &rrswapchain,
            rrcommand_pool.as_ref(),
            &mut rrrender_mut,
        ) {
            Ok(_) => {
                crate::log!("init_ray_tracing succeeded");
                crate::log!("gbuffer is_some: {}", data.raytracing.gbuffer.is_some());
            }
            Err(e) => {
                crate::log!("Failed to initialize ray tracing: {:?}", e);
            }
        }
        let rrrender = rrrender_mut;
        crate::log!("initialized ray tracing resources");

        let default_model_path = "assets/models/stickman/stickman.glb";
        if let Err(e) = Self::load_model_from_path_with_resources(
            &instance,
            &rrdevice,
            &mut data,
            &rrcommand_pool,
            &rrswapchain,
            default_model_path,
        ) {
            eprintln!("Failed to load model: {:?}", e);
            crate::log!("Failed to load model: {:?}", e);
        }
        crate::log!("loaded initial model: {}", default_model_path);

        if let Err(e) = Self::create_ray_tracing_pipelines_with_resources(
            &instance,
            &rrdevice,
            &mut data,
            &rrswapchain,
            &rrrender,
        ) {
            crate::log!("Failed to create ray tracing pipelines: {:?}", e);
        } else {
            crate::log!("Ray tracing pipelines created successfully");
        }

        let mut grid = GridData::default();
        grid.pipeline_id = Some(grid_pipeline_id);

        let tex_coord = Vec2::new(0.0, 0.0);
        let mut color = Vec4::new(1.0, 0.0, 0.0, 1.0);
        if let Err(e) = Self::create_grid_data(&mut grid, 0, color, tex_coord) {
            eprintln!("{:?}", e)
        }
        color = Vec4::new(0.0, 1.0, 0.0, 1.0);
        if let Err(e) = Self::create_grid_data(&mut grid, 1, color, tex_coord) {
            eprintln!("{:?}", e)
        }
        color = Vec4::new(0.0, 0.0, 1.0, 1.0);
        if let Err(e) = Self::create_grid_data(&mut grid, 2, color, tex_coord) {
            eprintln!("{:?}", e)
        }
        println!("created grid data ");
        grid.scale = 1.0;
        grid.vertex_buffer_handle = data.buffer_registry.create_vertex_buffer(
            &instance,
            &rrdevice,
            &rrcommand_pool,
            &grid.vertices,
            true,
        )?;
        println!("created grid vertex buffers");
        grid.index_buffer_handle = data.buffer_registry.create_index_buffer(
            &instance,
            &rrdevice,
            &rrcommand_pool,
            &grid.indices,
        )?;
        println!("created grid index buffer");

        grid.object_index = data.graphics_resources.objects.allocate_slot();
        crate::log!("Allocated object_index {} for Grid", grid.object_index);
        println!("allocated grid object_index");

        let mut rrcommand_buffer = RRCommandBuffer::new(&rrcommand_pool);
        if let Err(e) =
            RRCommandBuffer::allocate_command_buffers(&rrdevice, &rrrender, &mut rrcommand_buffer)
        {
            eprintln!("failed to allocate command buffers: {:?}", e);
        }
        println!("created command buffers");

        let (image_available_semaphores, render_finish_semaphores, in_flight_fences) =
            Self::create_sync_objects(&rrdevice.device)?;
        println!("created sync objects");

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
            default_model_path,
            rrdevice.msaa_samples,
        );
        println!("registered ECS resources");

        let frame = 0 as usize;
        let resized = false;
        let start = Instant::now();

        data.ecs_world.insert_resource(grid);

        println!("initialized finished");
        Ok(Self {
            entry,
            instance,
            rrdevice,
            data,
            frame,
            resized,
            start,
        })
    }

    extern "system" fn debug_callback(
        severity: vk::DebugUtilsMessageSeverityFlagsEXT,
        type_: vk::DebugUtilsMessageTypeFlagsEXT,
        data: *const vk::DebugUtilsMessengerCallbackDataEXT,
        _: *mut c_void,
    ) -> vk::Bool32 {
        let data = unsafe { *data };
        let message = unsafe { CStr::from_ptr(data.message) }.to_string_lossy();

        // コンソール（色付き）とログファイルの両方に出力
        use log::{debug, error, trace, warn};
        if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::ERROR {
            error!("({:?}) {}", type_, message);
            crate::log!("ERROR ({:?}) {}", type_, message);
        } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::WARNING {
            warn!("({:?}) {}", type_, message);
            crate::log!("WARN ({:?}) {}", type_, message);
        } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::INFO {
            debug!("({:?}) {}", type_, message);
            crate::log!("INFO ({:?}) {}", type_, message);
        } else {
            trace!("({:?}) {}", type_, message);
            crate::log!("DEBUG ({:?}) {}", type_, message);
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
            log::info!("Enabling extensions for macOS portability.");
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
        let frame_sync = FrameSync::new(
            resources.image_available_semaphores.clone(),
            resources.render_finish_semaphores.clone(),
            resources.in_flight_fences.clone(),
        );
        data.ecs_world.insert_resource(frame_sync);

        let swapchain_state = SwapchainState::new(
            resources.rrswapchain.clone(),
            resources.rrswapchain.swapchain_images.len(),
        );
        data.ecs_world.insert_resource(swapchain_state);

        let render_targets = RenderTargets::new(resources.rrrender.clone());
        data.ecs_world.insert_resource(render_targets);

        let command_state = CommandState::new(
            resources.rrcommand_pool.clone(),
            resources.rrcommand_buffer.clone(),
        );
        data.ecs_world.insert_resource(command_state);

        let pipeline_state = PipelineState::new(resources.model_pipeline.clone());
        data.ecs_world.insert_resource(pipeline_state);

        let surface_state = SurfaceState::new(resources.surface, resources.messenger);
        data.ecs_world.insert_resource(surface_state);

        let has_playback = data.ecs_world.contains_resource::<AnimationPlayback>();
        if has_playback {
            let mut playback = data.ecs_world.resource_mut::<AnimationPlayback>();
            if playback.model_path.is_empty() {
                playback.model_path = model_path.to_string();
            }
        } else {
            let animation_playback = AnimationPlayback::with_model_path(model_path.to_string());
            data.ecs_world.insert_resource(animation_playback);
        }

        if !data.ecs_world.contains_resource::<RenderConfig>() {
            let render_config = RenderConfig::new(msaa_samples);
            data.ecs_world.insert_resource(render_config);
        }

        if !data
            .ecs_world
            .contains_resource::<crate::ecs::UIEventQueue>()
        {
            let ui_event_queue = crate::ecs::UIEventQueue::new();
            data.ecs_world.insert_resource(ui_event_queue);
        }
    }

    pub unsafe fn init_imgui_rendering(
        instance: &Instance,
        rrdevice: &RRDevice,
        data: &mut AppData,
        imgui: &mut imgui::Context,
        rrcommand_pool: &RRCommandPool,
        rrrender: &RRRender,
    ) -> Result<()> {
        crate::log!("Initializing ImGui Vulkan rendering resources");

        // Get font texture data from ImGui
        let font_atlas = imgui.fonts();
        let font_texture = font_atlas.build_rgba32_texture();
        let width = font_texture.width;
        let height = font_texture.height;
        let font_data: &[u8] = &font_texture.data;

        crate::log!("Font texture size: {}x{}", width, height);

        // Create font image
        let extent = vk::Extent3D {
            width,
            height,
            depth: 1,
        };

        let image_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .extent(extent)
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        let image = rrdevice.device.create_image(&image_info, None)?;

        // Allocate image memory
        let requirements = rrdevice.device.get_image_memory_requirements(image);
        let memory_type_index = get_memory_type_index(
            instance,
            rrdevice.physical_device,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            requirements,
        )?;

        let allocate_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(requirements.size)
            .memory_type_index(memory_type_index);

        let image_memory = rrdevice.device.allocate_memory(&allocate_info, None)?;
        rrdevice.device.bind_image_memory(image, image_memory, 0)?;

        // Create staging buffer
        let buffer_size = (width * height * 4) as vk::DeviceSize;

        let buffer_info = vk::BufferCreateInfo::builder()
            .size(buffer_size)
            .usage(vk::BufferUsageFlags::TRANSFER_SRC)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let staging_buffer = rrdevice.device.create_buffer(&buffer_info, None)?;
        let buffer_requirements = rrdevice
            .device
            .get_buffer_memory_requirements(staging_buffer);

        let memory_type_index = get_memory_type_index(
            instance,
            rrdevice.physical_device,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            buffer_requirements,
        )?;

        let allocate_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(buffer_requirements.size)
            .memory_type_index(memory_type_index);

        let staging_buffer_memory = rrdevice.device.allocate_memory(&allocate_info, None)?;
        rrdevice
            .device
            .bind_buffer_memory(staging_buffer, staging_buffer_memory, 0)?;

        // Copy font data to staging buffer
        let memory_ptr = rrdevice.device.map_memory(
            staging_buffer_memory,
            0,
            buffer_size,
            vk::MemoryMapFlags::empty(),
        )?;
        memcpy(font_data.as_ptr(), memory_ptr.cast(), font_data.len());
        rrdevice.device.unmap_memory(staging_buffer_memory);

        // Transition image layout and copy from staging buffer
        Self::transition_image_layout_and_copy(
            &rrdevice.device,
            rrcommand_pool,
            &rrdevice.graphics_queue,
            image,
            staging_buffer,
            width,
            height,
        )?;

        // Cleanup staging buffer
        rrdevice.device.destroy_buffer(staging_buffer, None);
        rrdevice.device.free_memory(staging_buffer_memory, None);

        // Create image view
        let view_info = vk::ImageViewCreateInfo::builder()
            .image(image)
            .view_type(vk::ImageViewType::_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        let image_view = rrdevice.device.create_image_view(&view_info, None)?;

        // Create sampler
        let sampler_info = vk::SamplerCreateInfo::builder()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .min_lod(0.0)
            .max_lod(1.0);

        let sampler = rrdevice.device.create_sampler(&sampler_info, None)?;

        // Create descriptor pool for ImGui
        let pool_sizes = [vk::DescriptorPoolSize::builder()
            .type_(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)];

        let pool_info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(&pool_sizes)
            .max_sets(1);

        let descriptor_pool = rrdevice.device.create_descriptor_pool(&pool_info, None)?;

        // Create descriptor set layout
        let bindings = [vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)];

        let layout_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings);

        let descriptor_set_layout = rrdevice
            .device
            .create_descriptor_set_layout(&layout_info, None)?;

        // Allocate descriptor set
        let layouts = [descriptor_set_layout];
        let allocate_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&layouts);

        let descriptor_sets = rrdevice.device.allocate_descriptor_sets(&allocate_info)?;
        let descriptor_set = descriptor_sets[0];

        // Update descriptor set with font texture
        let image_info = [vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(image_view)
            .sampler(sampler)];

        let descriptor_writes = [vk::WriteDescriptorSet::builder()
            .dst_set(descriptor_set)
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(&image_info)];

        rrdevice
            .device
            .update_descriptor_sets(&descriptor_writes, &[] as &[vk::CopyDescriptorSet]);

        // Create ImGui pipeline using RRPipeline
        let msaa_samples = {
            let render_config = data.ecs_world.resource::<RenderConfig>();
            if !render_config.msaa_samples.is_empty() {
                render_config.msaa_samples
            } else {
                vk::SampleCountFlags::_8
            }
        };

        let imgui_pipeline = RRPipeline::new_imgui(
            rrdevice,
            rrrender,
            descriptor_set_layout,
            "assets/shaders/imguiVert.spv",
            "assets/shaders/imguiFrag.spv",
            msaa_samples,
        )?;

        data.imgui.pipeline = Some(imgui_pipeline.pipeline);
        data.imgui.pipeline_layout = Some(imgui_pipeline.pipeline_layout);
        data.imgui.descriptor_set = Some(descriptor_set);
        data.imgui.descriptor_set_layout = Some(descriptor_set_layout);
        data.imgui.descriptor_pool = Some(descriptor_pool);
        data.imgui.font_image = Some(image);
        data.imgui.font_image_memory = Some(image_memory);
        data.imgui.font_image_view = Some(image_view);
        data.imgui.sampler = Some(sampler);

        crate::log!("ImGui rendering resources initialized successfully");
        crate::log!("  Pipeline: {:?}", imgui_pipeline.pipeline);
        crate::log!("  Descriptor Set: {:?}", descriptor_set);

        Ok(())
    }
}
