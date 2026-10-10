use crate::app::{App, AppData};

use crate::ecs::component::RenderInfo;
use crate::ecs::resource::GridMeshData;
use crate::ecs::systems::{create_default_grid_scale, create_grid_mesh};
use crate::ecs::{
    ClipLibrary, GpuDescriptors, MaterialRegistry, MeshAssets, ModelState, NodeAssets,
};
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

use anyhow::{Context, Result};
use std::rc::Rc;

pub(super) struct VulkanResources {
    pub(super) messenger: vk::DebugUtilsMessengerEXT,
    pub(super) surface: vk::SurfaceKHR,
    pub(super) rrswapchain: RRSwapchain,
    pub(super) rrrender: RRRender,
    pub(super) rrcommand_pool: Rc<RRCommandPool>,
    pub(super) rrcommand_buffer: RRCommandBuffer,
    pub(super) model_pipeline: RRPipeline,
    pub(super) image_available_semaphores: Vec<vk::Semaphore>,
    pub(super) render_finish_semaphores: Vec<vk::Semaphore>,
    pub(super) in_flight_fences: Vec<vk::Fence>,
}

impl App {
    pub(super) unsafe fn initialize_graphics_and_ecs(
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
                // covered", which the tonemap needs to keep the grid color. Effects
                // composite over it afterwards with premultiplied blending.
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

    pub(super) unsafe fn initialize_ray_tracing(
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

    pub(super) unsafe fn build_grid_mesh(
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

    pub(super) fn register_vulkan_resources(
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
}
