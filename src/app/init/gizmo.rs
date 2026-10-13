use crate::app::{App, AppData};

use crate::ecs::resource::gizmo::{BoneDisplayStyle, BoneGizmoData, ConstraintGizmoData};
use crate::ecs::resource::pipeline_allocate_id;
use crate::ecs::systems::{
    billboard_create_buffers, create_billboard, create_grid_gizmo, create_light_gizmo,
    gizmo_create_buffers,
};
use crate::ecs::{LightState, PipelineManager};
use crate::vulkanr::command::*;
use crate::vulkanr::data::*;
use crate::vulkanr::descriptor::*;
use crate::vulkanr::device::*;
use crate::vulkanr::pipeline::{
    BlendConfig, DepthTestConfig, PipelineBuilder, PushConstantConfig, RRPipeline,
    VertexInputConfig,
};
use crate::vulkanr::render::*;
use crate::vulkanr::swapchain::*;
use crate::vulkanr::vulkan::*;
use crate::vulkanr::VulkanBackend;

use anyhow::{Context, Result};
use std::rc::Rc;

pub(super) struct GizmoPipelineIds {
    pub(super) grid: usize,
    gizmo: usize,
    bone_solid: usize,
    bone_wire: usize,
    bone_solid_depth: usize,
    bone_wire_depth: usize,
    bone_solid_occluded: usize,
    bone_wire_occluded: usize,
}

impl App {
    pub(super) unsafe fn build_gizmo_pipelines(
        rrdevice: &RRDevice,
        rrswapchain: &RRSwapchain,
        rrrender: &RRRender,
        render_layouts: &[&ReflectedSetLayout],
        pipeline_storage: &mut crate::vulkanr::resource::PipelineStorage,
        pipeline_manager: &mut PipelineManager,
    ) -> Result<GizmoPipelineIds> {
        let grid = PipelineBuilder::from_pass(&GRID)
            .vertex_input(VertexInputConfig::Gizmo)
            .topology(vk::PrimitiveTopology::LINE_LIST)
            .polygon_mode(vk::PolygonMode::LINE)
            .depth_test(DepthTestConfig {
                test_enable: true,
                write_enable: false,
                compare_op: vk::CompareOp::GREATER_OR_EQUAL,
            })
            .descriptor_layouts(&render_layouts)
            .build(rrdevice, rrrender, Some(rrswapchain.swapchain_extent))
            .context("Failed to create grid pipeline")?;
        let grid = pipeline_storage.register(grid);
        pipeline_allocate_id(pipeline_manager);

        let gizmo = PipelineBuilder::from_pass(&GIZMO)
            .vertex_input(VertexInputConfig::Gizmo)
            .topology(vk::PrimitiveTopology::LINE_LIST)
            .polygon_mode(vk::PolygonMode::LINE)
            .no_depth_test()
            .descriptor_layouts(&render_layouts)
            .build(rrdevice, rrrender, Some(rrswapchain.swapchain_extent))
            .context("Failed to create gizmo pipeline")?;
        let gizmo = pipeline_storage.register(gizmo);
        pipeline_allocate_id(pipeline_manager);

        let bone_push_constants = PushConstantConfig {
            stage_flags: vk::ShaderStageFlags::FRAGMENT,
            offset: 0,
            size: std::mem::size_of::<f32>() as u32,
        };
        let depth_front = DepthTestConfig {
            test_enable: true,
            write_enable: false,
            compare_op: vk::CompareOp::GREATER_OR_EQUAL,
        };
        let depth_behind = DepthTestConfig {
            test_enable: true,
            write_enable: false,
            compare_op: vk::CompareOp::LESS_OR_EQUAL,
        };

        let bone_solid = Self::build_bone_pipeline(
            rrdevice,
            rrswapchain,
            rrrender,
            render_layouts,
            vk::PrimitiveTopology::TRIANGLE_LIST,
            vk::PolygonMode::FILL,
            Some(vk::CullModeFlags::BACK),
            None,
            None,
            bone_push_constants,
            pipeline_storage,
            pipeline_manager,
            "bone solid",
        )?;

        let bone_wire = Self::build_bone_pipeline(
            rrdevice,
            rrswapchain,
            rrrender,
            render_layouts,
            vk::PrimitiveTopology::LINE_LIST,
            vk::PolygonMode::LINE,
            None,
            None,
            None,
            bone_push_constants,
            pipeline_storage,
            pipeline_manager,
            "bone wire",
        )?;

        let bone_solid_depth = Self::build_bone_pipeline(
            rrdevice,
            rrswapchain,
            rrrender,
            render_layouts,
            vk::PrimitiveTopology::TRIANGLE_LIST,
            vk::PolygonMode::FILL,
            Some(vk::CullModeFlags::BACK),
            Some(depth_front),
            None,
            bone_push_constants,
            pipeline_storage,
            pipeline_manager,
            "bone solid depth",
        )?;

        let bone_wire_depth = Self::build_bone_pipeline(
            rrdevice,
            rrswapchain,
            rrrender,
            render_layouts,
            vk::PrimitiveTopology::LINE_LIST,
            vk::PolygonMode::LINE,
            None,
            Some(depth_front),
            None,
            bone_push_constants,
            pipeline_storage,
            pipeline_manager,
            "bone wire depth",
        )?;

        let bone_solid_occluded = Self::build_bone_pipeline(
            rrdevice,
            rrswapchain,
            rrrender,
            render_layouts,
            vk::PrimitiveTopology::TRIANGLE_LIST,
            vk::PolygonMode::FILL,
            Some(vk::CullModeFlags::BACK),
            Some(depth_behind),
            Some(BlendConfig::default()),
            bone_push_constants,
            pipeline_storage,
            pipeline_manager,
            "bone solid occluded",
        )?;

        let bone_wire_occluded = Self::build_bone_pipeline(
            rrdevice,
            rrswapchain,
            rrrender,
            render_layouts,
            vk::PrimitiveTopology::LINE_LIST,
            vk::PolygonMode::LINE,
            None,
            Some(depth_behind),
            Some(BlendConfig::default()),
            bone_push_constants,
            pipeline_storage,
            pipeline_manager,
            "bone wire occluded",
        )?;

        Ok(GizmoPipelineIds {
            grid,
            gizmo,
            bone_solid,
            bone_wire,
            bone_solid_depth,
            bone_wire_depth,
            bone_solid_occluded,
            bone_wire_occluded,
        })
    }

    #[allow(clippy::too_many_arguments)]
    unsafe fn build_bone_pipeline(
        rrdevice: &RRDevice,
        rrswapchain: &RRSwapchain,
        rrrender: &RRRender,
        render_layouts: &[&ReflectedSetLayout],
        topology: vk::PrimitiveTopology,
        polygon_mode: vk::PolygonMode,
        cull_mode: Option<vk::CullModeFlags>,
        depth_test: Option<DepthTestConfig>,
        blend: Option<BlendConfig>,
        push_constants: PushConstantConfig,
        pipeline_storage: &mut crate::vulkanr::resource::PipelineStorage,
        pipeline_manager: &mut PipelineManager,
        label: &str,
    ) -> Result<usize> {
        let mut builder = PipelineBuilder::from_pass(&BONE)
            .vertex_input(VertexInputConfig::Gizmo)
            .topology(topology)
            .polygon_mode(polygon_mode)
            .push_constants(push_constants)
            .descriptor_layouts(render_layouts);

        if let Some(cull) = cull_mode {
            builder = builder.cull_mode(cull);
        }

        match depth_test {
            Some(config) => builder = builder.depth_test(config),
            None => builder = builder.no_depth_test(),
        }

        if let Some(blend_config) = blend {
            builder = builder.blend(blend_config);
        }

        let pipeline = builder
            .build(rrdevice, rrrender, Some(rrswapchain.swapchain_extent))
            .context(format!("Failed to create {} pipeline", label))?;
        let id = pipeline_storage.register(pipeline);
        pipeline_allocate_id(pipeline_manager);
        log!("Registered {} pipeline with id {}", label, id);

        Ok(id)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) unsafe fn initialize_gizmo_resources(
        instance: &Instance,
        rrdevice: &RRDevice,
        rrcommand_pool: &Rc<RRCommandPool>,
        rrswapchain: &RRSwapchain,
        rrrender: &RRRender,
        pipeline_ids: &GizmoPipelineIds,
        data: &mut AppData,
        pipeline_manager: &mut PipelineManager,
    ) -> Result<()> {
        let mut gizmo_data = create_grid_gizmo();
        gizmo_data.render_info.object_index = data.graphics_resources.objects.allocate_slot();
        gizmo_data.render_info.pipeline_id = Some(pipeline_ids.gizmo);
        {
            let mut backend = VulkanBackend::new(
                instance,
                rrdevice,
                rrcommand_pool.clone(),
                &mut data.graphics_resources,
                &mut data.raytracing,
                &mut data.buffer_registry,
            );
            gizmo_create_buffers(
                &mut gizmo_data.mesh,
                &mut backend,
                0,
                crate::render::BufferMemoryType::DeviceLocal,
            )
            .expect("Failed to create gizmo buffers");
        }

        let light_position = data.ecs_world.resource::<LightState>().light_position;
        let mut light_gizmo_data = create_light_gizmo(light_position);
        light_gizmo_data.render_info.pipeline_id = Some(pipeline_ids.gizmo);
        light_gizmo_data.render_info.object_index = data.graphics_resources.objects.allocate_slot();
        {
            let mut backend = VulkanBackend::new(
                instance,
                rrdevice,
                rrcommand_pool.clone(),
                &mut data.graphics_resources,
                &mut data.raytracing,
                &mut data.buffer_registry,
            );
            gizmo_create_buffers(
                &mut light_gizmo_data.mesh,
                &mut backend,
                0,
                crate::render::BufferMemoryType::HostVisible,
            )
            .expect("Failed to create light gizmo buffers");
        }

        Self::setup_bone_gizmo_resources(pipeline_ids, data);
        Self::setup_transform_gizmo_resources(pipeline_ids, data);

        data.ecs_world
            .insert_resource(crate::ecs::resource::PointerState::default());
        data.ecs_world
            .insert_resource(crate::ecs::resource::PointerCapture::default());

        let billboard_data = Self::initialize_billboard(
            instance,
            rrdevice,
            rrcommand_pool,
            rrswapchain,
            rrrender,
            data,
            pipeline_manager,
        )?;

        data.ecs_world.insert_resource(gizmo_data);
        data.ecs_world.insert_resource(light_gizmo_data);
        data.ecs_world.insert_resource(billboard_data);

        Ok(())
    }

    fn setup_bone_gizmo_resources(pipeline_ids: &GizmoPipelineIds, data: &mut AppData) {
        let mut bone_gizmo_data = BoneGizmoData::default();
        bone_gizmo_data.stick_render_info.pipeline_id = Some(pipeline_ids.grid);
        bone_gizmo_data.stick_render_info.object_index =
            data.graphics_resources.objects.allocate_slot();
        bone_gizmo_data.solid_render_info.pipeline_id = Some(pipeline_ids.bone_solid);
        bone_gizmo_data.solid_render_info.object_index =
            data.graphics_resources.objects.allocate_slot();
        bone_gizmo_data.wire_render_info.pipeline_id = Some(pipeline_ids.bone_wire);
        bone_gizmo_data.wire_render_info.object_index =
            data.graphics_resources.objects.allocate_slot();

        bone_gizmo_data.solid_depth_render_info.pipeline_id = Some(pipeline_ids.bone_solid_depth);
        bone_gizmo_data.solid_depth_render_info.object_index =
            bone_gizmo_data.solid_render_info.object_index;
        bone_gizmo_data.wire_depth_render_info.pipeline_id = Some(pipeline_ids.bone_wire_depth);
        bone_gizmo_data.wire_depth_render_info.object_index =
            bone_gizmo_data.wire_render_info.object_index;
        bone_gizmo_data.solid_occluded_render_info.pipeline_id =
            Some(pipeline_ids.bone_solid_occluded);
        bone_gizmo_data.solid_occluded_render_info.object_index =
            bone_gizmo_data.solid_render_info.object_index;
        bone_gizmo_data.wire_occluded_render_info.pipeline_id =
            Some(pipeline_ids.bone_wire_occluded);
        bone_gizmo_data.wire_occluded_render_info.object_index =
            bone_gizmo_data.wire_render_info.object_index;

        bone_gizmo_data.display_style = BoneDisplayStyle::Octahedral;
        data.ecs_world.insert_resource(bone_gizmo_data);
        data.ecs_world
            .insert_resource(crate::ecs::resource::gizmo::BoneSelectionState::default());
        data.ecs_world
            .insert_resource(crate::ecs::resource::WeightHeatmapState::default());

        let mut constraint_gizmo_data = ConstraintGizmoData::default();
        constraint_gizmo_data.wire_render_info.pipeline_id = Some(pipeline_ids.bone_wire);
        constraint_gizmo_data.wire_render_info.object_index =
            data.graphics_resources.objects.allocate_slot();
        data.ecs_world.insert_resource(constraint_gizmo_data);

        let mut spring_bone_gizmo_data =
            crate::ecs::resource::gizmo::SpringBoneGizmoData::default();
        spring_bone_gizmo_data.wire_render_info.pipeline_id = Some(pipeline_ids.bone_wire);
        spring_bone_gizmo_data.wire_render_info.object_index =
            data.graphics_resources.objects.allocate_slot();
        data.ecs_world.insert_resource(spring_bone_gizmo_data);
        data.ecs_world
            .insert_resource(crate::ecs::resource::SpringBoneEditorState::default());
        data.ecs_world
            .insert_resource(crate::ecs::resource::BlendShapeInspectorState::default());
        data.ecs_world
            .insert_resource(crate::ecs::resource::MorphTrackPlayback::default());
        data.ecs_world
            .insert_resource(crate::ecs::resource::ExpressionLibraryState::default());
        data.ecs_world
            .insert_resource(crate::ecs::resource::AvatarSetupState::default());
        data.ecs_world
            .insert_resource(crate::ecs::resource::MaterialTextureState::default());
    }

    fn setup_transform_gizmo_resources(pipeline_ids: &GizmoPipelineIds, data: &mut AppData) {
        let mut tg = crate::ecs::resource::gizmo::TransformGizmoData::default();
        tg.line_render_info.pipeline_id = Some(pipeline_ids.bone_wire);
        tg.line_render_info.object_index = data.graphics_resources.objects.allocate_slot();
        tg.solid_render_info.pipeline_id = Some(pipeline_ids.bone_solid);
        tg.solid_render_info.object_index = data.graphics_resources.objects.allocate_slot();
        data.ecs_world.insert_resource(tg);
        data.ecs_world
            .insert_resource(crate::ecs::resource::TransformGizmoState::default());
    }

    unsafe fn initialize_billboard(
        instance: &Instance,
        rrdevice: &RRDevice,
        rrcommand_pool: &Rc<RRCommandPool>,
        rrswapchain: &RRSwapchain,
        rrrender: &RRRender,
        data: &mut AppData,
        pipeline_manager: &mut PipelineManager,
    ) -> Result<crate::ecs::resource::billboard::BillboardData> {
        let mut billboard_data = create_billboard();
        billboard_data.render_info.object_index = data.graphics_resources.objects.allocate_slot();

        {
            let mut backend = VulkanBackend::new(
                instance,
                rrdevice,
                rrcommand_pool.clone(),
                &mut data.graphics_resources,
                &mut data.raytracing,
                &mut data.buffer_registry,
            );
            billboard_create_buffers(&mut billboard_data, &mut backend)
                .context("Failed to create billboard buffers")?;
        }

        billboard_data.render_state.descriptor_set = RRBillboardDescriptorSet::new(rrdevice)
            .context("Failed to create billboard descriptor set")?;
        billboard_data
            .render_state
            .descriptor_set
            .rrdata
            .push(RRData::new(instance, rrdevice, rrswapchain, "billboard")?);

        billboard_data
            .render_state
            .descriptor_set
            .allocate_descriptor_sets(rrdevice, rrswapchain)
            .context("Failed to allocate billboard descriptor sets")?;

        billboard_data
            .render_state
            .descriptor_set
            .update_descriptor_sets(rrdevice, rrswapchain)
            .context("Failed to update billboard descriptor sets")?;

        let billboard_pipeline = RRPipeline::new_billboard(
            rrdevice,
            rrrender,
            rrswapchain,
            &billboard_data.render_state.descriptor_set.layout,
            &BILLBOARD,
        )
        .context("Failed to create billboard pipeline")?;
        let billboard_pipeline_id = data.pipeline_storage.register(billboard_pipeline);
        pipeline_allocate_id(pipeline_manager);
        billboard_data.render_info.pipeline_id = Some(billboard_pipeline_id);

        Ok(billboard_data)
    }
}
