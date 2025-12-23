#![allow(
    dead_code,
    unused_variables,
    clippy::too_many_arguments,
    clippy::unnecessary_wraps
)]

// lib.rsからモジュールをインポート
use rust_rendering::vulkanr::buffer::*;
use rust_rendering::vulkanr::command::*;
use rust_rendering::vulkanr::data::{self, *};
use rust_rendering::vulkanr::descriptor::*;
use rust_rendering::vulkanr::device::*;
use rust_rendering::vulkanr::image::*;
use rust_rendering::vulkanr::pipeline::{
    PipelineBuilder, RRPipeline, VertexInputConfig, DepthTestConfig, BlendConfig, PushConstantConfig,
};
use rust_rendering::vulkanr::render::*;
use rust_rendering::vulkanr::swapchain::*;
use rust_rendering::vulkanr::vulkan::*;
use rust_rendering::vulkanr::window::*;
use rust_rendering::vulkanr::acceleration_structure::*;

use rust_rendering::gltf::gltf::*;
use rust_rendering::math::math::*;
use rust_rendering::gizmo::gizmo::*;

// Disambiguate Device type - use vulkanalia's Device explicitly where needed
use vulkanalia::Device as VkDevice;

// imgui
//use imgui::*;

// マクロをインポート
#[macro_use]
extern crate rust_rendering;

mod support;

use rust_rendering::logger::logger::*;
use rust_rendering::fbx::fbx::{FbxModel, load_fbx, load_fbx_with_russimp};

use anyhow::{anyhow, Result};
use core::result::Result::Ok;
const PORTABILITY_MACOS_VERSION: Version = Version::new(1, 3, 216);
use std::collections::HashSet;
use std::ffi::CStr;
use std::os::raw::c_void;
const VALIDATION_ENABLED: bool = cfg!(debug_assertions);
const VALIDATION_LAYER: vk::ExtensionName =
    vk::ExtensionName::from_bytes(b"VK_LAYER_KHRONOS_validation");
const DEVICE_EXTENSIONS: &[vk::ExtensionName] = &[
    vk::KHR_SWAPCHAIN_EXTENSION.name,
    vk::KHR_BUFFER_DEVICE_ADDRESS_EXTENSION.name,
    vk::KHR_ACCELERATION_STRUCTURE_EXTENSION.name,
    vk::KHR_RAY_QUERY_EXTENSION.name,
    vk::KHR_DEFERRED_HOST_OPERATIONS_EXTENSION.name,
];
use thiserror::Error;
use vulkanalia::bytecode::Bytecode;
const MAX_FRAMES_IN_FLIGHT: usize = 2; // how many frames should be processed concurrently GPU-GPU synchronization
use std::collections::HashMap;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::BufReader;
use std::mem::size_of;
use std::ptr::copy_nonoverlapping as memcpy;
use std::time::Instant;

use winit;
use winit::event::ElementState;

use cgmath::num_traits::AsPrimitive;
use cgmath::{Matrix4, Vector4};
use imgui::{Condition, MouseButton};
use serde::Serialize;
use std::borrow::BorrowMut;
use std::path::Path;
use std::rc::Rc;
use vulkanalia::vk::CommandPool;

/// Clean up old screenshot files from the log directory
fn cleanup_old_screenshots() -> Result<()> {
    use std::fs;
    use std::path::PathBuf;

    let log_dir = PathBuf::from("log");

    // Check if log directory exists
    if !log_dir.exists() {
        return Ok(());
    }

    // Read directory entries
    let entries = fs::read_dir(&log_dir)?;

    let mut deleted_count = 0;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        // Only process files (not directories)
        if path.is_file() {
            // Check if filename starts with "screenshot_"
            if let Some(filename) = path.file_name() {
                if let Some(filename_str) = filename.to_str() {
                    if filename_str.starts_with("screenshot_") {
                        // Delete the file
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

fn main() -> Result<()> {
    pretty_env_logger::init();

    // Clean up old screenshots from previous runs
    cleanup_old_screenshots()?;

    // imgui
    let mut system = support::init(file!());
    let mut gui_data = GUIData::default();

    // App
    let mut app = unsafe { App::create(&system.window)? };

    // Initialize ImGui rendering resources
    unsafe {
        App::init_imgui_rendering(
            &app.instance,
            &app.rrdevice,
            &mut app.data,
            &mut system.imgui,
        )?;
    }

    system.main_loop(&mut app, &mut gui_data);

    Ok(())
}

impl support::System {
    pub fn main_loop(
        self,
        app: &mut App,
        gui_data: &mut GUIData,
    ) {
        let support::System {
            event_loop,
            window,
            mut imgui,
            mut platform,
        } = self;
        let mut last_frame = Instant::now();

        event_loop
            .run(move |event, window_target| {
                match event {
                    Event::NewEvents(_) => {
                        let now = Instant::now();
                        imgui.io_mut().update_delta_time(now - last_frame);
                        last_frame = now;
                    }

                    Event::AboutToWait => {
                        platform
                            .prepare_frame(imgui.io_mut(), &window)
                            .expect("Failed to prepare frame");
                        window.request_redraw();
                    }

                    Event::WindowEvent {
                        event: ref window_event,
                        window_id,
                        ..
                    } => {
                        platform.handle_event(imgui.io_mut(), &window, &event);

                        match window_event {
                            WindowEvent::CursorMoved { position, .. } => {
                                gui_data.mouse_pos = [position.x as f32, position.y as f32];
                            }

                            WindowEvent::MouseInput { state, button, .. } => {
                                if *state == ElementState::Pressed
                                    && *button == winit::event::MouseButton::Left
                                {
                                    gui_data.is_left_clicked = true;
                                }
                            }

                            WindowEvent::MouseWheel { delta, .. } => match delta {
                                winit::event::MouseScrollDelta::LineDelta(x, y) => {
                                    gui_data.mouse_wheel = *y;
                                }
                                winit::event::MouseScrollDelta::PixelDelta(pos) => {
                                    gui_data.mouse_wheel = pos.y as f32;
                                }
                            },

                            WindowEvent::Resized(new_size) => {
                                if new_size.width > 0 && new_size.height > 0 {
                                    app.resized = true;
                                }
                            }

                            WindowEvent::CloseRequested => window_target.exit(),

                            WindowEvent::DroppedFile(path_buf) => {
                                if let Some(path) = path_buf.to_str() {
                                    gui_data.file_path = path.to_string();
                                }
                            }

                            WindowEvent::RedrawRequested => {
                                let ui = imgui.frame();

                                // Create main dockspace over the entire viewport
                                ui.dockspace_over_main_viewport();

                                // initialize gui_data
                                gui_data.is_left_clicked = false;
                                gui_data.is_wheel_clicked = false;
                                gui_data.monitor_value = 0.0;

                                if ui.is_mouse_down(MouseButton::Left) {
                                    gui_data.is_left_clicked = true;
                                }
                                if ui.is_mouse_down(MouseButton::Middle) {
                                    gui_data.is_wheel_clicked = true;
                                }

                                ui.window("debug window")
                                    .size([600.0, 220.0], Condition::FirstUseEver)
                                    .build(|| {
                                        // Model Loading Section
                                        ui.text("Model Loading:");
                                        if ui.button("Open FBX Model") {
                                            if let Some(path) = rfd::FileDialog::new()
                                                .add_filter("FBX Files", &["fbx"])
                                                .pick_file()
                                            {
                                                gui_data.selected_model_path = path.to_string_lossy().to_string();
                                                gui_data.file_changed = true;
                                                log!("Selected FBX file: {}", gui_data.selected_model_path);
                                            }
                                        }
                                        ui.same_line();
                                        if ui.button("Open glTF Model") {
                                            if let Some(path) = rfd::FileDialog::new()
                                                .add_filter("glTF Files", &["gltf", "glb"])
                                                .pick_file()
                                            {
                                                gui_data.selected_model_path = path.to_string_lossy().to_string();
                                                gui_data.file_changed = true;
                                                log!("Selected glTF file: {}", gui_data.selected_model_path);
                                            }
                                        }

                                        // Current model display
                                        ui.text(format!("Current Model: {}",
                                            if app.data.current_model_path.is_empty() {
                                                "None"
                                            } else {
                                                &app.data.current_model_path
                                            }
                                        ));

                                        // Load status display
                                        ui.text(format!("Status: {}", gui_data.load_status));

                                        ui.separator();

                                        // Camera Controls
                                        ui.text("Camera Controls:");
                                        if ui.button("reset camera") {
                                            unsafe {
                                                app.reset_camera();
                                            }
                                        }
                                        ui.same_line();
                                        if ui.button("reset camera up") {
                                            unsafe {
                                                app.reset_camera_up();
                                            }
                                        }
                                        ui.separator();

                                        // Screenshot
                                        ui.text("Screenshot:");
                                        if ui.button("Take Screenshot") {
                                            gui_data.take_screenshot = true;
                                        }
                                        ui.separator();

                                        // Debug Information
                                        ui.text("Debug Info:");
                                        ui.text(format!(
                                            "Mouse Position: ({:.1},{:.1})",
                                            gui_data.mouse_pos[0], gui_data.mouse_pos[1]
                                        ));
                                        ui.text(format!(
                                            "is left clicked: ({:.1})",
                                            gui_data.is_left_clicked
                                        ));
                                        ui.text(format!(
                                            "is wheel clicked: ({:.1})",
                                            gui_data.is_wheel_clicked
                                        ));
                                        ui.input_text("file path", &mut gui_data.file_path)
                                            .read_only(true)
                                            .build();
                                    });

                                platform.prepare_render(ui, &window);
                                let draw_data = imgui.render();

                                unsafe { app.render(&window, gui_data, draw_data) }.unwrap();

                                // TODO: summarize the data
                                // clear value
                                gui_data.mouse_wheel = 0.0;
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            })
            .expect("EventLoop error");
    }
}

#[derive(Clone, Debug, Serialize)]
struct GUIData {
    is_left_clicked: bool,
    is_wheel_clicked: bool,
    monitor_value: f32,
    mouse_pos: [f32; 2],
    mouse_wheel: f32,
    file_path: String,
    // New fields for file loading
    file_changed: bool,           // Flag indicating a new file was selected
    selected_model_path: String,  // Path of the selected model file
    load_status: String,          // Status message for loading (success/error)
    take_screenshot: bool,        // Flag to trigger screenshot capture
}

impl Default for GUIData {
    fn default() -> Self {
        Self {
            is_left_clicked: false,
            is_wheel_clicked: false,
            monitor_value: 0.0,
            mouse_pos: [0.0, 0.0],
            mouse_wheel: 0.0,
            file_path: String::default(),
            file_changed: false,
            selected_model_path: String::default(),
            load_status: String::from("No model loaded"),
            take_screenshot: false,
        }
    }
}

/// Vulkan app
#[derive(Clone, Debug)]
struct App {
    entry: Entry,
    instance: Instance,
    rrdevice: RRDevice,
    data: AppData,
    frame: usize,
    resized: bool,
    start: Instant,
}

#[derive(Clone, Debug, Default)]
struct AppData {
    messenger: vk::DebugUtilsMessengerEXT,
    surface: vk::SurfaceKHR,
    rrswapchain: RRSwapchain,
    rrrender: RRRender,
    rrcommand_pool: Rc<RRCommandPool>,
    rrcommand_buffer: RRCommandBuffer,
    model_pipeline: RRPipeline,
    model_descriptor_set: RRDescriptorSet,
    grid_pipeline: RRPipeline,
    grid_descriptor_set: RRDescriptorSet,
    grid_vertex_buffer: RRVertexBuffer,
    grid_index_buffer: RRIndexBuffer,
    gizmo_pipeline: RRPipeline,
    gizmo_descriptor_set: RRDescriptorSet,
    gizmo_data: GizmoData,
    command_pool: vk::CommandPool,
    image_available_semaphores: Vec<vk::Semaphore>, // semaphores are used to synchronize operations within or across command queues.
    render_finish_semaphores: Vec<vk::Semaphore>,
    in_flight_fences: Vec<vk::Fence>, // CPU-GPU sync. Fences are mainly designed to synchronize your application itself with rendering operation
    images_in_flight: Vec<vk::Fence>,
    msaa_samples: vk::SampleCountFlags,
    color_image: vk::Image, // We only need one render target since only one drawing operation is active at a time
    color_image_memory: vk::DeviceMemory,
    color_image_view: vk::ImageView,
    camera_direction: [f32; 3],
    camera_pos: [f32; 3],
    initial_camera_pos: [f32; 3],
    camera_up: [f32; 3],
    grid_vertices: Vec<data::Vertex>,
    grid_indices: Vec<u32>,
    is_left_clicked: bool,
    clicked_mouse_pos: [f32; 2],
    is_wheel_clicked: bool,
    gltf_model: GltfModel,
    fbx_model: FbxModel,
    animation_time: f32,           // 現在のアニメーション時間（秒）
    animation_playing: bool,       // アニメーション再生中フラグ
    current_animation_index: usize, // 現在再生中のアニメーションインデックス
    current_model_path: String,    // 現在読み込まれているモデルファイルのパス
    // ImGui rendering
    imgui_pipeline: Option<vk::Pipeline>,
    imgui_pipeline_layout: Option<vk::PipelineLayout>,
    imgui_descriptor_set: Option<vk::DescriptorSet>,
    imgui_descriptor_set_layout: Option<vk::DescriptorSetLayout>,
    imgui_descriptor_pool: Option<vk::DescriptorPool>,
    imgui_font_image: Option<vk::Image>,
    imgui_font_image_memory: Option<vk::DeviceMemory>,
    imgui_font_image_view: Option<vk::ImageView>,
    imgui_sampler: Option<vk::Sampler>,
    imgui_vertex_buffer: Option<vk::Buffer>,
    imgui_vertex_buffer_memory: Option<vk::DeviceMemory>,
    imgui_vertex_buffer_size: vk::DeviceSize,
    imgui_index_buffer: Option<vk::Buffer>,
    imgui_index_buffer_memory: Option<vk::DeviceMemory>,
    imgui_index_buffer_size: vk::DeviceSize,

    // Ray Tracing / Deferred Rendering components
    gbuffer: Option<RRGBuffer>,
    acceleration_structure: Option<RRAccelerationStructure>,
    ray_query_pipeline: Option<RRPipeline>,
    ray_query_descriptor: Option<RRRayQueryDescriptorSet>,
    scene_uniform_buffer: Option<vk::Buffer>,
    scene_uniform_buffer_memory: Option<vk::DeviceMemory>,
    composite_pipeline: Option<RRPipeline>,
    composite_descriptor: Option<RRCompositeDescriptorSet>,
    gbuffer_pipeline: Option<RRPipeline>,
    gbuffer_descriptor_set: Option<RRDescriptorSet>,
    gbuffer_sampler: Option<vk::Sampler>, // Sampler for G-Buffer images
}

// Helper function to load PNG images
fn load_png_image(path: &str) -> Result<(Vec<u8>, u32, u32)> {
    use std::fs::File;
    use png;

    let image_file = File::open(path)?;
    let decoder = png::Decoder::new(image_file);
    let mut reader = decoder.read_info()?;
    let mut pixels = vec![0; reader.info().raw_bytes()];
    reader.next_frame(&mut pixels)?;
    let (width, height) = reader.info().size();

    Ok((pixels, width, height))
}

impl App {
    unsafe fn create(window: &Window) -> Result<Self> {
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
            "src/shaders/vert.spv",
            "src/shaders/frag.spv",
            vk::PrimitiveTopology::TRIANGLE_LIST,
            vk::PolygonMode::FILL,
        );
        data.grid_pipeline = RRPipeline::new(
            &rrdevice,
            &data.rrswapchain,
            &data.rrrender,
            &data.grid_descriptor_set,
            "src/shaders/gridVert.spv",
            "src/shaders/gridFrag.spv",
            vk::PrimitiveTopology::LINE_LIST,
            vk::PolygonMode::LINE,
        );

        // Gizmo用のディスクリプタセットとパイプラインを作成
        data.gizmo_descriptor_set = RRDescriptorSet::new(&rrdevice, &data.rrswapchain);

        // Gizmo用のuniform bufferを作成（テクスチャは不要、グリッドと同じ方法）
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

        data.gizmo_pipeline = PipelineBuilder::new("src/shaders/gizmoVert.spv", "src/shaders/gizmoFrag.spv")
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
        data.gizmo_data = GizmoData::new();
        data.gizmo_data.create_buffers(&instance, &rrdevice, &data.rrcommand_pool)
            .expect("Failed to create gizmo buffers");

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
        color = Vec4::new(0.0, 1.0, 0.0, 1.0);
        if let Err(e) = Self::create_grid_data(&mut data, 1, color, tex_coord) {
            eprintln!("{:?}", e)
        }
        color = Vec4::new(0.0, 0.0, 1.0, 1.0);
        if let Err(e) = Self::create_grid_data(&mut data, 2, color, tex_coord) {
            eprintln!("{:?}", e)
        }
        println!("created grid data ");
        // let _ = Self::create_texture_image(&instance, &device, &mut data)?;
        // data.texture_image = RRImage::new(&instance, &rrdevice, &data.rrcommand_pool.borrow_mut());
        data.grid_vertex_buffer = RRVertexBuffer::new(
            &instance,
            &rrdevice,
            &data.rrcommand_pool,
            (size_of::<data::Vertex>() * data.grid_vertices.len()) as vk::DeviceSize,
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
        // Vulkan Y-down coordinate system: View from diagonal position
        data.initial_camera_pos = [5.0, -3.0, -5.0];
        data.camera_pos = data.initial_camera_pos;
        let camera_pos = vec3(data.camera_pos[0], data.camera_pos[1], data.camera_pos[2]);
        // Look at origin (0, 0, 0)
        let camera_direction = (vec3(0.0, 0.0, 0.0) - camera_pos).normalize();
        // Y-down (Vulkan coordinate system)
        let camera_up = vec3(0.0, -1.0, 0.0);
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

    unsafe fn render(&mut self, window: &Window, gui_data: &mut GUIData, draw_data: &imgui::DrawData) -> Result<()> {
        // Check if a new model file was selected
        if gui_data.file_changed {
            log!("Loading new model from: {}", gui_data.selected_model_path);

            // Wait for device to finish all operations before reloading
            self.rrdevice.device.device_wait_idle()?;

            match Self::load_model_from_path(
                &self.instance,
                &self.rrdevice,
                &mut self.data,
                &gui_data.selected_model_path,
            ) {
                Ok(_) => {
                    gui_data.load_status = format!("Loaded: {}", gui_data.selected_model_path);
                    log!("Successfully loaded model: {}", gui_data.selected_model_path);
                }
                Err(e) => {
                    gui_data.load_status = format!("Error: {}", e);
                    log!("Failed to load model: {:?}", e);
                }
            }

            gui_data.file_changed = false;
        }

        // Acquire an image from the swapchain
        // Execute the command buffer with that image as attachment in the framebuffer
        // Return the image to the swapchain for presentation
        self.rrdevice.device.wait_for_fences(
            &[self.data.in_flight_fences[self.frame]],
            true,
            u64::MAX,
        )?; // wait until all fences signaled

        let result = self.rrdevice.device.acquire_next_image_khr(
            self.data.rrswapchain.swapchain,
            u64::MAX,
            self.data.image_available_semaphores[self.frame],
            vk::Fence::null(),
        );

        let image_index = match result {
            Ok((image_index, _)) => image_index as usize,
            // TODO: Err(vk::ErrorCode::OUT_OF_DATE_KHR) => return self.recreate_swapchain(window),
            Err(e) => return Err(anyhow!(e)),
        };

        // sync CPU(swapchain image)
        if !self.data.images_in_flight[image_index as usize].is_null() {
            self.rrdevice.device.wait_for_fences(
                &[self.data.images_in_flight[image_index as usize]],
                true,
                u64::MAX,
            )?;
        }

        self.data.images_in_flight[image_index as usize] = self.data.in_flight_fences[self.frame];

        // FBXアニメーション更新
        if self.data.fbx_model.animation_count() > 0 {
            if !self.data.animation_playing {
                // アニメーションが一時停止中の場合のみ、最初のフレームでログを出力
                static mut LOGGED_PAUSED: bool = false;
                unsafe {
                    if !LOGGED_PAUSED {
                        log!("FBX animation is paused (animation_playing=false)");
                        LOGGED_PAUSED = true;
                    }
                }
            } else {
                // 経過時間を取得
                let elapsed = self.start.elapsed().as_secs_f32();

                // アニメーション時間を更新
                if let Some(duration) = self.data.fbx_model.get_animation_duration(self.data.current_animation_index) {
                    // Static pose (duration == 0) or animated
                    if duration > 0.0 {
                        // ループ再生（アニメーション）
                        let prev_time = self.data.animation_time;
                        self.data.animation_time = elapsed % duration;

                        // Log every 10 frames for debugging (avoid log spam)
                        static mut FRAME_COUNT: u32 = 0;
                        unsafe {
                            FRAME_COUNT += 1;
                            if FRAME_COUNT % 10 == 0 {
                                log!("Updating FBX animation: time={:.4}/{:.4}s (elapsed={:.4}, prev={:.4})",
                                     self.data.animation_time, duration, elapsed, prev_time);
                            }
                        }

                        // アニメーションを適用
                        self.data.fbx_model.update_animation(self.data.current_animation_index, self.data.animation_time);

                        // 頂点バッファを更新
                        Self::update_fbx_vertex_buffer(&self.instance, &self.rrdevice, &mut self.data)?;
                    } else {
                        // Static pose (duration == 0): keep time at 0, no need to update every frame
                        // Initial pose was already applied in load_model_from_path
                        static mut LOGGED_STATIC: bool = false;
                        unsafe {
                            if !LOGGED_STATIC {
                                log!("FBX animation has duration=0 (static pose)");
                                LOGGED_STATIC = true;
                            }
                        }
                    }
                } else {
                    static mut LOGGED_NO_DURATION: bool = false;
                    unsafe {
                        if !LOGGED_NO_DURATION {
                            log!("FBX animation has no duration (get_animation_duration returned None)");
                            LOGGED_NO_DURATION = true;
                        }
                    }
                }
            }
        }

        // Apply animation for glTF models (skeletal or node animation)
        if !self.data.gltf_model.gltf_data.is_empty() {
            let time = self.start.elapsed().as_secs_f32();

            // Log every 60 frames (approximately 1 second at 60fps)
            static mut FRAME_COUNT: u32 = 0;
            unsafe {
                FRAME_COUNT += 1;
                if FRAME_COUNT % 60 == 0 {
                    if self.data.gltf_model.has_skinned_meshes {
                        log!("Updating glTF skeletal animation: time={:.4}s, joint_animations={}, gltf_data={}",
                             time, self.data.gltf_model.joint_animations.len(), self.data.gltf_model.gltf_data.len());
                    } else {
                        log!("Updating glTF node animation: time={:.4}s, node_animations={}, gltf_data={}",
                             time, self.data.gltf_model.node_animations.len(), self.data.gltf_model.gltf_data.len());
                    }
                }
            }

            if self.data.gltf_model.has_skinned_meshes {
                // Skeletal animation: use joint transforms with weights
                self.data
                    .gltf_model
                    .reset_vertices_animation_position(time);
                self.data.gltf_model.apply_animation(
                    time,
                    0,
                    Matrix4::identity(),
                );
            } else {
                // Node animation: transform nodes and propagate to children
                self.data
                    .gltf_model
                    .reset_vertices_animation_position(time);
            }

            Self::update_vertex_buffer(&self.instance, &self.rrdevice, &mut self.data)?;
        }

        self.update_uniform_buffer(
            image_index,
            gui_data.mouse_pos,
            gui_data.mouse_wheel,
            gui_data,
        )?;

        // Update ImGui buffers
        Self::update_imgui_buffers(&self.instance, &self.rrdevice, &mut self.data, draw_data)?;

        // Record command buffer with 3D rendering and ImGui
        self.record_command_buffer(image_index, draw_data)?;

        let wait_semaphores = &[self.data.image_available_semaphores[self.frame]];
        let wait_stages = &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let command_buffers = &[self.data.rrcommand_buffer.command_buffers[image_index]];
        let signal_semaphores = &[self.data.render_finish_semaphores[self.frame]];
        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(wait_semaphores)
            .wait_dst_stage_mask(wait_stages) // Each entry in the wait_stages array corresponds to the semaphore with the same index in wait_semaphores.
            .command_buffers(command_buffers)
            .signal_semaphores(signal_semaphores);

        self.rrdevice
            .device
            .reset_fences(&[self.data.in_flight_fences[self.frame]])?;
        self.rrdevice.device.queue_submit(
            self.rrdevice.graphics_queue,
            &[submit_info],
            self.data.in_flight_fences[self.frame],
        )?;

        let swapchains = &[self.data.rrswapchain.swapchain];
        let image_indices = &[image_index as u32];
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(signal_semaphores)
            .swapchains(swapchains)
            .image_indices(image_indices);
        let present_result = self
            .rrdevice
            .device
            .queue_present_khr(self.rrdevice.present_queue, &present_info);
        let changed = present_result == Ok(vk::SuccessCode::SUBOPTIMAL_KHR)
            || present_result == Err(vk::ErrorCode::OUT_OF_DATE_KHR);

        if changed || self.resized {
            self.resized = false;
            // TODO: self.recreate_swapchain(window)?;
        } else if let Err(e) = present_result {
            return Err(anyhow!(e));
        }

        // Handle screenshot request
        if gui_data.take_screenshot {
            log!("Taking screenshot...");
            self.save_screenshot(image_index)?;
            gui_data.take_screenshot = false;
            log!("Screenshot saved!");
        }

        self.frame = (self.frame + 1) % MAX_FRAMES_IN_FLIGHT;

        Ok(())
    }

    unsafe fn save_screenshot(&self, image_index: usize) -> Result<()> {
        use std::fs::File;
        use std::io::BufWriter;
        use std::time::SystemTime;

        let device = &self.rrdevice.device;
        let swapchain_image = self.data.rrswapchain.swapchain_images[image_index];
        let extent = self.data.rrswapchain.swapchain_extent;
        let width = extent.width;
        let height = extent.height;

        // Create a buffer to copy the image to
        let image_size = (width * height * 4) as vk::DeviceSize; // RGBA format
        let buffer_info = vk::BufferCreateInfo::builder()
            .size(image_size)
            .usage(vk::BufferUsageFlags::TRANSFER_DST)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = device.create_buffer(&buffer_info, None)?;

        // Allocate memory for the buffer
        let mem_requirements = device.get_buffer_memory_requirements(buffer);
        let memory_type_index = self.get_memory_type_index(
            mem_requirements.memory_type_bits,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let alloc_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(mem_requirements.size)
            .memory_type_index(memory_type_index);

        let buffer_memory = device.allocate_memory(&alloc_info, None)?;
        device.bind_buffer_memory(buffer, buffer_memory, 0)?;

        // Create a command buffer for the copy operation
        let command_pool = &self.data.rrcommand_pool.command_pool;
        let alloc_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(*command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let command_buffers = device.allocate_command_buffers(&alloc_info)?;
        let command_buffer = command_buffers[0];

        // Begin command buffer
        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        device.begin_command_buffer(command_buffer, &begin_info)?;

        // Transition image layout to TRANSFER_SRC_OPTIMAL
        let barrier = vk::ImageMemoryBarrier::builder()
            .old_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(swapchain_image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .src_access_mask(vk::AccessFlags::MEMORY_READ)
            .dst_access_mask(vk::AccessFlags::TRANSFER_READ);

        device.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::TRANSFER,
            vk::DependencyFlags::empty(),
            &[] as &[vk::MemoryBarrier],
            &[] as &[vk::BufferMemoryBarrier],
            &[barrier.build()],
        );

        // Copy image to buffer
        let region = vk::BufferImageCopy::builder()
            .buffer_offset(0)
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            })
            .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
            .image_extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            });

        device.cmd_copy_image_to_buffer(
            command_buffer,
            swapchain_image,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            buffer,
            &[region.build()],
        );

        // Transition image layout back to PRESENT_SRC_KHR
        let barrier = vk::ImageMemoryBarrier::builder()
            .old_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
            .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(swapchain_image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .src_access_mask(vk::AccessFlags::TRANSFER_READ)
            .dst_access_mask(vk::AccessFlags::MEMORY_READ);

        device.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::TRANSFER,
            vk::DependencyFlags::empty(),
            &[] as &[vk::MemoryBarrier],
            &[] as &[vk::BufferMemoryBarrier],
            &[barrier.build()],
        );

        // End and submit command buffer
        device.end_command_buffer(command_buffer)?;

        let command_buffers_slice = [command_buffer];
        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(&command_buffers_slice);
        device.queue_submit(self.rrdevice.graphics_queue, &[submit_info.build()], vk::Fence::null())?;
        device.queue_wait_idle(self.rrdevice.graphics_queue)?;

        // Map memory and read data
        let data = device.map_memory(buffer_memory, 0, image_size, vk::MemoryMapFlags::empty())?;
        let slice = std::slice::from_raw_parts(data as *const u8, image_size as usize);

        // Convert BGRA to RGBA
        let mut rgba_data = vec![0u8; (width * height * 4) as usize];
        for y in 0..height {
            for x in 0..width {
                let i = ((y * width + x) * 4) as usize;
                rgba_data[i] = slice[i + 2];     // R = B
                rgba_data[i + 1] = slice[i + 1]; // G = G
                rgba_data[i + 2] = slice[i];     // B = R
                rgba_data[i + 3] = slice[i + 3]; // A = A
            }
        }

        device.unmap_memory(buffer_memory);

        // Generate filename with timestamp
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs();
        let filename = format!("log/screenshot_{}.png", timestamp);

        // Ensure log directory exists
        std::fs::create_dir_all("log")?;

        // Save as PNG
        let file = File::create(&filename)?;
        let writer = BufWriter::new(file);
        let mut encoder = png::Encoder::new(writer, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&rgba_data)?;

        log!("Screenshot saved to: {}", filename);

        // Cleanup
        device.free_command_buffers(*command_pool, &[command_buffer]);
        device.free_memory(buffer_memory, None);
        device.destroy_buffer(buffer, None);

        Ok(())
    }

    unsafe fn get_memory_type_index(
        &self,
        type_filter: u32,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<u32> {
        let mem_properties = self.instance.get_physical_device_memory_properties(self.rrdevice.physical_device);

        for i in 0..mem_properties.memory_type_count {
            let has_type = (type_filter & (1 << i)) != 0;
            let has_properties = mem_properties.memory_types[i as usize]
                .property_flags
                .contains(properties);

            if has_type && has_properties {
                return Ok(i);
            }
        }

        Err(anyhow!("Failed to find suitable memory type"))
    }

    unsafe fn destroy(&mut self) {
        log!("Destroying application resources...");

        if let Some(sampler) = self.data.gbuffer_sampler {
            self.rrdevice.device.destroy_sampler(sampler, None);
            log!("Destroyed G-Buffer sampler");
        }

        if let Some(mut gbuffer_descriptor) = self.data.gbuffer_descriptor_set.take() {
            gbuffer_descriptor.destroy(&self.rrdevice.device);
            log!("Destroyed G-Buffer descriptor set");
        }

        if let Some(gbuffer_pipeline) = self.data.gbuffer_pipeline.take() {
            gbuffer_pipeline.destroy(&self.rrdevice.device);
            log!("Destroyed G-Buffer pipeline");
        }

        if let Some(mut composite_descriptor) = self.data.composite_descriptor.take() {
            composite_descriptor.destroy(&self.rrdevice.device);
            log!("Destroyed composite descriptor set");
        }

        if let Some(composite_pipeline) = self.data.composite_pipeline.take() {
            composite_pipeline.destroy(&self.rrdevice.device);
            log!("Destroyed composite pipeline");
        }

        if let (Some(buffer), Some(memory)) = (
            self.data.scene_uniform_buffer,
            self.data.scene_uniform_buffer_memory,
        ) {
            self.rrdevice.device.destroy_buffer(buffer, None);
            self.rrdevice.device.free_memory(memory, None);
            log!("Destroyed scene uniform buffer");
        }

        if let Some(mut ray_query_descriptor) = self.data.ray_query_descriptor.take() {
            ray_query_descriptor.destroy(&self.rrdevice.device);
            log!("Destroyed ray query descriptor set");
        }

        if let Some(ray_query_pipeline) = self.data.ray_query_pipeline.take() {
            ray_query_pipeline.destroy(&self.rrdevice.device);
            log!("Destroyed ray query pipeline");
        }

        if let Some(mut acceleration_structure) = self.data.acceleration_structure.take() {
            acceleration_structure.destroy(&self.rrdevice.device);
            log!("Destroyed acceleration structure");
        }

        if let Some(mut gbuffer) = self.data.gbuffer.take() {
            gbuffer.destroy(&self.rrdevice.device);
            log!("Destroyed G-Buffer");
        }

        log!("All application resources destroyed");
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

        // Vulkan validation layerのメッセージは標準logクレートでコンソールに出力
        use log::{error, warn, debug, trace};
        if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::ERROR {
            error!("({:?}) {}", type_, message);
        } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::WARNING {
            warn!("({:?}) {}", type_, message);
        } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::INFO {
            debug!("({:?}) {}", type_, message);
        } else {
            trace!("({:?}) {}", type_, message);
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

    //unsafe fn recreate_swapchain(&mut self, window: &Window) -> Result<()> {
    // self.rrdevice.device.device_wait_idle()?;
    // self.destroy_swapchain();
    // Self::create_swapchain(
    //     window,
    //     &self.instance,
    //     &self.rrdevice.device,
    //     &mut self.data,
    // )?;
    // Self::create_swapchain_image_view(&self.rrdevice.device, &mut self.data)?;
    // Self::create_render_pass(&self.instance, &self.rrdevice.device, &mut self.data)?;
    // Self::create_pipeline(&self.rrdevice.device, &mut self.data)?;
    // Self::create_color_objects(&self.instance, &self.rrdevice.device, &mut self.data)?;
    // Self::create_depth_objects(&self.instance, &self.rrdevice.device, &mut self.data)?;
    // Self::create_framebuffers(&self.rrdevice.device, &mut self.data)?;
    // Self::create_uniform_buffers(&self.instance, &self.rrdevice.device, &mut self.data)?;
    // Self::create_uniform_buffers_grid(&self.instance, &self.rrdevice.device, &mut self.data)?;
    // Self::create_descriptor_pool(&self.rrdevice.device, &mut self.data)?;
    // Self::create_descriptor_sets(&self.rrdevice.device, &mut self.data)?;
    // Self::create_descriptor_sets_grid(&self.rrdevice.device, &mut self.data)?;
    // Self::create_command_buffers(&self.rrdevice.device, &mut self.data)?;
    // self.data
    //     .images_in_flight
    //     .resize(self.data.swapchain_images.len(), vk::Fence::null());
    //
    //Ok(())
    // }

    unsafe fn destroy_swapchain(&mut self) {
        // // depth objects
        // self.rrdevice
        //     .device
        //     .destroy_image(self.data.depth_image, None);
        // self.rrdevice
        //     .device
        //     .free_memory(self.data.depth_image_memory, None);
        // self.rrdevice
        //     .device
        //     .destroy_image_view(self.data.depth_image_view, None);
        // // color objects
        // self.rrdevice
        //     .device
        //     .destroy_image(self.data.color_image, None);
        // self.rrdevice
        //     .device
        //     .free_memory(self.data.color_image_memory, None);
        // self.rrdevice
        //     .device
        //     .destroy_image_view(self.data.color_image_view, None);
        // // descriptor pool
        // self.rrdevice
        //     .device
        //     .destroy_descriptor_pool(self.data.descriptor_pool, None);
        // // uniform buffers
        // self.data
        //     .uniform_buffers
        //     .iter()
        //     .for_each(|b| self.rrdevice.device.destroy_buffer(*b, None));
        // self.data
        //     .uniform_buffer_memories
        //     .iter()
        //     .for_each(|m| self.rrdevice.device.free_memory(*m, None));
        // // framebuffers
        // self.data
        //     .framebuffers
        //     .iter()
        //     .for_each(|f| self.rrdevice.device.destroy_framebuffer(*f, None));
        // // command buffers
        // self.rrdevice
        //     .device
        //     .free_command_buffers(self.data.command_pool, &self.data.command_buffers);
        // // The pipeline layout will be referenced throughout the program's lifetime
        // self.rrdevice
        //     .device
        //     .destroy_pipeline_layout(self.data.pipeline_layout, None);
        // // render pass
        // self.rrdevice
        //     .device
        //     .destroy_render_pass(self.data.render_pass, None);
        // // graphics pipeline
        // self.rrdevice
        //     .device
        //     .destroy_pipeline(self.data.pipeline, None);
        // // swapchain imageviews
        // self.data
        //     .swapchain_image_views
        //     .iter()
        //     .for_each(|v| self.rrdevice.device.destroy_image_view(*v, None));
        // // swapchain
        // self.rrdevice
        //     .device
        //     .destroy_swapchain_khr(self.data.swapchain, None);
    }

    unsafe fn create_grid_data(
        data: &mut AppData,
        index: i32,
        color: Vec4,
        tex_coord: Vec2,
    ) -> Result<()> {
        for i in 0..100 {
            let mut pos1 = Vec3::new(0.0, 0.0, 0.0);
            if index == 0 {
                // y = 0
                pos1.x = 100.0;
                pos1.z = i as f32 * 0.1;
            } else if index == 1 {
                // x = 0
                pos1.z = i as f32 * 0.1;
                pos1.y = 100.0;
            } else if index == 2 {
                // y = 0
                pos1.x = i as f32 * 0.1;
                pos1.z = 100.0;
            }
            let mut pos2 = Vec3::new(0.0, 0.0, 0.0);
            if index == 0 {
                // y = 0
                pos2.x = -100.0;
                pos2.z = pos1.z;
            } else if index == 1 {
                // fix x coordinate
                pos2.z = pos1.z;
                pos2.y = -100.0;
            } else if index == 2 {
                // fix z coordinate
                pos2.x = pos1.x;
                pos2.z = -100.0;
            }
            let vertex1 = data::Vertex::new(pos1, color, tex_coord);
            let vertex2 = data::Vertex::new(pos2, color, tex_coord);
            let vertex3 = data::Vertex::new(-pos1, color, tex_coord);
            let vertex4 = data::Vertex::new(-pos2, color, tex_coord);
            data.grid_vertices.push(vertex1);
            data.grid_indices.push(data.grid_indices.len() as u32);
            data.grid_vertices.push(vertex2);
            data.grid_indices.push(data.grid_indices.len() as u32);
            data.grid_vertices.push(vertex3);
            data.grid_indices.push(data.grid_indices.len() as u32);
            data.grid_vertices.push(vertex4);
            data.grid_indices.push(data.grid_indices.len() as u32);
        }

        Ok(())
    }

    unsafe fn update_uniform_buffer(
        &mut self,
        image_index: usize,
        mouse_pos: [f32; 2],
        mouse_wheel: f32,
        gui_data: &mut GUIData,
    ) -> Result<()> {
        //let mut model = Mat4::from_axis_angle(vec3(0.0, 0.0, 1.0), Deg(0.0));
        // update vertex buffer
        self.morphing(self.start.elapsed().as_secs_f32());

        // Note: Animation updates are now handled in draw() method before rendering

        // update uniform buffer
        let model = Mat4::identity();

        let mut camera_pos = vec3_from_array(self.data.camera_pos);
        let mut camera_direction = vec3_from_array(self.data.camera_direction);
        let mut camera_up = vec3_from_array(self.data.camera_up);

        let mouse_pos = Vector2::new(mouse_pos[0], mouse_pos[1]);

        // Unity-style camera rotation (Y-down coordinate system):
        // - Horizontal rotation: Always around world Y-down axis (0, -1, 0) - prevents gimbal lock
        // - Vertical rotation: Around camera's local right axis
        let world_y_down = vec3(0.0, -1.0, 0.0);  // Y-down world axis (fixed)
        let camera_right = camera_direction.cross(camera_up).normalize();

        // For pan operation, use view-based axes
        let last_view = view(camera_pos, camera_direction, camera_up);
        let base_x_4 = last_view * vec4(1.0, 0.0, 0.0, 0.0);
        let base_y_4 = last_view * vec4(0.0, -1.0, 0.0, 0.0);
        let base_x = vec3(base_x_4.x, base_x_4.y, base_x_4.z);
        let base_y = vec3(base_y_4.x, base_y_4.y, base_y_4.z);

        // Camera rotation logging counter
        static mut ROTATION_LOG_COUNTER: u32 = 0;

        if gui_data.is_left_clicked || self.data.is_left_clicked {
            // first clicked
            if !self.data.is_left_clicked {
                self.data.clicked_mouse_pos = [mouse_pos[0], mouse_pos[1]];
                self.data.is_left_clicked = true;
            }
            let clicked_mouse_pos = vec2_from_array(self.data.clicked_mouse_pos);

            // FIX: Use delta from previous frame (Unity-style) instead of cumulative diff
            let diff = mouse_pos - clicked_mouse_pos;
            let distance = Vector2::distance(mouse_pos, clicked_mouse_pos);
            gui_data.monitor_value = distance;
            if 0.001 < distance {
                let mut rotate_x = Mat3::identity();
                let mut rotate_y = Mat3::identity();
                let theta_x = -diff.x * 0.005;
                let theta_y = diff.y * 0.005;  // Inverted for intuitive up/down rotation

                // Horizontal rotation: Around world Y-down axis (Unity-style, gimbal-lock free)
                let _ = rodrigues(
                    &mut rotate_x,
                    Rad(theta_x).cos(),
                    Rad(theta_x).sin(),
                    &world_y_down,
                );
                // Vertical rotation: Around camera's local right axis
                let _ = rodrigues(
                    &mut rotate_y,
                    Rad(theta_y).cos(),
                    Rad(theta_y).sin(),
                    &camera_right,
                );

                // Log rotation info every 30 frames
                unsafe {
                    ROTATION_LOG_COUNTER += 1;
                    if ROTATION_LOG_COUNTER % 30 == 0 {
                        log!("=== Camera Rotation Debug (frame {}) ===", ROTATION_LOG_COUNTER);
                        log!("  Mouse diff: ({:.3}, {:.3}), theta: ({:.3}, {:.3})",
                             diff.x, diff.y, theta_x, theta_y);
                        log!("  Before rotation:");
                        log!("    direction: ({:.3}, {:.3}, {:.3})",
                             camera_direction.x, camera_direction.y, camera_direction.z);
                        log!("    up: ({:.3}, {:.3}, {:.3})",
                             camera_up.x, camera_up.y, camera_up.z);
                        log!("    right: ({:.3}, {:.3}, {:.3})",
                             camera_right.x, camera_right.y, camera_right.z);
                        log!("  Rotation axes:");
                        log!("    horizontal (world Y-down): ({:.3}, {:.3}, {:.3})",
                             world_y_down.x, world_y_down.y, world_y_down.z);
                        log!("    vertical (camera right): ({:.3}, {:.3}, {:.3})",
                             camera_right.x, camera_right.y, camera_right.z);
                    }
                }

                let rotate = rotate_y * rotate_x;
                camera_up = rotate * camera_up;
                camera_direction = rotate * camera_direction;

                // Re-orthogonalize camera vectors to prevent drift and maintain stability
                camera_direction = camera_direction.normalize();
                let camera_right_new = camera_direction.cross(camera_up).normalize();
                camera_up = camera_right_new.cross(camera_direction).normalize();

                // Log after rotation
                unsafe {
                    if ROTATION_LOG_COUNTER % 30 == 0 {
                        log!("  After rotation & re-orthogonalization:");
                        log!("    direction: ({:.3}, {:.3}, {:.3})",
                             camera_direction.x, camera_direction.y, camera_direction.z);
                        log!("    up: ({:.3}, {:.3}, {:.3})",
                             camera_up.x, camera_up.y, camera_up.z);
                        log!("    right: ({:.3}, {:.3}, {:.3})",
                             camera_right_new.x, camera_right_new.y, camera_right_new.z);

                        // Check orthogonality
                        let dot_dir_up = camera_direction.dot(camera_up);
                        let dot_dir_right = camera_direction.dot(camera_right_new);
                        let dot_up_right = camera_up.dot(camera_right_new);
                        log!("  Orthogonality check (should be ~0):");
                        log!("    direction·up: {:.6}", dot_dir_up);
                        log!("    direction·right: {:.6}", dot_dir_right);
                        log!("    up·right: {:.6}", dot_up_right);
                    }
                }

                // Update camera state every frame (not just on release)
                self.data.camera_direction = array3_from_vec(camera_direction);
                self.data.camera_up = array3_from_vec(camera_up);

                // Update previous mouse position every frame for delta calculation
                self.data.clicked_mouse_pos = [mouse_pos[0], mouse_pos[1]];
            }

            if !gui_data.is_left_clicked {
                // left button released
                self.data.is_left_clicked = false;
            }
        }

        if gui_data.is_wheel_clicked || self.data.is_wheel_clicked {
            // first clicked
            if !self.data.is_wheel_clicked {
                self.data.clicked_mouse_pos = [mouse_pos[0], mouse_pos[1]];
                self.data.is_wheel_clicked = true;
            }
            let clicked_mouse_pos = vec2_from_array(self.data.clicked_mouse_pos);

            // FIX: Use delta from previous frame (Unity-style) instead of cumulative diff
            let diff = mouse_pos - clicked_mouse_pos;
            let distance = Vector2::distance(mouse_pos, clicked_mouse_pos);
            gui_data.monitor_value = distance;
            if 0.001 < distance {
                let translate_x_v = base_x * -diff.x;
                let translate_y_v = base_y * diff.y;
                camera_pos += translate_x_v + translate_y_v;

                // Update camera position every frame (not just on release)
                self.data.camera_pos = array3_from_vec(camera_pos);

                // Update previous mouse position every frame for delta calculation
                self.data.clicked_mouse_pos = [mouse_pos[0], mouse_pos[1]];
            }

            if !gui_data.is_wheel_clicked {
                // left button released
                self.data.is_wheel_clicked = false;
            }
        }

        if mouse_wheel != 0.0 {
            let diff_view = camera_direction * mouse_wheel * -5.0;
            camera_pos += diff_view;
            self.data.camera_pos = array3_from_vec(camera_pos);
        }

        let view = view(camera_pos, camera_direction, camera_up);

        let correction = Mat4::new(
            // column-major order
            1.0,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
            0.0, // cgmath was originally designed for OpenGL, where the Y coordinate of the clip coordinates is inverted.
            0.0,
            0.0,
            1.0 / 2.0,
            0.0, // depth [-1.0, 1.0] (OpenGL) -> [0.0, 1.0] (Vulkan)
            0.0,
            0.0,
            1.0 / 2.0,
            1.0,
        );
        let proj = correction
            * cgmath::perspective(
            Deg(45.0),
            self.data.rrswapchain.swapchain_extent.width as f32
                / self.data.rrswapchain.swapchain_extent.height as f32,
            0.1,
            1000.0,
        );

        for i in 0..self.data.model_descriptor_set.rrdata.len() {
            let rrdata = &mut self.data.model_descriptor_set.rrdata[i];
            let ubo = UniformBufferObject { model, view, proj };
            let ubo_memory = rrdata.rruniform_buffers[image_index].buffer_memory;
            let memory = self.rrdevice.device.map_memory(
                ubo_memory,
                0,
                size_of::<UniformBufferObject>() as u64,
                vk::MemoryMapFlags::empty(),
            )?;
            memcpy(&ubo, memory.cast(), 1);
            self.rrdevice.device.unmap_memory(ubo_memory);
        }

        // Update Scene Uniform Buffer for Ray Tracing
        if let (Some(scene_buffer), Some(scene_memory)) =
            (self.data.scene_uniform_buffer, self.data.scene_uniform_buffer_memory)
        {
            let scene_data = SceneUniformData {
                light_position: Vec4::new(5.0, 5.0, 5.0, 1.0),
                light_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
                view,
                proj,
            };

            let data_ptr = self.rrdevice.device.map_memory(
                scene_memory,
                0,
                std::mem::size_of::<SceneUniformData>() as u64,
                vk::MemoryMapFlags::empty(),
            )?;

            std::ptr::copy_nonoverlapping(
                &scene_data as *const SceneUniformData,
                data_ptr as *mut SceneUniformData,
                1,
            );

            self.rrdevice.device.unmap_memory(scene_memory);
        }

        // update for grid
        let model_grid = Mat4::identity();
        for i in 0..self.data.grid_descriptor_set.rrdata.len() {
            let rrdata = &mut self.data.grid_descriptor_set.rrdata[i];
            let grid_ubo_memory = rrdata.rruniform_buffers[image_index].buffer_memory;
            let ubo_grid = UniformBufferObject {
                model: model_grid,
                view: view,
                proj: proj,
            };
            let memory_grid = self.rrdevice.device.map_memory(
                grid_ubo_memory,
                0,
                size_of::<UniformBufferObject>() as u64,
                vk::MemoryMapFlags::empty(),
            )?;
            memcpy(&ubo_grid, memory_grid.cast(), 1);
            self.rrdevice
                .device
                .unmap_memory(rrdata.rruniform_buffers[image_index].buffer_memory);
        }

        // Gizmo用のuniform bufferを更新
        for i in 0..self.data.gizmo_descriptor_set.rrdata.len() {
            let rrdata = &mut self.data.gizmo_descriptor_set.rrdata[i];
            let gizmo_ubo_memory = rrdata.rruniform_buffers[image_index].buffer_memory;
            let ubo_gizmo = UniformBufferObject {
                model: Mat4::identity(),
                view: view,
                proj: proj,
            };
            let memory_gizmo = self.rrdevice.device.map_memory(
                gizmo_ubo_memory,
                0,
                size_of::<UniformBufferObject>() as u64,
                vk::MemoryMapFlags::empty(),
            )?;
            memcpy(&ubo_gizmo, memory_gizmo.cast(), 1);
            self.rrdevice
                .device
                .unmap_memory(rrdata.rruniform_buffers[image_index].buffer_memory);
        }

        // Gizmoの頂点をカメラの向きに応じて更新
        // カメラのright/up/direction（forward）ベクトルから直接Gizmo軸を計算
        let camera_right = camera_direction.cross(camera_up).normalize();

        // カメラの向き（forward）は camera_direction
        // X軸（赤）= カメラのright
        // Y軸（緑）= カメラのup
        // Z軸（青）= カメラのdirection（forward）
        let gizmo_rotation = cgmath::Matrix3::from_cols(
            camera_right,      // X軸方向
            camera_up,         // Y軸方向
            camera_direction,  // Z軸方向
        );

        // Gizmoの頂点を更新
        self.data.gizmo_data.update_rotation(&gizmo_rotation);

        // Gizmo方向確認用ログ（60フレームごと）
        static mut GIZMO_LOG_COUNTER: u32 = 0;
        unsafe {
            GIZMO_LOG_COUNTER += 1;
            if GIZMO_LOG_COUNTER % 60 == 0 {
                log!("=== Gizmo Direction Debug (frame {}) ===", GIZMO_LOG_COUNTER);
                log!("Camera state:");
                log!("  position: ({:.3}, {:.3}, {:.3})", camera_pos.x, camera_pos.y, camera_pos.z);
                log!("  direction: ({:.3}, {:.3}, {:.3})", camera_direction.x, camera_direction.y, camera_direction.z);
                log!("  up: ({:.3}, {:.3}, {:.3})", camera_up.x, camera_up.y, camera_up.z);

                log!("  right: ({:.3}, {:.3}, {:.3})", camera_right.x, camera_right.y, camera_right.z);

                log!("Gizmo rotation matrix (from camera vectors):");
                log!("  X-axis (red):   [{:.3}, {:.3}, {:.3}] = camera right", gizmo_rotation.x.x, gizmo_rotation.x.y, gizmo_rotation.x.z);
                log!("  Y-axis (green): [{:.3}, {:.3}, {:.3}] = camera up", gizmo_rotation.y.x, gizmo_rotation.y.y, gizmo_rotation.y.z);
                log!("  Z-axis (blue):  [{:.3}, {:.3}, {:.3}] = camera direction", gizmo_rotation.z.x, gizmo_rotation.z.y, gizmo_rotation.z.z);

                log!("Gizmo vertices (after rotation):");
                log!("  Origin: ({:.3}, {:.3}, {:.3})",
                     self.data.gizmo_data.vertices[0].pos[0],
                     self.data.gizmo_data.vertices[0].pos[1],
                     self.data.gizmo_data.vertices[0].pos[2]);
                log!("  X-axis (red): ({:.3}, {:.3}, {:.3})",
                     self.data.gizmo_data.vertices[1].pos[0],
                     self.data.gizmo_data.vertices[1].pos[1],
                     self.data.gizmo_data.vertices[1].pos[2]);
                log!("  Y-axis (green): ({:.3}, {:.3}, {:.3})",
                     self.data.gizmo_data.vertices[2].pos[0],
                     self.data.gizmo_data.vertices[2].pos[1],
                     self.data.gizmo_data.vertices[2].pos[2]);
                log!("  Z-axis (blue): ({:.3}, {:.3}, {:.3})",
                     self.data.gizmo_data.vertices[3].pos[0],
                     self.data.gizmo_data.vertices[3].pos[1],
                     self.data.gizmo_data.vertices[3].pos[2]);
            }
        }

        // 頂点バッファを更新（デバイスローカルメモリなので、staging bufferを使う必要があります）
        // 今回は簡単のため、毎フレーム再作成します
        if let Some(vertex_buffer) = self.data.gizmo_data.vertex_buffer {
            self.rrdevice.device.destroy_buffer(vertex_buffer, None);
        }
        if let Some(vertex_buffer_memory) = self.data.gizmo_data.vertex_buffer_memory {
            self.rrdevice.device.free_memory(vertex_buffer_memory, None);
        }

        // 頂点バッファを再作成
        let vertex_buffer_size = (size_of::<GizmoVertex>() * self.data.gizmo_data.vertices.len()) as u64;
        let (staging_buffer, staging_buffer_memory) = create_buffer(
            &self.instance,
            &self.rrdevice,
            vertex_buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let data_ptr = self.rrdevice
            .device
            .map_memory(staging_buffer_memory, 0, vertex_buffer_size, vk::MemoryMapFlags::empty())?;
        std::ptr::copy_nonoverlapping(self.data.gizmo_data.vertices.as_ptr(), data_ptr.cast(), self.data.gizmo_data.vertices.len());
        self.rrdevice.device.unmap_memory(staging_buffer_memory);

        let (vertex_buffer, vertex_buffer_memory) = create_buffer(
            &self.instance,
            &self.rrdevice,
            vertex_buffer_size,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        copy_buffer(
            &self.rrdevice,
            &self.data.rrcommand_pool,
            staging_buffer,
            vertex_buffer,
            vertex_buffer_size,
        )?;

        self.rrdevice.device.destroy_buffer(staging_buffer, None);
        self.rrdevice.device.free_memory(staging_buffer_memory, None);

        self.data.gizmo_data.vertex_buffer = Some(vertex_buffer);
        self.data.gizmo_data.vertex_buffer_memory = Some(vertex_buffer_memory);

        Ok(())
    }

    unsafe fn load_model(
        instance: &Instance,
        rrdevice: &RRDevice,
        data: &mut AppData,
    ) -> Result<()> {
        // fbx model

        let model_path_fbx = "src/resources/phoenix-bird/source/fly.fbx";
        // Use russimp-based loader for better compatibility
        data.fbx_model = load_fbx_with_russimp(model_path_fbx)?;

        // Apply initial pose before creating vertex buffers
        if data.fbx_model.animation_count() > 0 {
            log!("Applying initial pose (time=0) for FBX skeletal animation...");
            data.fbx_model.update_animation(0, 0.0);
        }

        // Create separate rrdata for each FBX mesh
        for (mesh_idx, fbx_data) in data.fbx_model.fbx_data.iter().enumerate() {
            log!("Creating RRData for FBX mesh {}: {} vertices, texture: {:?}",
                mesh_idx, fbx_data.positions.len(), fbx_data.diffuse_texture);

            // Debug: log first vertex position to verify skinning was applied
            if !fbx_data.positions.is_empty() {
                let first_pos = &fbx_data.positions[0];
                log!("DEBUG: Mesh {} first vertex position: ({}, {}, {})", mesh_idx, first_pos.x, first_pos.y, first_pos.z);
            }

            let mut rrdata = RRData::new(&instance, &rrdevice, &data.rrswapchain);

            // Load texture for this mesh
            if let Some(texture_path) = &fbx_data.diffuse_texture {
                log!("Loading texture: {}", texture_path);
                match load_png_image(texture_path) {
                    Ok((image_data, width, height)) => {
                        match create_texture_image_pixel(
                            instance,
                            rrdevice,
                            data.rrcommand_pool.borrow_mut(),
                            &image_data,
                            width,
                            height,
                        ) {
                            Ok((image, image_memory, mip_level)) => {
                                rrdata.image = image;
                                rrdata.image_memory = image_memory;
                                rrdata.mip_level = mip_level;
                                log!("Texture loaded successfully for mesh {}", mesh_idx);
                            }
                            Err(e) => {
                                log!("Failed to create texture image for mesh {}: {}", mesh_idx, e);
                                // Fall back to white texture
                                (rrdata.image, rrdata.image_memory, rrdata.mip_level) = create_texture_image_pixel(
                                    instance,
                                    rrdevice,
                                    data.rrcommand_pool.borrow_mut(),
                                    &vec![255u8, 255, 255, 255],
                                    1,
                                    1,
                                )?;
                            }
                        }
                    }
                    Err(e) => {
                        log!("Failed to load texture file {}: {}", texture_path, e);
                        // Fall back to white texture
                        (rrdata.image, rrdata.image_memory, rrdata.mip_level) = create_texture_image_pixel(
                            instance,
                            rrdevice,
                            data.rrcommand_pool.borrow_mut(),
                            &vec![255u8, 255, 255, 255],
                            1,
                            1,
                        )?;
                    }
                }
            } else {
                log!("No texture specified for mesh {}, using white", mesh_idx);
                // Use white texture as fallback
                (rrdata.image, rrdata.image_memory, rrdata.mip_level) = create_texture_image_pixel(
                    instance,
                    rrdevice,
                    data.rrcommand_pool.borrow_mut(),
                    &vec![255u8, 255, 255, 255],
                    1,
                    1,
                )?;
            }

            // Create vertex data
            rrdata.vertex_data = VertexData::default();
            for (i, position) in fbx_data.positions.iter().enumerate() {
                // Get UV coordinates (or use default if index out of bounds)
                let uv = if i < fbx_data.tex_coords.len() {
                    fbx_data.tex_coords[i]
                } else {
                    [0.5, 0.5]
                };

                let vertex = data::Vertex::new(
                    Vec3::new(position.x, position.y, position.z),
                    Vec4::new(1.0, 1.0, 1.0, 1.0),  // White color for proper texturing
                    Vec2::new_array(uv),             // Use actual UV coordinates
                );
                rrdata.vertex_data.vertices.push(vertex);
            }

            // Set indices
            rrdata.vertex_data.indices = fbx_data.indices.clone();

            data.model_descriptor_set.rrdata.push(rrdata);
        }

        // アニメーションがあれば自動再生を開始
        if data.fbx_model.animation_count() > 0 {
            data.animation_playing = true;
            data.current_animation_index = 0;
            data.animation_time = 0.0;
            log!("FBX animation loaded: {} animations", data.fbx_model.animation_count());
            if let Some(duration) = data.fbx_model.get_animation_duration(0) {
                log!("Animation 0 duration: {} seconds", duration);
            }
        }

        // Set current model path
        let model_path_fbx = "src/resources/phoenix-bird/source/fly.fbx";
        data.current_model_path = model_path_fbx.to_string();

        // Note: descriptor sets and command buffers are created by the initialization code
        // (after this function returns) in the main initialization sequence
        log!("=== FBX model loaded successfully ===");
        Ok(())
    }

    unsafe fn update_vertex_buffer(
        instance: &Instance,
        rrdevice: &RRDevice,
        data: &mut AppData,
    ) -> Result<()> {
        // Only update if glTF model is loaded (check if gltf_data matches rrdata count)
        if data.gltf_model.gltf_data.is_empty() {
            return Ok(());
        }

        // Only update glTF meshes (up to gltf_data.len())
        let gltf_mesh_count = data.gltf_model.gltf_data.len();
        for i in 0..gltf_mesh_count {
            if i >= data.model_descriptor_set.rrdata.len() {
                break;
            }

            let rrdata = &mut data.model_descriptor_set.rrdata[i];
            let vertex_data = &mut rrdata.vertex_data;
            let gltf_data = &data.gltf_model.gltf_data[i];

            for vertex in &gltf_data.vertices {
                vertex_data.vertices[vertex.index].pos.x = vertex.animation_position[0];
                vertex_data.vertices[vertex.index].pos.y = vertex.animation_position[1];
                vertex_data.vertices[vertex.index].pos.z = vertex.animation_position[2];
            }
            if let Err(e) = rrdata.vertex_buffer.update(
                instance,
                rrdevice,
                &data.rrcommand_pool,
                (size_of::<data::Vertex>() * rrdata.vertex_data.vertices.len())
                    as vk::DeviceSize,
                rrdata.vertex_data.vertices.as_ptr() as *const c_void,
                rrdata.vertex_data.vertices.len(),
            ) {
                eprintln!("Failed to update vertex buffer: {}", e);
            }
        }
        Ok(())
    }

    unsafe fn update_fbx_vertex_buffer(
        instance: &Instance,
        rrdevice: &RRDevice,
        data: &mut AppData,
    ) -> Result<()> {
        if data.fbx_model.fbx_data.is_empty() {
            return Ok(());
        }

        // Update each FBX mesh's vertex buffer
        for (mesh_idx, fbx_data) in data.fbx_model.fbx_data.iter().enumerate() {
            if let Some(rrdata) = data.model_descriptor_set.rrdata.get_mut(mesh_idx) {
                let vertex_data = &mut rrdata.vertex_data;

                // Update vertex positions from fbx_data
                for (vertex_idx, pos) in fbx_data.positions.iter().enumerate() {
                    if vertex_idx < vertex_data.vertices.len() {
                        vertex_data.vertices[vertex_idx].pos.x = pos.x;
                        vertex_data.vertices[vertex_idx].pos.y = pos.y;
                        vertex_data.vertices[vertex_idx].pos.z = pos.z;
                    }
                }

                // Update vertex buffer for this mesh
                if let Err(e) = rrdata.vertex_buffer.update(
                    instance,
                    rrdevice,
                    &data.rrcommand_pool,
                    (size_of::<data::Vertex>() * vertex_data.vertices.len()) as vk::DeviceSize,
                    vertex_data.vertices.as_ptr() as *const c_void,
                    vertex_data.vertices.len(),
                ) {
                    eprintln!("Failed to update FBX vertex buffer for mesh {}: {}", mesh_idx, e);
                }
            }
        }

        Ok(())
    }

    unsafe fn reset_camera(&mut self) {
        self.data.camera_pos = self.data.initial_camera_pos;
        let camera_pos = vec3_from_array(self.data.camera_pos);
        // Look at origin (0, 0, 0)
        let camera_direction = (vec3(0.0, 0.0, 0.0) - camera_pos).normalize();
        // Y-down (Vulkan coordinate system)
        let camera_up = vec3(0.0, -1.0, 0.0);
        self.data.camera_direction = array3_from_vec(camera_direction);
        self.data.camera_up = array3_from_vec(camera_up);
    }

    unsafe fn reset_camera_up(&mut self) {
        let camera_pos = vec3_from_array(self.data.camera_pos);
        let mut camera_direction = vec3_from_array(self.data.camera_direction);
        let mut camera_up = vec3_from_array(self.data.camera_up);
        let horizon = Vector3::cross(camera_up, camera_direction);
        // Reset to Y-down (Vulkan coordinate system)
        camera_up = vec3(0.0, -1.0, 0.0);
        camera_direction = Vector3::cross(horizon, camera_up);
        self.data.camera_up = array3_from_vec(camera_up);
        self.data.camera_direction = array3_from_vec(camera_direction);
    }

    // TODO: efficiency
    unsafe fn morphing(&mut self, time: f32) {
        if self.data.gltf_model.morph_animations.len() <= 0 {
            return;
        }

        for i in 0..self.data.gltf_model.gltf_data.len() {
            let animation_index = self.data.gltf_model.morph_target_index(time);

            let gltf_model = &mut self.data.gltf_model;
            let gltf_data = &mut gltf_model.gltf_data[i];
            if gltf_data.morph_targets.len() <= 0 {
                return;
            };
            // reset
            let rrdata = &mut self.data.model_descriptor_set.rrdata[i];
            let vertices = &mut rrdata.vertex_data.vertices;
            for i in 0..vertices.len() {
                vertices[i].pos = Vec3::new_array(gltf_data.vertices[i].position);
            }

            let morph_animation = &gltf_model.morph_animations[animation_index];
            for i in 0..morph_animation.weights.len() {
                let morph_target = &gltf_data.morph_targets[i];
                for j in 0..morph_target.positions.len() {
                    let delta_position = Vec3::new_array(morph_target.positions[j])
                        * morph_animation.weights[i]
                        * 0.01f32;
                    vertices[j].pos += delta_position;
                }
            }

            if let Err(e) = rrdata.vertex_buffer.update(
                &self.instance,
                &self.rrdevice,
                &self.data.rrcommand_pool,
                (size_of::<data::Vertex>() * vertices.len()) as vk::DeviceSize,
                vertices.as_ptr() as *const c_void,
                vertices.len(),
            ) {
                eprintln!("failed to update vertex buffer: {}", e);
            }
        }
    }

    /// Initialize Ray Tracing resources (G-Buffer, Acceleration Structure, Pipelines)
    unsafe fn init_ray_tracing(
        instance: &Instance,
        rrdevice: &RRDevice,
        data: &mut AppData,
    ) -> Result<()> {
        log::info!("Initializing Ray Tracing resources...");

        // 1. Create G-Buffer
        let gbuffer = RRGBuffer::new(
            instance,
            rrdevice,
            data.rrswapchain.swapchain_extent.width,
            data.rrswapchain.swapchain_extent.height,
        )?;

        // Transition G-Buffer images to proper layouts
        gbuffer.transition_layouts(rrdevice, data.rrcommand_pool.command_pool)?;

        data.gbuffer = Some(gbuffer);
        log::info!("Created G-Buffer");

        // 2. Create G-Buffer render pass and framebuffer
        create_gbuffer_render_pass(instance, rrdevice, &mut data.rrrender)?;

        if let Some(ref gbuffer) = data.gbuffer {
            create_gbuffer_framebuffer(instance, rrdevice, &mut data.rrrender, gbuffer)?;
        }
        log::info!("Created G-Buffer render pass and framebuffer");

        // 3. Create G-Buffer pipeline and descriptor set
        data.gbuffer_descriptor_set = Some(RRDescriptorSet::new(rrdevice, &data.rrswapchain));

        // Note: G-Buffer pipeline will be created after model is loaded
        // because it needs to match the vertex format

        log::info!("Ray Tracing initialization complete");
        Ok(())
    }

    /// Build acceleration structures from loaded model
    unsafe fn build_acceleration_structures(
        instance: &Instance,
        rrdevice: &RRDevice,
        data: &mut AppData,
    ) -> Result<()> {
        log::info!("Building acceleration structures...");

        let mut acceleration_structure = RRAccelerationStructure::new();

        // Build BLAS for each mesh in the model
        for rrdata in &data.model_descriptor_set.rrdata {
            let blas = RRAccelerationStructure::create_blas(
                instance,
                rrdevice,
                &data.rrcommand_pool,
                &rrdata.vertex_buffer.buffer,
                rrdata.vertex_data.vertices.len() as u32,
                std::mem::size_of::<data::Vertex>() as u32,
                &rrdata.index_buffer.buffer,
                rrdata.vertex_data.indices.len() as u32,
            )?;

            acceleration_structure.blas_list.push(blas);
            log::info!("Created BLAS for mesh");
        }

        // Build TLAS from all BLAS
        if !acceleration_structure.blas_list.is_empty() {
            let tlas = RRAccelerationStructure::create_tlas(
                instance,
                rrdevice,
                &data.rrcommand_pool,
                &acceleration_structure.blas_list,
            )?;
            acceleration_structure.tlas = tlas;
            log::info!("Created TLAS with {} instances", acceleration_structure.blas_list.len());
        }

        data.acceleration_structure = Some(acceleration_structure);
        log::info!("Acceleration structures built successfully");
        Ok(())
    }

    /// Create Ray Tracing pipelines after AS is built
    unsafe fn create_ray_tracing_pipelines(
        instance: &Instance,
        rrdevice: &RRDevice,
        data: &mut AppData,
    ) -> Result<()> {
        log::info!("Creating Ray Tracing pipelines...");

        // 1. Create G-Buffer pipeline
        if let Some(ref mut gbuffer_desc) = data.gbuffer_descriptor_set {
            // Copy model data for G-Buffer rendering
            for rrdata in &data.model_descriptor_set.rrdata {
                gbuffer_desc.rrdata.push(rrdata.clone());
            }

            // Create descriptor sets
            RRDescriptorSet::create_descriptor_set(rrdevice, &data.rrswapchain, gbuffer_desc)?;

            // Create G-Buffer pipeline with MRT
            let gbuffer_pipeline = PipelineBuilder::new(
                "src/shaders/gbufferVert.spv",
                "src/shaders/gbufferFrag.spv",
            )
            .vertex_input(VertexInputConfig::Standard)
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .polygon_mode(vk::PolygonMode::FILL)
            .custom_render_pass(data.rrrender.gbuffer_render_pass)
            .mrt_attachments(2) // position and normal
            .descriptor_layouts(vec![gbuffer_desc.descriptor_set_layout])
            .build(rrdevice, &data.rrrender, Some(data.rrswapchain.swapchain_extent))?;

            data.gbuffer_pipeline = Some(gbuffer_pipeline);
            log::info!("Created G-Buffer pipeline");
        }

        // 2. Create scene uniform buffer
        let (scene_buffer, scene_memory) = create_buffer(
            instance,
            rrdevice,
            std::mem::size_of::<SceneUniformData>() as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        data.scene_uniform_buffer = Some(scene_buffer);
        data.scene_uniform_buffer_memory = Some(scene_memory);

        // 3. Create Ray Query descriptor set and pipeline
        let mut ray_query_descriptor = RRRayQueryDescriptorSet {
            descriptor_set_layout: RRRayQueryDescriptorSet::create_layout(rrdevice)?,
            descriptor_pool: RRRayQueryDescriptorSet::create_pool(rrdevice)?,
            descriptor_set: vk::DescriptorSet::null(),
        };

        // Allocate and update descriptor set with G-Buffer images, TLAS, and scene uniform buffer
        if let (Some(ref gbuffer), Some(ref accel_struct)) = (&data.gbuffer, &data.acceleration_structure) {
            if let Some(tlas) = accel_struct.tlas.acceleration_structure {
                ray_query_descriptor.allocate_and_update(
                    rrdevice,
                    gbuffer.position_image_view,
                    gbuffer.normal_image_view,
                    gbuffer.shadow_mask_image_view,
                    tlas,
                    scene_buffer,
                )?;
            }
        }

        let ray_query_pipeline = RRPipeline::new_compute(
            rrdevice,
            "src/shaders/rayQueryShadow.spv",
            &[ray_query_descriptor.descriptor_set_layout],
        )?;
        data.ray_query_pipeline = Some(ray_query_pipeline);
        data.ray_query_descriptor = Some(ray_query_descriptor);
        log::info!("Created Ray Query descriptor set and pipeline");

        // 4. Create G-Buffer sampler
        let gbuffer_sampler = create_texture_sampler(rrdevice, 1)?;
        data.gbuffer_sampler = Some(gbuffer_sampler);

        // 5. Create composite descriptor set and pipeline
        let mut composite_descriptor = RRCompositeDescriptorSet {
            descriptor_set_layout: RRCompositeDescriptorSet::create_layout(rrdevice)?,
            descriptor_pool: RRCompositeDescriptorSet::create_pool(rrdevice)?,
            descriptor_set: vk::DescriptorSet::null(),
        };

        // Allocate and update descriptor set with G-Buffer images and scene uniform buffer
        if let Some(ref gbuffer) = data.gbuffer {
            composite_descriptor.allocate_and_update(
                rrdevice,
                gbuffer.position_image_view,
                gbuffer_sampler,
                gbuffer.normal_image_view,
                gbuffer_sampler,
                gbuffer.shadow_mask_image_view,
                gbuffer_sampler,
                scene_buffer,
            )?;
        }

        let composite_pipeline = PipelineBuilder::new(
            "src/shaders/compositeVert.spv",
            "src/shaders/compositeFrag.spv",
        )
        .vertex_input(VertexInputConfig::Custom {
            bindings: vec![],
            attributes: vec![],
        })
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .polygon_mode(vk::PolygonMode::FILL)
        .no_depth_test()
        .descriptor_layouts(vec![composite_descriptor.descriptor_set_layout])
        .build(rrdevice, &data.rrrender, Some(data.rrswapchain.swapchain_extent))?;

        data.composite_pipeline = Some(composite_pipeline);
        data.composite_descriptor = Some(composite_descriptor);
        log::info!("Created composite descriptor set and pipeline");

        log::info!("Ray Tracing pipelines created successfully");
        Ok(())
    }

    unsafe fn reload_model_data_buffer(
        instance: &Instance,
        rrdevice: &RRDevice,
        data: &mut AppData,
    ) -> Result<()> {
        if let Err(e) = Self::load_model(&instance, &rrdevice, data) {
            eprintln!("{:?}", e);
            log!("{:?}", e)
        }
        println!("reloaded model");

        for i in 0..data.model_descriptor_set.rrdata.len() {
            let rrdata = &mut data.model_descriptor_set.rrdata[i];
            rrdata.delete(rrdevice);

            rrdata.vertex_buffer = RRVertexBuffer::new(
                &instance,
                &rrdevice,
                &data.rrcommand_pool,
                (size_of::<data::Vertex>() * rrdata.vertex_data.vertices.len())
                    as vk::DeviceSize,
                rrdata.vertex_data.vertices.as_ptr() as *const c_void,
                rrdata.vertex_data.vertices.len(),
            );

            rrdata.index_buffer = RRIndexBuffer::new(
                &instance,
                &rrdevice,
                &data.rrcommand_pool,
                (size_of::<u32>() * rrdata.vertex_data.indices.len()) as u64,
                rrdata.vertex_data.indices.as_ptr() as *const c_void,
                rrdata.vertex_data.indices.len(),
            );

            RRData::create_uniform_buffers(rrdata, &instance, &rrdevice, &data.rrswapchain);

            rrdata.image_view = create_image_view(
                &rrdevice,
                rrdata.image,
                vk::Format::R8G8B8A8_SRGB,
                vk::ImageAspectFlags::COLOR,
                rrdata.mip_level,
            )?;

            rrdata.sampler = create_texture_sampler(&rrdevice, rrdata.mip_level)?;
        }

        // Build acceleration structures after model is loaded
        if let Err(e) = Self::build_acceleration_structures(instance, rrdevice, data) {
            eprintln!("Failed to build acceleration structures: {:?}", e);
            log!("Failed to build acceleration structures: {:?}", e);
        }

        // Create Ray Tracing pipelines after AS is built
        if let Err(e) = Self::create_ray_tracing_pipelines(instance, rrdevice, data) {
            eprintln!("Failed to create ray tracing pipelines: {:?}", e);
            log!("Failed to create ray tracing pipelines: {:?}", e);
        }

        Ok(())
    }

    /// Load a model from the specified file path
    unsafe fn load_model_from_path(
        instance: &Instance,
        rrdevice: &RRDevice,
        data: &mut AppData,
        model_path: &str,
    ) -> Result<()> {
        log!("=== Loading model from path: {} ===", model_path);

        // Determine file type by extension
        let path_lower = model_path.to_lowercase();
        let is_fbx = path_lower.ends_with(".fbx");
        let is_gltf = path_lower.ends_with(".gltf") || path_lower.ends_with(".glb");

        if !is_fbx && !is_gltf {
            return Err(anyhow!("Unsupported file format. Only FBX and glTF/GLB are supported."));
        }

        // Clean up existing model data (this will free descriptor sets and reuse the pool)
        log!("Cleaning up existing model data...");
        data.model_descriptor_set.delete_data(rrdevice);
        data.model_descriptor_set.rrdata.clear();
        log!("Cleared existing data, descriptor pool will be reused");

        // Load the model based on file type
        if is_fbx {
            log!("Loading FBX model...");

            // Clear glTF model data when loading FBX
            data.gltf_model = GltfModel::default();
            log!("Cleared glTF model data");

            // Use fbxcel for stickman_bin.fbx (russimp doesn't read its animation correctly)
            // Use russimp for other FBX files (better compatibility)
            if model_path.contains("stickman_bin.fbx") {
                log!("Using fbxcel loader for stickman_bin.fbx");
                unsafe {
                    data.fbx_model = load_fbx(model_path)?;
                }
            } else {
                log!("Using russimp loader");
                data.fbx_model = load_fbx_with_russimp(model_path)?;
            }

            // Create separate rrdata for each FBX mesh
            for (mesh_idx, fbx_data) in data.fbx_model.fbx_data.iter().enumerate() {
                log!("Creating RRData for FBX mesh {}: {} vertices, texture: {:?}",
                    mesh_idx, fbx_data.positions.len(), fbx_data.diffuse_texture);

                let mut rrdata = RRData::new(&instance, &rrdevice, &data.rrswapchain);

                // Load texture for this mesh
                if let Some(texture_path) = &fbx_data.diffuse_texture {
                    log!("Loading texture: {}", texture_path);
                    match load_png_image(texture_path) {
                        Ok((image_data, width, height)) => {
                            match create_texture_image_pixel(
                                instance,
                                rrdevice,
                                data.rrcommand_pool.borrow_mut(),
                                &image_data,
                                width,
                                height,
                            ) {
                                Ok((image, image_memory, mip_level)) => {
                                    rrdata.image = image;
                                    rrdata.image_memory = image_memory;
                                    rrdata.mip_level = mip_level;
                                    log!("Texture loaded successfully for mesh {}", mesh_idx);
                                }
                                Err(e) => {
                                    log!("Failed to create texture image for mesh {}: {}", mesh_idx, e);
                                    // Fall back to white texture
                                    (rrdata.image, rrdata.image_memory, rrdata.mip_level) = create_texture_image_pixel(
                                        instance,
                                        rrdevice,
                                        data.rrcommand_pool.borrow_mut(),
                                        &vec![255u8, 255, 255, 255],
                                        1,
                                        1,
                                    )?;
                                }
                            }
                        }
                        Err(e) => {
                            log!("Failed to load texture file {}: {}", texture_path, e);
                            // Fall back to white texture
                            (rrdata.image, rrdata.image_memory, rrdata.mip_level) = create_texture_image_pixel(
                                instance,
                                rrdevice,
                                data.rrcommand_pool.borrow_mut(),
                                &vec![255u8, 255, 255, 255],
                                1,
                                1,
                            )?;
                        }
                    }
                } else {
                    log!("No texture specified for mesh {}, using white", mesh_idx);
                    // Use white texture as fallback
                    (rrdata.image, rrdata.image_memory, rrdata.mip_level) = create_texture_image_pixel(
                        instance,
                        rrdevice,
                        data.rrcommand_pool.borrow_mut(),
                        &vec![255u8, 255, 255, 255],
                        1,
                        1,
                    )?;
                }

                // Create vertex data
                rrdata.vertex_data = VertexData::default();
                for (i, position) in fbx_data.positions.iter().enumerate() {
                    // Get UV coordinates (or use default if index out of bounds)
                    let uv = if i < fbx_data.tex_coords.len() {
                        fbx_data.tex_coords[i]
                    } else {
                        [0.5, 0.5]
                    };

                    let vertex = data::Vertex::new(
                        Vec3::new(position.x, position.y, position.z),
                        Vec4::new(1.0, 1.0, 1.0, 1.0),  // White color for proper texturing
                        Vec2::new_array(uv),             // Use actual UV coordinates
                    );
                    rrdata.vertex_data.vertices.push(vertex);
                }

                // Set indices
                rrdata.vertex_data.indices = fbx_data.indices.clone();

                data.model_descriptor_set.rrdata.push(rrdata);
            }

            // Initialize animation if available
            if data.fbx_model.animation_count() > 0 {
                data.animation_playing = true;
                data.current_animation_index = 0;
                data.animation_time = 0.0;
                log!("FBX animation loaded: {} animations", data.fbx_model.animation_count());
            }

        } else if is_gltf {
            log!("Loading glTF model...");

            // Clear FBX model data and animation state when loading glTF
            data.fbx_model = FbxModel::default();
            data.animation_playing = false;
            data.current_animation_index = 0;
            data.animation_time = 0.0;
            log!("Cleared FBX model data and animation state");

            data.gltf_model = GltfModel::load_model(model_path);

            for (i, gltf_data) in data.gltf_model.gltf_data.iter().enumerate() {
                log!("Creating RRData for glTF mesh {}: {} vertices", i, gltf_data.vertices.len());

                let mut rrdata = RRData::new(&instance, &rrdevice, &data.rrswapchain);

                // Load texture from image_data
                if !gltf_data.image_data.is_empty() {
                    log!("Loading texture from glTF image data for mesh {}", i);
                    match create_texture_image_pixel(
                        instance,
                        rrdevice,
                        data.rrcommand_pool.borrow_mut(),
                        &gltf_data.image_data[0].data,
                        gltf_data.image_data[0].width,
                        gltf_data.image_data[0].height,
                    ) {
                        Ok((image, image_memory, mip_level)) => {
                            rrdata.image = image;
                            rrdata.image_memory = image_memory;
                            rrdata.mip_level = mip_level;
                            log!("Texture loaded successfully for mesh {}", i);
                        }
                        Err(e) => {
                            log!("Failed to create texture image for mesh {}: {}", i, e);
                            // Fallback to white texture
                            (rrdata.image, rrdata.image_memory, rrdata.mip_level) = create_texture_image_pixel(
                                instance,
                                rrdevice,
                                data.rrcommand_pool.borrow_mut(),
                                &vec![255u8, 255, 255, 255],
                                1,
                                1,
                            )?;
                        }
                    }
                } else {
                    log!("No texture data for mesh {}, using white", i);
                    // Use white texture as fallback
                    (rrdata.image, rrdata.image_memory, rrdata.mip_level) = create_texture_image_pixel(
                        instance,
                        rrdevice,
                        data.rrcommand_pool.borrow_mut(),
                        &vec![255u8, 255, 255, 255],
                        1,
                        1,
                    )?;
                }

                // Create vertex data
                rrdata.vertex_data = VertexData::default();
                for gltf_vertex in &gltf_data.vertices {
                    rrdata
                        .vertex_data
                        .vertices
                        .push(data::Vertex::default());
                }

                for gltf_vertex in &gltf_data.vertices {
                    let vertex = data::Vertex::new(
                        Vec3::new_array(gltf_vertex.position),
                        Vec4::new(0.0, 1.0, 0.0, 1.0),
                        Vec2::new_array(gltf_vertex.tex_coord),
                    );
                    rrdata.vertex_data.vertices[gltf_vertex.index] = vertex;
                }

                rrdata.vertex_data.indices = gltf_data.indices.clone();

                data.model_descriptor_set.rrdata.push(rrdata);
            }
        }

        // Recreate buffers and descriptor sets
        log!("Recreating buffers and descriptor sets...");
        for i in 0..data.model_descriptor_set.rrdata.len() {
            let rrdata = &mut data.model_descriptor_set.rrdata[i];

            rrdata.vertex_buffer = RRVertexBuffer::new(
                &instance,
                &rrdevice,
                &data.rrcommand_pool,
                (size_of::<data::Vertex>() * rrdata.vertex_data.vertices.len())
                    as vk::DeviceSize,
                rrdata.vertex_data.vertices.as_ptr() as *const c_void,
                rrdata.vertex_data.vertices.len(),
            );

            rrdata.index_buffer = RRIndexBuffer::new(
                &instance,
                &rrdevice,
                &data.rrcommand_pool,
                (size_of::<u32>() * rrdata.vertex_data.indices.len()) as u64,
                rrdata.vertex_data.indices.as_ptr() as *const c_void,
                rrdata.vertex_data.indices.len(),
            );

            RRData::create_uniform_buffers(rrdata, &instance, &rrdevice, &data.rrswapchain);

            rrdata.image_view = create_image_view(
                &rrdevice,
                rrdata.image,
                vk::Format::R8G8B8A8_SRGB,
                vk::ImageAspectFlags::COLOR,
                rrdata.mip_level,
            )?;

            rrdata.sampler = create_texture_sampler(&rrdevice, rrdata.mip_level)?;
        }

        // Apply initial pose for glTF models with animation
        if is_gltf && (!data.gltf_model.joint_animations.is_empty() || !data.gltf_model.node_animations.is_empty()) {
            if data.gltf_model.has_skinned_meshes {
                log!("Applying initial pose (time=0) for glTF skeletal animation...");
                data.gltf_model.reset_vertices_animation_position(0.0);
                data.gltf_model.apply_animation(0.0, 0, Matrix4::identity());
                log!("Initial pose applied successfully for glTF");
            } else {
                log!("Applying initial pose (time=0) for glTF node animation...");
                data.gltf_model.reset_vertices_animation_position(0.0);
                log!("Initial pose applied successfully for glTF");
            }

            // Update vertex buffers with initial pose
            for i in 0..data.gltf_model.gltf_data.len() {
                if i >= data.model_descriptor_set.rrdata.len() {
                    break;
                }

                let rrdata = &mut data.model_descriptor_set.rrdata[i];
                let vertex_data = &mut rrdata.vertex_data;
                let gltf_data = &data.gltf_model.gltf_data[i];

                for vertex in &gltf_data.vertices {
                    vertex_data.vertices[vertex.index].pos.x = vertex.animation_position[0];
                    vertex_data.vertices[vertex.index].pos.y = vertex.animation_position[1];
                    vertex_data.vertices[vertex.index].pos.z = vertex.animation_position[2];
                }

                // Update vertex buffer with initial pose
                if let Err(e) = rrdata.vertex_buffer.update(
                    &instance,
                    &rrdevice,
                    &data.rrcommand_pool,
                    (size_of::<data::Vertex>() * vertex_data.vertices.len()) as vk::DeviceSize,
                    vertex_data.vertices.as_ptr() as *const c_void,
                    vertex_data.vertices.len(),
                ) {
                    log!("Failed to update vertex buffer for glTF mesh {} with initial pose: {}", i, e);
                }
            }
            log!("Initial pose applied successfully for glTF");
        }

        // Apply initial pose for FBX models with skeletal animation
        if is_fbx && data.fbx_model.animation_count() > 0 {
            log!("Applying initial pose (time=0) for FBX skeletal animation...");
            data.fbx_model.update_animation(0, 0.0);

            // Update vertex buffers with initial pose
            for (mesh_idx, fbx_data) in data.fbx_model.fbx_data.iter().enumerate() {
                if let Some(rrdata) = data.model_descriptor_set.rrdata.get_mut(mesh_idx) {
                    let vertex_data = &mut rrdata.vertex_data;

                    // Update vertex positions from fbx_data (after animation applied)
                    for (vertex_idx, pos) in fbx_data.positions.iter().enumerate() {
                        if vertex_idx < vertex_data.vertices.len() {
                            vertex_data.vertices[vertex_idx].pos.x = pos.x;
                            vertex_data.vertices[vertex_idx].pos.y = pos.y;
                            vertex_data.vertices[vertex_idx].pos.z = pos.z;
                        }
                    }

                    // Update vertex buffer with initial pose
                    if let Err(e) = rrdata.vertex_buffer.update(
                        &instance,
                        &rrdevice,
                        &data.rrcommand_pool,
                        (size_of::<data::Vertex>() * vertex_data.vertices.len()) as vk::DeviceSize,
                        vertex_data.vertices.as_ptr() as *const c_void,
                        vertex_data.vertices.len(),
                    ) {
                        log!("Failed to update vertex buffer for mesh {} with initial pose: {}", mesh_idx, e);
                    }
                }
            }
            log!("Initial pose applied successfully for FBX");
        }

        // Recreate descriptor sets
        log!("Creating descriptor sets...");
        if let Err(e) = RRDescriptorSet::create_descriptor_set(
            &rrdevice,
            &data.rrswapchain,
            &mut data.model_descriptor_set,
        ) {
            log!("Failed to create model descriptor set: {:?}", e);
            return Err(anyhow!("Failed to create descriptor sets: {:?}", e));
        }

        // Recreate command buffers
        log!("Recreating command buffers...");
        let mut rrbind_info = Vec::new();
        rrbind_info.push(RRBindInfo::new(
            &data.grid_pipeline,
            &data.grid_descriptor_set,
            &data.grid_vertex_buffer,
            &data.grid_index_buffer,
            0,
            0,
            0,
        ));

        for i in 0..data.model_descriptor_set.rrdata.len() {
            rrbind_info.push(RRBindInfo::new(
                &data.model_pipeline,
                &data.model_descriptor_set,
                &data.model_descriptor_set.rrdata[i].vertex_buffer,
                &data.model_descriptor_set.rrdata[i].index_buffer,
                0,
                0,
                i,
            ));
        }

        for i in 0..data.rrrender.framebuffers.len() {
            if let Err(e) = RRCommandBuffer::bind_command(
                &rrdevice,
                &data.rrrender,
                &data.rrswapchain,
                &rrbind_info,
                &mut data.rrcommand_buffer,
                i,
            ) {
                log!("Failed to bind command for framebuffer {}: {:?}", i, e);
                return Err(anyhow!("Failed to bind command: {:?}", e));
            }
        }

        // Update current model path
        data.current_model_path = model_path.to_string();

        log!("=== Model loaded successfully ===");
        Ok(())
    }

    /// Initialize ImGui rendering resources
    unsafe fn init_imgui_rendering(
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
            "src/shaders/imguiVert.spv",
            "src/shaders/imguiFrag.spv",
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

    /// Update ImGui vertex and index buffers
    unsafe fn update_imgui_buffers(
        instance: &Instance,
        rrdevice: &RRDevice,
        data: &mut AppData,
        draw_data: &imgui::DrawData,
    ) -> Result<()> {
        if draw_data.total_vtx_count == 0 || draw_data.total_idx_count == 0 {
            return Ok(());
        }

        // Calculate required buffer sizes
        let vtx_buffer_size = (draw_data.total_vtx_count as usize * std::mem::size_of::<imgui::DrawVert>()) as vk::DeviceSize;
        let idx_buffer_size = (draw_data.total_idx_count as usize * std::mem::size_of::<imgui::DrawIdx>()) as vk::DeviceSize;

        // Create or resize vertex buffer if needed
        if data.imgui_vertex_buffer.is_none() || vtx_buffer_size > data.imgui_vertex_buffer_size {
            // Destroy old buffer if exists
            if let Some(buffer) = data.imgui_vertex_buffer {
                rrdevice.device.destroy_buffer(buffer, None);
            }
            if let Some(memory) = data.imgui_vertex_buffer_memory {
                rrdevice.device.free_memory(memory, None);
            }

            // Create new vertex buffer
            let buffer_info = vk::BufferCreateInfo::builder()
                .size(vtx_buffer_size)
                .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);

            let vertex_buffer = rrdevice.device.create_buffer(&buffer_info, None)?;
            let mem_requirements = rrdevice.device.get_buffer_memory_requirements(vertex_buffer);

            let mem_alloc_info = vk::MemoryAllocateInfo::builder()
                .allocation_size(mem_requirements.size)
                .memory_type_index(get_memory_type_index(
                    instance,
                    rrdevice.physical_device,
                    vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                    mem_requirements,
                )?);

            let vertex_buffer_memory = rrdevice.device.allocate_memory(&mem_alloc_info, None)?;
            rrdevice.device.bind_buffer_memory(vertex_buffer, vertex_buffer_memory, 0)?;

            data.imgui_vertex_buffer = Some(vertex_buffer);
            data.imgui_vertex_buffer_memory = Some(vertex_buffer_memory);
            data.imgui_vertex_buffer_size = vtx_buffer_size;
        }

        // Create or resize index buffer if needed
        if data.imgui_index_buffer.is_none() || idx_buffer_size > data.imgui_index_buffer_size {
            // Destroy old buffer if exists
            if let Some(buffer) = data.imgui_index_buffer {
                rrdevice.device.destroy_buffer(buffer, None);
            }
            if let Some(memory) = data.imgui_index_buffer_memory {
                rrdevice.device.free_memory(memory, None);
            }

            // Create new index buffer
            let buffer_info = vk::BufferCreateInfo::builder()
                .size(idx_buffer_size)
                .usage(vk::BufferUsageFlags::INDEX_BUFFER)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);

            let index_buffer = rrdevice.device.create_buffer(&buffer_info, None)?;
            let mem_requirements = rrdevice.device.get_buffer_memory_requirements(index_buffer);

            let mem_alloc_info = vk::MemoryAllocateInfo::builder()
                .allocation_size(mem_requirements.size)
                .memory_type_index(get_memory_type_index(
                    instance,
                    rrdevice.physical_device,
                    vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                    mem_requirements,
                )?);

            let index_buffer_memory = rrdevice.device.allocate_memory(&mem_alloc_info, None)?;
            rrdevice.device.bind_buffer_memory(index_buffer, index_buffer_memory, 0)?;

            data.imgui_index_buffer = Some(index_buffer);
            data.imgui_index_buffer_memory = Some(index_buffer_memory);
            data.imgui_index_buffer_size = idx_buffer_size;
        }

        // Upload vertex data
        if let Some(vertex_buffer_memory) = data.imgui_vertex_buffer_memory {
            let ptr = rrdevice.device.map_memory(
                vertex_buffer_memory,
                0,
                vtx_buffer_size,
                vk::MemoryMapFlags::empty(),
            )?;

            let mut offset = 0;
            for draw_list in draw_data.draw_lists() {
                let vtx_buffer = draw_list.vtx_buffer();
                let vtx_size = (vtx_buffer.len() * std::mem::size_of::<imgui::DrawVert>()) as usize;
                std::ptr::copy_nonoverlapping(
                    vtx_buffer.as_ptr() as *const u8,
                    (ptr as *mut u8).add(offset),
                    vtx_size,
                );
                offset += vtx_size;
            }

            rrdevice.device.unmap_memory(vertex_buffer_memory);
        }

        // Upload index data
        if let Some(index_buffer_memory) = data.imgui_index_buffer_memory {
            let ptr = rrdevice.device.map_memory(
                index_buffer_memory,
                0,
                idx_buffer_size,
                vk::MemoryMapFlags::empty(),
            )?;

            let mut offset = 0;
            for draw_list in draw_data.draw_lists() {
                let idx_buffer = draw_list.idx_buffer();
                let idx_size = (idx_buffer.len() * std::mem::size_of::<imgui::DrawIdx>()) as usize;
                std::ptr::copy_nonoverlapping(
                    idx_buffer.as_ptr() as *const u8,
                    (ptr as *mut u8).add(offset),
                    idx_size,
                );
                offset += idx_size;
            }

            rrdevice.device.unmap_memory(index_buffer_memory);
        }

        Ok(())
    }

    /// Record command buffer with 3D rendering and ImGui
    unsafe fn record_command_buffer(
        &self,
        image_index: usize,
        draw_data: &imgui::DrawData,
    ) -> Result<()> {
        let command_buffer = self.data.rrcommand_buffer.command_buffers[image_index];

        // Reset command buffer
        self.rrdevice.device.reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())?;

        // Begin command buffer
        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::empty());
        self.rrdevice.device.begin_command_buffer(command_buffer, &begin_info)?;

        // Check if Ray Tracing is enabled
        let use_ray_tracing = self.data.gbuffer.is_some()
            && self.data.ray_query_pipeline.is_some()
            && self.data.composite_pipeline.is_some();

        if use_ray_tracing {
            // === Ray Tracing Path (3-pass rendering) ===

            // Pass 1: Render to G-Buffer
            self.record_gbuffer_pass(command_buffer, image_index)?;

            // Pass 2: Ray Query compute shader (shadow calculation)
            self.record_ray_query_pass(command_buffer)?;

            // Pass 3: Composite pass (final image) + ImGui
            self.record_composite_pass(command_buffer, image_index, draw_data)?;
        } else {
            // === Traditional Forward Rendering Path ===

            // Begin render pass
            let render_area = vk::Rect2D::builder()
                .offset(vk::Offset2D::default())
                .extent(self.data.rrswapchain.swapchain_extent);

            let color_clear_value = vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            };
            let depth_clear_value = vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            };
            let clear_values = [color_clear_value, depth_clear_value];

            let render_pass_info = vk::RenderPassBeginInfo::builder()
                .render_pass(self.data.rrrender.render_pass)
                .framebuffer(self.data.rrrender.framebuffers[image_index])
                .render_area(render_area)
                .clear_values(&clear_values);

            self.rrdevice.device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_info,
                vk::SubpassContents::INLINE,
            );

            // Draw 3D models (existing rendering logic)
            self.record_3d_rendering(command_buffer, image_index)?;

            // Draw ImGui
            self.record_imgui_rendering(command_buffer, draw_data)?;

            // End render pass
            self.rrdevice.device.cmd_end_render_pass(command_buffer);
        }

        // End command buffer
        self.rrdevice.device.end_command_buffer(command_buffer)?;

        Ok(())
    }

    /// Record 3D model rendering commands
    unsafe fn record_3d_rendering(
        &self,
        command_buffer: vk::CommandBuffer,
        image_index: usize,
    ) -> Result<()> {
        // This is the existing rendering logic from bind_command
        let mut rrbind_info = Vec::new();

        // Grid pipeline bindings
        rrbind_info.push(RRBindInfo::new(
            &self.data.grid_pipeline,
            &self.data.grid_descriptor_set,
            &self.data.grid_vertex_buffer,
            &self.data.grid_index_buffer,
            0,
            0,
            0,
        ));

        // Model pipeline bindings
        for i in 0..self.data.model_descriptor_set.rrdata.len() {
            rrbind_info.push(RRBindInfo::new(
                &self.data.model_pipeline,
                &self.data.model_descriptor_set,
                &self.data.model_descriptor_set.rrdata[i].vertex_buffer,
                &self.data.model_descriptor_set.rrdata[i].index_buffer,
                0,
                0,
                i,
            ));
        }

        // Execute all bind commands
        for bind_info in &rrbind_info {
            self.rrdevice.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                bind_info.rrpipeline.pipeline,
            );

            // すべてのパイプラインで線幅を設定（RRPipeline::new()はすべてLINE_WIDTHをdynamic stateに含む）
            // パイプラインバインド直後に設定（Vulkanのベストプラクティス）
            self.rrdevice.device.cmd_set_line_width(command_buffer, 1.0);

            self.rrdevice.device.cmd_bind_vertex_buffers(
                command_buffer,
                0,
                &[bind_info.rrvertex_buffer.buffer],
                &[0],
            );

            self.rrdevice.device.cmd_bind_index_buffer(
                command_buffer,
                bind_info.rrindex_buffer.buffer,
                0,
                vk::IndexType::UINT32,
            );

            let swapchain_images_len = bind_info.rrdescriptor_set.descriptor_sets.len() /
                bind_info.rrdescriptor_set.rrdata.len().max(1);
            let descriptor_set_index = bind_info.data_index * swapchain_images_len + image_index;

            self.rrdevice.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                bind_info.rrpipeline.pipeline_layout,
                0,
                &[bind_info.rrdescriptor_set.descriptor_sets[descriptor_set_index]],
                &[],
            );

            self.rrdevice.device.cmd_draw_indexed(
                command_buffer,
                bind_info.rrindex_buffer.indices,
                1,
                bind_info.offset_index,
                bind_info.offset_index as i32,
                0,
            );
        }

        // Gizmoの描画（常に最後に描画して、他のオブジェクトの上に表示）
        if let (Some(vertex_buffer), Some(index_buffer)) =
            (self.data.gizmo_data.vertex_buffer, self.data.gizmo_data.index_buffer) {

            // Gizmoパイプラインをバインド
            self.rrdevice.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.data.gizmo_pipeline.pipeline,
            );

            // 線幅を設定（wideLinesが無効なので1.0のみ使用可能）- パイプラインバインド直後に設定
            self.rrdevice.device.cmd_set_line_width(command_buffer, 1.0);

            // 頂点バッファをバインド
            self.rrdevice.device.cmd_bind_vertex_buffers(
                command_buffer,
                0,
                &[vertex_buffer],
                &[0],
            );

            // インデックスバッファをバインド
            self.rrdevice.device.cmd_bind_index_buffer(
                command_buffer,
                index_buffer,
                0,
                vk::IndexType::UINT32,
            );

            // ディスクリプタセットをバインド
            // Gizmoは常にdata_index=0（1つのRRDataのみ）
            let swapchain_images_len = self.data.gizmo_descriptor_set.descriptor_sets.len() /
                self.data.gizmo_descriptor_set.rrdata.len().max(1);
            let descriptor_set_index = 0 * swapchain_images_len + image_index;

            self.rrdevice.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.data.gizmo_pipeline.pipeline_layout,
                0,
                &[self.data.gizmo_descriptor_set.descriptor_sets[descriptor_set_index]],
                &[],
            );

            // Gizmoを描画
            self.rrdevice.device.cmd_draw_indexed(
                command_buffer,
                self.data.gizmo_data.indices.len() as u32,
                1,
                0,
                0,
                0,
            );
        }

        Ok(())
    }

    /// Record ImGui rendering commands
    unsafe fn record_imgui_rendering(
        &self,
        command_buffer: vk::CommandBuffer,
        draw_data: &imgui::DrawData,
    ) -> Result<()> {
        if draw_data.total_vtx_count == 0 || draw_data.total_idx_count == 0 {
            return Ok(());
        }

        let pipeline = self.data.imgui_pipeline.ok_or_else(|| anyhow!("ImGui pipeline not initialized"))?;
        let pipeline_layout = self.data.imgui_pipeline_layout.ok_or_else(|| anyhow!("ImGui pipeline layout not initialized"))?;
        let descriptor_set = self.data.imgui_descriptor_set.ok_or_else(|| anyhow!("ImGui descriptor set not initialized"))?;
        let vertex_buffer = self.data.imgui_vertex_buffer.ok_or_else(|| anyhow!("ImGui vertex buffer not initialized"))?;
        let index_buffer = self.data.imgui_index_buffer.ok_or_else(|| anyhow!("ImGui index buffer not initialized"))?;

        // Bind pipeline
        self.rrdevice.device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            pipeline,
        );

        // Bind descriptor sets
        self.rrdevice.device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            pipeline_layout,
            0,
            &[descriptor_set],
            &[],
        );

        // Bind vertex and index buffers
        self.rrdevice.device.cmd_bind_vertex_buffers(command_buffer, 0, &[vertex_buffer], &[0]);
        self.rrdevice.device.cmd_bind_index_buffer(command_buffer, index_buffer, 0, vk::IndexType::UINT16);

        // Setup viewport and scissor
        let fb_width = draw_data.display_size[0] * draw_data.framebuffer_scale[0];
        let fb_height = draw_data.display_size[1] * draw_data.framebuffer_scale[1];

        let viewport = vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(fb_width)
            .height(fb_height)
            .min_depth(0.0)
            .max_depth(1.0);
        self.rrdevice.device.cmd_set_viewport(command_buffer, 0, &[viewport]);

        // Setup scale and translation for ImGui coordinates -> NDC
        let scale = [
            2.0 / draw_data.display_size[0],
            2.0 / draw_data.display_size[1],
        ];
        let translate = [
            -1.0 - draw_data.display_pos[0] * scale[0],
            -1.0 - draw_data.display_pos[1] * scale[1],
        ];
        let push_constants = [scale[0], scale[1], translate[0], translate[1]];

        self.rrdevice.device.cmd_push_constants(
            command_buffer,
            pipeline_layout,
            vk::ShaderStageFlags::VERTEX,
            0,
            std::slice::from_raw_parts(
                push_constants.as_ptr() as *const u8,
                std::mem::size_of_val(&push_constants),
            ),
        );

        // Render draw lists
        let mut vertex_offset: u32 = 0;
        let mut index_offset: u32 = 0;

        for draw_list in draw_data.draw_lists() {
            for cmd in draw_list.commands() {
                match cmd {
                    imgui::DrawCmd::Elements { count, cmd_params } => {
                        let clip_rect = cmd_params.clip_rect;
                        let scissor = vk::Rect2D::builder()
                            .offset(vk::Offset2D {
                                x: ((clip_rect[0] - draw_data.display_pos[0]) * draw_data.framebuffer_scale[0]).max(0.0) as i32,
                                y: ((clip_rect[1] - draw_data.display_pos[1]) * draw_data.framebuffer_scale[1]).max(0.0) as i32,
                            })
                            .extent(vk::Extent2D {
                                width: ((clip_rect[2] - clip_rect[0]) * draw_data.framebuffer_scale[0]) as u32,
                                height: ((clip_rect[3] - clip_rect[1]) * draw_data.framebuffer_scale[1]) as u32,
                            });
                        self.rrdevice.device.cmd_set_scissor(command_buffer, 0, &[scissor]);

                        self.rrdevice.device.cmd_draw_indexed(
                            command_buffer,
                            count as u32,
                            1,
                            index_offset + cmd_params.idx_offset as u32,
                            (vertex_offset + cmd_params.vtx_offset as u32) as i32,
                            0,
                        );
                    }
                    _ => {}
                }
            }

            vertex_offset += draw_list.vtx_buffer().len() as u32;
            index_offset += draw_list.idx_buffer().len() as u32;
        }

        Ok(())
    }

    /// Pass 1: Render models to G-Buffer (position and normal)
    unsafe fn record_gbuffer_pass(
        &self,
        command_buffer: vk::CommandBuffer,
        image_index: usize,
    ) -> Result<()> {
        let gbuffer = self.data.gbuffer.as_ref()
            .ok_or_else(|| anyhow!("G-Buffer not initialized"))?;
        let gbuffer_pipeline = self.data.gbuffer_pipeline.as_ref()
            .ok_or_else(|| anyhow!("G-Buffer pipeline not initialized"))?;
        let gbuffer_descriptor_set = self.data.gbuffer_descriptor_set.as_ref()
            .ok_or_else(|| anyhow!("G-Buffer descriptor set not initialized"))?;

        // Begin G-Buffer render pass
        let render_area = vk::Rect2D::builder()
            .offset(vk::Offset2D::default())
            .extent(vk::Extent2D {
                width: gbuffer.width,
                height: gbuffer.height,
            });

        // Clear values for position, normal, and depth
        let position_clear = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 0.0],
            },
        };
        let normal_clear = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 0.0],
            },
        };
        let depth_clear = vk::ClearValue {
            depth_stencil: vk::ClearDepthStencilValue {
                depth: 1.0,
                stencil: 0,
            },
        };
        let clear_values = [position_clear, normal_clear, depth_clear];

        let render_pass_info = vk::RenderPassBeginInfo::builder()
            .render_pass(self.data.rrrender.gbuffer_render_pass)
            .framebuffer(self.data.rrrender.gbuffer_framebuffer)
            .render_area(render_area)
            .clear_values(&clear_values);

        self.rrdevice.device.cmd_begin_render_pass(
            command_buffer,
            &render_pass_info,
            vk::SubpassContents::INLINE,
        );

        // Bind G-Buffer pipeline
        self.rrdevice.device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            gbuffer_pipeline.pipeline,
        );

        // Render all model meshes to G-Buffer
        for i in 0..gbuffer_descriptor_set.rrdata.len() {
            let rrdata = &gbuffer_descriptor_set.rrdata[i];

            // Bind vertex and index buffers
            self.rrdevice.device.cmd_bind_vertex_buffers(
                command_buffer,
                0,
                &[rrdata.vertex_buffer.buffer],
                &[0],
            );

            self.rrdevice.device.cmd_bind_index_buffer(
                command_buffer,
                rrdata.index_buffer.buffer,
                0,
                vk::IndexType::UINT32,
            );

            // Bind descriptor set
            let swapchain_images_len = gbuffer_descriptor_set.descriptor_sets.len() /
                gbuffer_descriptor_set.rrdata.len().max(1);
            let descriptor_set_index = i * swapchain_images_len + image_index;

            self.rrdevice.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                gbuffer_pipeline.pipeline_layout,
                0,
                &[gbuffer_descriptor_set.descriptor_sets[descriptor_set_index]],
                &[],
            );

            // Draw
            self.rrdevice.device.cmd_draw_indexed(
                command_buffer,
                rrdata.index_buffer.indices,
                1,
                0,
                0,
                0,
            );
        }

        // End G-Buffer render pass
        self.rrdevice.device.cmd_end_render_pass(command_buffer);

        Ok(())
    }

    /// Pass 2: Execute Ray Query compute shader to calculate shadows
    unsafe fn record_ray_query_pass(
        &self,
        command_buffer: vk::CommandBuffer,
    ) -> Result<()> {
        let gbuffer = self.data.gbuffer.as_ref()
            .ok_or_else(|| anyhow!("G-Buffer not initialized"))?;
        let ray_query_pipeline = self.data.ray_query_pipeline.as_ref()
            .ok_or_else(|| anyhow!("Ray Query pipeline not initialized"))?;
        let ray_query_descriptor = self.data.ray_query_descriptor.as_ref()
            .ok_or_else(|| anyhow!("Ray Query descriptor set not initialized"))?;

        // Memory barrier: G-Buffer writes -> compute shader reads
        let image_barriers = [
            vk::ImageMemoryBarrier::builder()
                .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ)
                .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .new_layout(vk::ImageLayout::GENERAL)
                .image(gbuffer.position_image)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .build(),
            vk::ImageMemoryBarrier::builder()
                .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ)
                .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .new_layout(vk::ImageLayout::GENERAL)
                .image(gbuffer.normal_image)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .build(),
        ];

        self.rrdevice.device.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::DependencyFlags::empty(),
            &[] as &[vk::MemoryBarrier],
            &[] as &[vk::BufferMemoryBarrier],
            &image_barriers,
        );

        // Bind compute pipeline
        self.rrdevice.device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::COMPUTE,
            ray_query_pipeline.pipeline,
        );

        // Bind descriptor set
        self.rrdevice.device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::COMPUTE,
            ray_query_pipeline.pipeline_layout,
            0,
            &[ray_query_descriptor.descriptor_set],
            &[],
        );

        // Dispatch compute shader (one thread per pixel)
        let group_count_x = (gbuffer.width + 15) / 16;  // 16x16 local workgroup size
        let group_count_y = (gbuffer.height + 15) / 16;
        self.rrdevice.device.cmd_dispatch(command_buffer, group_count_x, group_count_y, 1);

        // Memory barrier: compute shader writes -> fragment shader reads
        let shadow_barrier = vk::ImageMemoryBarrier::builder()
            .src_access_mask(vk::AccessFlags::SHADER_WRITE)
            .dst_access_mask(vk::AccessFlags::SHADER_READ)
            .old_layout(vk::ImageLayout::GENERAL)
            .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image(gbuffer.shadow_mask_image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .build();

        self.rrdevice.device.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            vk::DependencyFlags::empty(),
            &[] as &[vk::MemoryBarrier],
            &[] as &[vk::BufferMemoryBarrier],
            &[shadow_barrier],
        );

        Ok(())
    }

    /// Pass 3: Composite final image and render ImGui
    unsafe fn record_composite_pass(
        &self,
        command_buffer: vk::CommandBuffer,
        image_index: usize,
        draw_data: &imgui::DrawData,
    ) -> Result<()> {
        let composite_pipeline = self.data.composite_pipeline.as_ref()
            .ok_or_else(|| anyhow!("Composite pipeline not initialized"))?;
        let composite_descriptor = self.data.composite_descriptor.as_ref()
            .ok_or_else(|| anyhow!("Composite descriptor set not initialized"))?;

        // Begin main render pass
        let render_area = vk::Rect2D::builder()
            .offset(vk::Offset2D::default())
            .extent(self.data.rrswapchain.swapchain_extent);

        let color_clear_value = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        };
        let depth_clear_value = vk::ClearValue {
            depth_stencil: vk::ClearDepthStencilValue {
                depth: 1.0,
                stencil: 0,
            },
        };
        let clear_values = [color_clear_value, depth_clear_value];

        let render_pass_info = vk::RenderPassBeginInfo::builder()
            .render_pass(self.data.rrrender.render_pass)
            .framebuffer(self.data.rrrender.framebuffers[image_index])
            .render_area(render_area)
            .clear_values(&clear_values);

        self.rrdevice.device.cmd_begin_render_pass(
            command_buffer,
            &render_pass_info,
            vk::SubpassContents::INLINE,
        );

        // Draw fullscreen quad with composite shader
        self.rrdevice.device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            composite_pipeline.pipeline,
        );

        self.rrdevice.device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            composite_pipeline.pipeline_layout,
            0,
            &[composite_descriptor.descriptor_set],
            &[],
        );

        // Draw fullscreen triangle (no vertex buffer needed)
        self.rrdevice.device.cmd_draw(command_buffer, 3, 1, 0, 0);

        // Draw ImGui on top
        self.record_imgui_rendering(command_buffer, draw_data)?;

        // End render pass
        self.rrdevice.device.cmd_end_render_pass(command_buffer);

        Ok(())
    }

    /// Helper function to transition image layout and copy buffer to image
    unsafe fn transition_image_layout_and_copy(
        device: &vulkanalia::Device,
        command_pool: &RRCommandPool,
        graphics_queue: &vk::Queue,
        image: vk::Image,
        buffer: vk::Buffer,
        width: u32,
        height: u32,
    ) -> Result<()> {
        let allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(command_pool.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let command_buffer = device.allocate_command_buffers(&allocate_info)?[0];

        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        device.begin_command_buffer(command_buffer, &begin_info)?;

        // Transition to TRANSFER_DST_OPTIMAL
        let barrier = vk::ImageMemoryBarrier::builder()
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .src_access_mask(vk::AccessFlags::empty())
            .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE);

        device.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::TRANSFER,
            vk::DependencyFlags::empty(),
            &[] as &[vk::MemoryBarrier],
            &[] as &[vk::BufferMemoryBarrier],
            &[barrier],
        );

        // Copy buffer to image
        let region = vk::BufferImageCopy::builder()
            .buffer_offset(0)
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            })
            .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
            .image_extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            });

        device.cmd_copy_buffer_to_image(
            command_buffer,
            buffer,
            image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &[region],
        );

        // Transition to SHADER_READ_ONLY_OPTIMAL
        let barrier = vk::ImageMemoryBarrier::builder()
            .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .dst_access_mask(vk::AccessFlags::SHADER_READ);

        device.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            vk::DependencyFlags::empty(),
            &[] as &[vk::MemoryBarrier],
            &[] as &[vk::BufferMemoryBarrier],
            &[barrier],
        );

        device.end_command_buffer(command_buffer)?;

        // Submit command buffer
        let command_buffers = [command_buffer];
        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(&command_buffers);

        device.queue_submit(*graphics_queue, &[submit_info], vk::Fence::null())?;
        device.queue_wait_idle(*graphics_queue)?;

        device.free_command_buffers(command_pool.command_pool, &[command_buffer]);

        Ok(())
    }
}

