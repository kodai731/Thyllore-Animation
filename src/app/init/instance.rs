use crate::app::{App, AppData};
use crate::app::data::GUIData;

use rust_rendering::vulkanr::buffer::*;
use rust_rendering::vulkanr::command::*;
use rust_rendering::vulkanr::data as vulkan_data;
use rust_rendering::vulkanr::data::*;
use rust_rendering::vulkanr::descriptor::*;
use rust_rendering::vulkanr::device::*;
use rust_rendering::vulkanr::image::*;
use rust_rendering::vulkanr::pipeline::{
    PipelineBuilder, RRPipeline, VertexInputConfig, DepthTestConfig, BlendConfig, PushConstantConfig,
};
use rust_rendering::vulkanr::render::*;
use rust_rendering::vulkanr::swapchain::*;
use rust_rendering::vulkanr::vulkan::*;
use rust_rendering::vulkanr::raytracing::acceleration::*;

use rust_rendering::loader::gltf::gltf::*;
use rust_rendering::math::*;
use rust_rendering::debugview::*;
use rust_rendering::loader::fbx::fbx::{FbxModel, load_fbx, load_fbx_with_russimp};
use rust_rendering::logger::logger::*;

use vulkanalia::Device as VkDevice;

use anyhow::{anyhow, Result};
use std::collections::HashSet;
use std::ffi::CStr;
use std::os::raw::c_void;
use std::collections::HashMap;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::BufReader;
use std::mem::size_of;
use std::ptr::copy_nonoverlapping as memcpy;
use std::time::Instant;
use std::borrow::BorrowMut;
use std::rc::Rc;

use winit::window::Window;
use cgmath::num_traits::AsPrimitive;
use cgmath::{Matrix4, Vector4};
use vulkanalia::prelude::v1_0::*;
use vulkanalia::bytecode::Bytecode;
use vulkanalia::vk::KhrSwapchainExtension;
use vulkanalia::loader::{LibloadingLoader, LIBRARY};
use vulkanalia::vk::KhrSurfaceExtension;

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

// Initialization methods for App
impl App {
    pub unsafe fn create(window: &Window) -> Result<Self> {
        let loader = LibloadingLoader::new(LIBRARY)?;
        let entry = Entry::new(loader).map_err(|b| anyhow!("{}", b))?;
        let mut data = AppData::default();
        let instance = Self::create_instance(window, &entry, &mut data)?;
        data.surface = vk_window::create_surface(&instance, &window, &window)?;
        let rrdevice = RRDevice::new(
            &entry,
            &instance,
            &data.surface,
            VALIDATION_ENABLED,
            VALIDATION_LAYER,
            DEVICE_EXTENSIONS,
            PORTABILITY_MACOS_VERSION,
        )?;
        data.rrswapchain = RRSwapchain::new(window, &instance, &data.surface, &rrdevice);
        data.rrcommand_pool = Rc::new(RRCommandPool::new(&instance, &data.surface, &rrdevice));
        data.rrrender = RRRender::new(
            &instance,
            &rrdevice,
            &data.rrswapchain,
            &data.rrcommand_pool.borrow_mut(),
        );
        data.model_descriptor_set = RRDescriptorSet::new(&rrdevice, &data.rrswapchain);
        data.grid_descriptor_set = RRDescriptorSet::new(&rrdevice, &data.rrswapchain);
        data.model_pipeline = RRPipeline::new(
            &rrdevice,
            &data.rrswapchain,
            &data.rrrender,
            &data.model_descriptor_set,
            "assets/shaders/vert.spv",
            "assets/shaders/frag.spv",
            vk::PrimitiveTopology::TRIANGLE_LIST,
            vk::PolygonMode::FILL,
        );
        data.grid_pipeline = RRPipeline::new(
            &rrdevice,
            &data.rrswapchain,
            &data.rrrender,
            &data.grid_descriptor_set,
            "assets/shaders/gridVert.spv",
            "assets/shaders/gridFrag.spv",
            vk::PrimitiveTopology::LINE_LIST,
            vk::PolygonMode::LINE,
        );

        // Gizmo用のディスクリプタセットとパイプラインを作成
        data.gizmo_descriptor_set = RRDescriptorSet::new(&rrdevice, &data.rrswapchain);

        // Grid Gizmo用のuniform bufferを作成
        data.gizmo_descriptor_set
            .rrdata
            .push(RRData::new(&instance, &rrdevice, &data.rrswapchain));

        // Light Gizmo用のuniform bufferを作成
        data.gizmo_descriptor_set
            .rrdata
            .push(RRData::new(&instance, &rrdevice, &data.rrswapchain));

        if let Err(e) = RRDescriptorSet::create_descriptor_set(
            &rrdevice,
            &data.rrswapchain,
            &mut data.gizmo_descriptor_set,
        ) {
            eprintln!("failed to create gizmo descriptor set: {:?}", e);
        }
        println!("created gizmo descriptor set");

        data.gizmo_pipeline = PipelineBuilder::new("assets/shaders/gizmoVert.spv", "assets/shaders/gizmoFrag.spv")
            .vertex_input(VertexInputConfig::Custom {
                bindings: vec![GizmoVertex::binding_description()],
                attributes: GizmoVertex::attribute_descriptions().to_vec(),
            })
            .topology(vk::PrimitiveTopology::LINE_LIST)
            .polygon_mode(vk::PolygonMode::LINE)
            .no_depth_test()  // Gizmoは常に手前に表示
            .dynamic_states(vec![vk::DynamicState::LINE_WIDTH])
            .descriptor_layouts(vec![data.gizmo_descriptor_set.descriptor_set_layout])
            .build(&rrdevice, &data.rrrender, Some(data.rrswapchain.swapchain_extent))
            .expect("Failed to create gizmo pipeline");

        // Gizmoデータを初期化
        data.gizmo_data = GridGizmoData::new();
        data.gizmo_data.create_buffers(&instance, &rrdevice, &data.rrcommand_pool)
            .expect("Failed to create gizmo buffers");

        // ライトGizmoデータを初期化
        data.light_gizmo_data = LightGizmoData::new(data.rt_debug_state.light_position);
        data.light_gizmo_data.create_buffers(&instance, &rrdevice, &data.rrcommand_pool)
            .expect("Failed to create light gizmo buffers");
        data.light_gizmo_selected = false;
        data.light_drag_axis = LightGizmoAxis::None;

        // ビルボード用のテクスチャを先にロード
        data.light_gizmo_data.create_billboard_buffers(&instance, &rrdevice, &data.rrcommand_pool)
            .expect("Failed to create billboard buffers");

        // ビルボード用のディスクリプタセットとパイプラインを作成
        data.billboard_descriptor_set = RRDescriptorSet::new(&rrdevice, &data.rrswapchain);
        data.billboard_descriptor_set
            .rrdata
            .push(RRData::new(&instance, &rrdevice, &data.rrswapchain));

        if let Err(e) = create_billboard_descriptor_set(
            &instance,
            &rrdevice,
            &data.rrswapchain,
            &mut data.billboard_descriptor_set,
            &data.light_gizmo_data,
        ) {
            eprintln!("failed to create billboard descriptor set: {:?}", e);
        }

        data.billboard_pipeline = RRPipeline::new_billboard(
            &rrdevice,
            &data.rrrender,
            &data.rrswapchain,
            data.billboard_descriptor_set.descriptor_set_layout,
            "assets/shaders/billboardVert.spv",
            "assets/shaders/billboardFrag.spv",
            rust_rendering::debugview::gizmo::BillboardVertex::binding_description(),
            rust_rendering::debugview::gizmo::BillboardVertex::attribute_descriptions().to_vec(),
        )
        .expect("Failed to create billboard pipeline");

        println!("created pipeline");

        if let Err(e) = Self::reload_model_data_buffer(&instance, &rrdevice, &mut data) {
            eprintln!("{:?}", e)
        }
        println!("reloaded model");

        let tex_coord = Vec2::new(0.0, 0.0);
        let mut color = Vec4::new(1.0, 0.0, 0.0, 1.0);
        if let Err(e) = Self::create_grid_data(&mut data, 0, color, tex_coord) {
            eprintln!("{:?}", e)
        }
        color = Vec4::new(0.0, 0.0, 1.0, 1.0);
        if let Err(e) = Self::create_grid_data(&mut data, 2, color, tex_coord) {
            eprintln!("{:?}", e)
        }
        println!("created grid data ");
        data.grid_scale = 1.0;
        data.near_plane = 0.1;
        data.far_plane = 1000.0;
        // let _ = Self::create_texture_image(&instance, &device, &mut data)?;
        // data.texture_image = RRImage::new(&instance, &rrdevice, &data.rrcommand_pool.borrow_mut());
        data.grid_vertex_buffer = RRVertexBuffer::new(
            &instance,
            &rrdevice,
            &data.rrcommand_pool,
            (size_of::<vulkan_data::Vertex>() * data.grid_vertices.len()) as vk::DeviceSize,
            data.grid_vertices.as_ptr() as *const c_void,
            data.grid_vertices.len(),
        );
        println!("created grid vertex buffers");
        data.grid_index_buffer = RRIndexBuffer::new(
            &instance,
            &rrdevice,
            &data.rrcommand_pool,
            (size_of::<u32>() * data.grid_indices.len()) as u64,
            data.grid_indices.as_ptr() as *const c_void,
            data.grid_indices.len(),
        );
        println!("created grid index buffer");

        data.grid_descriptor_set
            .rrdata
            .push(RRData::new(&instance, &rrdevice, &data.rrswapchain));
        println!("created grid uniform buffers");

        // Light Ray用のuniform buffer（model = 単位行列）
        data.grid_descriptor_set
            .rrdata
            .push(RRData::new(&instance, &rrdevice, &data.rrswapchain));
        println!("created light ray uniform buffers");

        // let grid_rrdata = &mut data.grid_descriptor_set.rrdata[0];
        // grid_rrdata.image_view = create_image_view(
        //     &rrdevice,
        //     data.texture_images[0],
        //     vk::Format::R8G8B8A8_SRGB,
        //     vk::ImageAspectFlags::COLOR,
        //     data.mip_levels[0],
        // )?;
        // data.grid_descriptor_set.rrdata.sampler =
        //     create_texture_sampler(&rrdevice, data.mip_levels[0])?;

        if let Err(e) = RRDescriptorSet::create_descriptor_set(
            &rrdevice,
            &data.rrswapchain,
            &mut data.model_descriptor_set,
        ) {
            eprintln!("failed to create model descriptor set: {:?}", e);
        };
        println!("created model descriptor set");
        if let Err(e) = RRDescriptorSet::create_descriptor_set(
            &rrdevice,
            &data.rrswapchain,
            &mut data.grid_descriptor_set,
        ) {
            eprintln!("failed to create grid descriptor set: {:?}", e);
        }
        println!("created grid descriptor set");
        let offset_vertex = (data.grid_vertices.len()) as u32;
        let offset_index = (data.grid_indices.len()) as u32;
        data.rrcommand_buffer = RRCommandBuffer::new(&data.rrcommand_pool);

        if let Err(e) = RRCommandBuffer::allocate_command_buffers(
            &rrdevice,
            &data.rrrender,
            &mut data.rrcommand_buffer,
        ) {
            eprintln!("failed to allocate command buffers: {:?}", e);
        }
        let mut rrbind_info = Vec::new();
        rrbind_info.push(RRBindInfo::new(
            &data.grid_pipeline,
            &data.grid_descriptor_set,
            &data.grid_vertex_buffer,
            &data.grid_index_buffer,
            0,
            0,
            0,  // data_index for grid (always 0)
        ));

        for i in 0..data.model_descriptor_set.rrdata.len() {
            rrbind_info.push(RRBindInfo::new(
                &data.model_pipeline,
                &data.model_descriptor_set,
                &data.model_descriptor_set.rrdata[i].vertex_buffer,
                &data.model_descriptor_set.rrdata[i].index_buffer,
                0,
                0,
                i,  // data_index corresponds to rrdata index
            ));
        }

        for i in 0..data.rrrender.framebuffers.len() {
            for j in 0..rrbind_info.len() {
                if let Err(e) = RRCommandBuffer::bind_command(
                    &rrdevice,
                    &data.rrrender,
                    &data.rrswapchain,
                    &rrbind_info,
                    &mut data.rrcommand_buffer,
                    i,
                ) {
                    eprintln!("failed to create command buffers: {:?}", e);
                }
            }
        }

        println!("created command buffer");

        let _ = Self::create_sync_objects(&rrdevice.device, &mut data)?;
        println!("created sync objects");

        // Initialize Ray Tracing (G-Buffer, etc.)
        // Note: Acceleration structures will be built after model is loaded
        if let Err(e) = Self::init_ray_tracing(&instance, &rrdevice, &mut data) {
            eprintln!("Failed to initialize ray tracing: {:?}", e);
        }
        println!("initialized ray tracing resources");

        let frame = 0 as usize;
        let resized = false;
        let start = Instant::now();

        data.initial_camera_pos = [5.0, 3.0, -5.0];
        data.camera_pos = data.initial_camera_pos;
        let camera_pos = vec3(data.camera_pos[0], data.camera_pos[1], data.camera_pos[2]);
        let camera_direction = (vec3(0.0, 0.0, 0.0) - camera_pos).normalize();
        let camera_up = vec3(0.0, 1.0, 0.0);
        data.camera_direction = [camera_direction.x, camera_direction.y, camera_direction.z];
        data.camera_up = [camera_up.x, camera_up.y, camera_up.z];
        data.is_left_clicked = false;

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

    unsafe fn create_instance(
        window: &Window,
        entry: &Entry,
        data: &mut AppData,
    ) -> Result<Instance> {
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

        // for Mac ablability
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

        if VALIDATION_ENABLED {
            data.messenger = instance.create_debug_utils_messenger_ext(&debug_info, None)?;
        }

        Ok(instance)
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
        use log::{error, warn, debug, trace};
        if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::ERROR {
            error!("({:?}) {}", type_, message);
            log!("ERROR ({:?}) {}", type_, message);
        } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::WARNING {
            warn!("({:?}) {}", type_, message);
            log!("WARN ({:?}) {}", type_, message);
        } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::INFO {
            debug!("({:?}) {}", type_, message);
            log!("INFO ({:?}) {}", type_, message);
        } else {
            trace!("({:?}) {}", type_, message);
            log!("DEBUG ({:?}) {}", type_, message);
        }

        vk::FALSE
    }

    unsafe fn create_sync_objects(device: &VkDevice, data: &mut AppData) -> Result<()> {
        let semaphore_info = vk::SemaphoreCreateInfo::builder();
        let fence_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);

        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            data.image_available_semaphores
                .push(device.create_semaphore(&semaphore_info, None)?);
            data.render_finish_semaphores
                .push(device.create_semaphore(&semaphore_info, None)?);
            data.in_flight_fences
                .push(device.create_fence(&fence_info, None)?);
        }

        data.images_in_flight = data
            .rrswapchain
            .swapchain_images
            .iter()
            .map(|_| vk::Fence::null())
            .collect();

        Ok(())
    }
    pub unsafe fn init_imgui_rendering(
        instance: &Instance,
        rrdevice: &RRDevice,
        data: &mut AppData,
        imgui: &mut imgui::Context,
    ) -> Result<()> {
        log!("Initializing ImGui Vulkan rendering resources");

        // Get font texture data from ImGui
        let font_atlas = imgui.fonts();
        let font_texture = font_atlas.build_rgba32_texture();
        let width = font_texture.width;
        let height = font_texture.height;
        let font_data: &[u8] = &font_texture.data;

        log!("Font texture size: {}x{}", width, height);

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
        let buffer_requirements = rrdevice.device.get_buffer_memory_requirements(staging_buffer);

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
        rrdevice.device.bind_buffer_memory(staging_buffer, staging_buffer_memory, 0)?;

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
            &data.rrcommand_pool,
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

        let descriptor_set_layout = rrdevice.device.create_descriptor_set_layout(&layout_info, None)?;

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

        rrdevice.device.update_descriptor_sets(&descriptor_writes, &[] as &[vk::CopyDescriptorSet]);

        // Create ImGui pipeline using RRPipeline
        let msaa_samples = if !data.msaa_samples.is_empty() {
            data.msaa_samples
        } else {
            vk::SampleCountFlags::_8
        };

        let imgui_pipeline = RRPipeline::new_imgui(
            rrdevice,
            &data.rrrender,
            descriptor_set_layout,
            "assets/shaders/imguiVert.spv",
            "assets/shaders/imguiFrag.spv",
            msaa_samples,
        )?;

        // Store in AppData
        data.imgui_pipeline = Some(imgui_pipeline.pipeline);
        data.imgui_pipeline_layout = Some(imgui_pipeline.pipeline_layout);
        data.imgui_descriptor_set = Some(descriptor_set);
        data.imgui_descriptor_set_layout = Some(descriptor_set_layout);
        data.imgui_descriptor_pool = Some(descriptor_pool);
        data.imgui_font_image = Some(image);
        data.imgui_font_image_memory = Some(image_memory);
        data.imgui_font_image_view = Some(image_view);
        data.imgui_sampler = Some(sampler);

        log!("ImGui rendering resources initialized successfully");
        log!("  Pipeline: {:?}", imgui_pipeline.pipeline);
        log!("  Descriptor Set: {:?}", descriptor_set);

        Ok(())
    }
}

unsafe fn create_billboard_descriptor_set(
    _instance: &Instance,
    rrdevice: &RRDevice,
    rrswapchain: &RRSwapchain,
    billboard_descriptor_set: &mut RRDescriptorSet,
    light_gizmo_data: &rust_rendering::debugview::gizmo::LightGizmoData,
) -> Result<(), Box<dyn std::error::Error>> {
    use rust_rendering::vulkanr::descriptor::*;
    use rust_rendering::logger::logger::*;

    RRDescriptorSet::create_descriptor_set(rrdevice, rrswapchain, billboard_descriptor_set)?;

    log!("billboard_texture is_some: {}", light_gizmo_data.billboard_texture.is_some());

    if let Some(ref billboard_texture) = light_gizmo_data.billboard_texture {
        let swapchain_images_len = rrswapchain.swapchain_images.len();
        log!("Updating billboard descriptor sets: rrdata.len={}, swapchain_images_len={}",
             billboard_descriptor_set.rrdata.len(), swapchain_images_len);

        for i in 0..billboard_descriptor_set.rrdata.len() {
            for j in 0..swapchain_images_len {
                let descriptor_set_index = i * swapchain_images_len + j;
                log!("Updating descriptor set index {} (i={}, j={})", descriptor_set_index, i, j);

                let descriptor_image_info = vk::DescriptorImageInfo::builder()
                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .image_view(billboard_texture.image_view)
                    .sampler(billboard_texture.sampler);

                let descriptor_image_infos = &[descriptor_image_info];

                let sampler_descriptor_write = vk::WriteDescriptorSet::builder()
                    .dst_set(billboard_descriptor_set.descriptor_sets[descriptor_set_index])
                    .dst_binding(1)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(descriptor_image_infos);

                rrdevice.device.update_descriptor_sets(
                    &[sampler_descriptor_write.build()],
                    &[] as &[vk::CopyDescriptorSet],
                );
            }
        }
    } else {
        log!("WARNING: billboard_texture is None!");
    }

    Ok(())
}
