use crate::app::init::MAX_FRAMES_IN_FLIGHT;
use crate::app::App;
use crate::ecs::resource::billboard::BillboardData;
use crate::ecs::resource::gizmo::{GridGizmoData, LightGizmoData};
use crate::ecs::resource::{Camera, GridMeshData, ImguiTextures};
use crate::ecs::systems::render_data_systems::{
    gizmo_mesh_render_data, gizmo_selectable_render_data, grid_mesh_render_data,
};
use crate::vulkanr::context::{
    CommandState, FrameSync, PipelineState, RenderTargets, SwapchainState,
};
use crate::vulkanr::descriptor::CompositeGBufferViews;
use crate::vulkanr::renderer::scene_renderer::render_scene_objects;
use crate::vulkanr::vulkan::*;
use thyllore_vulkan_core::renderer::create_gbuffer_framebuffer;

use anyhow::{anyhow, Result};
use thyllore_vulkan_core::core::{
    acquire_swapchain_image, submit_and_present, AcquireOutcome, FrameSubmit, PresentOutcome,
};
use thyllore_vulkan_core::renderer::{
    begin_render_pass_cleared, bind_graphics_descriptor_set, bind_graphics_mesh, draw_indexed,
    push_vertex_constants, set_full_viewport_and_scissor, set_scissor, set_viewport_size,
    IndexWidth,
};
use thyllore_vulkan_core::resource::read_picked_texel;
use thyllore_vulkan_core::types::{
    Buffer, CommandBuffer, Extent2D, ImageView, Offset2D, Pipeline, PipelineLayout, Rect2D, Sampler,
};

impl App {
    pub unsafe fn begin_frame(&mut self) -> Result<usize> {
        self.handle_viewport_resize()?;

        let current_fence = self.resource::<FrameSync>().current_fence();
        self.rrdevice.wait_for_fence(current_fence)?;

        let frame_slot = self.resource::<FrameSync>().current_frame;
        self.data
            .viewport
            .transient
            .begin_frame(&self.rrdevice.device, frame_slot)?;

        self.update_auto_exposure();
        self.read_object_id_readback();

        let swapchain = self.resource::<SwapchainState>().swapchain.swapchain;
        let image_available = self.resource::<FrameSync>().current_image_available();
        let image_index = match acquire_swapchain_image(&self.rrdevice, swapchain, image_available)?
        {
            AcquireOutcome::Image(image_index) => image_index,
            AcquireOutcome::OutOfDate => return Err(anyhow!("SWAPCHAIN_OUT_OF_DATE")),
        };

        let image_in_flight = self.resource::<SwapchainState>().images_in_flight[image_index];
        if !image_in_flight.is_null() {
            self.rrdevice.wait_for_fence(image_in_flight)?;
        }

        let current_fence = self.resource::<FrameSync>().current_fence();
        self.resource_mut::<SwapchainState>().images_in_flight[image_index] = current_fence;

        Ok(image_index)
    }

    unsafe fn handle_viewport_resize(&mut self) -> Result<()> {
        let pending = self
            .data
            .ecs_world
            .resource_mut::<crate::ecs::resource::ViewportInput>()
            .resize_pending
            .take();
        let Some((width, height)) = pending else {
            return Ok(());
        };

        self.rrdevice.wait_idle()?;
        self.data.pass_image_states.clear();
        let command_pool = self.resource::<CommandState>().pool.command_pool;
        self.data
            .viewport
            .resize(&self.instance, &self.rrdevice, command_pool, width, height)?;

        self.resize_gbuffer(width, height)?;
        self.run_effect_viewport_resize()?;
        self.resize_post_process_bindings()?;

        if self
            .data
            .ecs_world
            .contains_resource::<crate::ecs::resource::ObjectIdReadback>()
        {
            let mut readback = self
                .data
                .ecs_world
                .resource_mut::<crate::ecs::resource::ObjectIdReadback>();
            readback.pending_pixel = None;
            readback.copy_in_flight = false;
            readback.last_read_object_id = None;
            readback.last_read_world_position = None;
        }

        Ok(())
    }

    unsafe fn resize_gbuffer(&mut self, new_width: u32, new_height: u32) -> Result<()> {
        let needs_resize = self
            .data
            .raytracing
            .gbuffer
            .as_ref()
            .map(|gb| gb.width != new_width || gb.height != new_height)
            .unwrap_or(false);

        if !needs_resize {
            self.attach_gbuffer_depth_to_hdr()?;
            return Ok(());
        }

        let command_pool = self.resource::<CommandState>().pool.command_pool;

        if let Some(ref mut gbuffer) = self.data.raytracing.gbuffer {
            gbuffer.resize(&self.instance, &self.rrdevice, new_width, new_height)?;
            gbuffer.transition_layouts(&self.rrdevice, command_pool)?;
        }

        let Some(ref gbuffer) = self.data.raytracing.gbuffer else {
            return Ok(());
        };
        let position_view = gbuffer.position_image_view;
        let normal_view = gbuffer.normal_image_view;
        let shadow_mask_view = gbuffer.shadow_mask_image_view;
        let albedo_view = gbuffer.albedo_image_view;
        let object_id_view = gbuffer.object_id_image_view;
        self.recreate_gbuffer_framebuffer()?;
        self.attach_gbuffer_depth_to_hdr()?;
        self.update_gbuffer_descriptors(
            position_view,
            normal_view,
            shadow_mask_view,
            albedo_view,
            object_id_view,
        )?;
        self.recreate_onion_skin_on_resize()?;
        log!("G-Buffer resized to: {}x{}", new_width, new_height);
        Ok(())
    }

    /// The HDR framebuffer borrows the G-buffer depth view; a resized HDR
    /// buffer has no framebuffer until the current depth is attached here.
    unsafe fn attach_gbuffer_depth_to_hdr(&mut self) -> Result<()> {
        let depth_view = {
            let rt = self.resource::<RenderTargets>();
            rt.render.gbuffer_depth_image_view
        };
        if depth_view == ImageView::null() {
            return Ok(());
        }
        if let Some(hdr_buffer) = &mut self.data.viewport.hdr_buffer {
            hdr_buffer.attach_depth(&self.rrdevice, depth_view)?;
        }
        Ok(())
    }

    unsafe fn recreate_gbuffer_framebuffer(&mut self) -> Result<()> {
        let mut render_targets = self.resource_mut::<RenderTargets>();
        render_targets
            .render
            .destroy_gbuffer_attachments(&self.rrdevice.device);

        if let Some(ref gbuffer) = self.data.raytracing.gbuffer {
            create_gbuffer_framebuffer(
                &self.instance,
                &self.rrdevice,
                &mut render_targets.render,
                gbuffer,
            )?;
        }

        Ok(())
    }

    unsafe fn update_gbuffer_descriptors(
        &mut self,
        position_view: ImageView,
        normal_view: ImageView,
        shadow_mask_view: ImageView,
        albedo_view: ImageView,
        object_id_view: ImageView,
    ) -> Result<()> {
        let gbuffer_sampler = self
            .data
            .raytracing
            .gbuffer_sampler
            .unwrap_or(Sampler::null());
        let object_id_sampler = self
            .data
            .raytracing
            .object_id_sampler
            .unwrap_or(Sampler::null());

        if let Some(ref composite_desc) = self.data.raytracing.composite_descriptor {
            composite_desc.update_gbuffer_views(
                &self.rrdevice,
                CompositeGBufferViews {
                    position_image_view: position_view,
                    position_sampler: gbuffer_sampler,
                    normal_image_view: normal_view,
                    normal_sampler: gbuffer_sampler,
                    shadow_mask_image_view: shadow_mask_view,
                    shadow_mask_sampler: gbuffer_sampler,
                    albedo_image_view: albedo_view,
                    albedo_sampler: gbuffer_sampler,
                    object_id_image_view: object_id_view,
                    object_id_sampler,
                },
            )?;
        }

        if let Some(ref ray_query_desc) = self.data.raytracing.ray_query_descriptor {
            ray_query_desc.update_gbuffer_views(
                &self.rrdevice,
                position_view,
                normal_view,
                shadow_mask_view,
            )?;
        }

        {
            let swapchain = self.resource::<SwapchainState>().swapchain.clone();
            let mut billboard = self.resource_mut::<BillboardData>();
            billboard
                .render_state
                .descriptor_set
                .update_position_sampler(
                    &self.rrdevice,
                    &swapchain,
                    position_view,
                    gbuffer_sampler,
                )?;
        }

        Ok(())
    }

    unsafe fn recreate_onion_skin_on_resize(&mut self) -> Result<()> {
        if let (Some(ref mut onion_pass), Some(ref offscreen)) = (
            &mut self.data.raytracing.onion_skin_pass,
            &self.data.viewport.offscreen,
        ) {
            onion_pass.recreate_on_resize(
                &self.instance,
                &self.rrdevice,
                offscreen.resolve_color_image_view,
                offscreen.width,
                offscreen.height,
            )?;
        }

        Ok(())
    }

    unsafe fn read_object_id_readback(&mut self) {
        use crate::ecs::resource::ObjectIdReadback;

        if !self.data.ecs_world.contains_resource::<ObjectIdReadback>() {
            return;
        }

        let readback = self.data.ecs_world.resource::<ObjectIdReadback>();
        if !readback.copy_in_flight {
            return;
        }
        drop(readback);

        let Some(ref gbuffer) = self.data.raytracing.gbuffer else {
            return;
        };

        if let Some((object_id, world_position)) =
            read_picked_texel(&self.rrdevice, gbuffer.readback_staging_memory)
        {
            let mut readback = self.data.ecs_world.resource_mut::<ObjectIdReadback>();
            readback.last_read_object_id = Some(object_id);
            readback.last_read_world_position = (object_id != 0).then_some(world_position);
            readback.copy_in_flight = false;
        }
    }

    unsafe fn update_auto_exposure(&mut self) {
        let ae_enabled = self
            .data
            .ecs_world
            .get_resource::<crate::ecs::resource::AutoExposure>()
            .map(|ae| ae.enabled)
            .unwrap_or(false);

        let clock = self.resource::<crate::ecs::resource::FrameClock>();
        let frame = clock.frame;
        let is_fixed_step = clock.is_fixed();
        drop(clock);

        if !ae_enabled {
            self.record_exposure_dump(frame, None);
            self.restore_manual_exposure_if_needed();
            return;
        }

        self.save_manual_exposure_if_needed();
        if let Some(light_referenced) = self.light_referenced_exposure() {
            let exposure_value = light_referenced.exposure_value;
            if let Some(mut exposure) = self
                .data
                .ecs_world
                .get_resource_mut::<crate::ecs::resource::Exposure>()
            {
                *exposure = light_referenced;
            }
            self.record_exposure_dump(frame, Some(exposure_value));
            return;
        }
        if is_fixed_step {
            let _ = self.rrdevice.wait_idle();
        }

        let readback = match self.data.viewport.auto_exposure_buffers {
            Some(ref ae_buffers) => {
                let current_slot = self.resource::<FrameSync>().current_frame;
                ae_buffers.read_luminance(&self.rrdevice.device, current_slot)
            }
            None => return,
        };
        let Some(readback) = readback else {
            return;
        };

        if readback.adapted_exposure > 0.0 {
            if let Some(mut exposure) = self
                .data
                .ecs_world
                .get_resource_mut::<crate::ecs::resource::Exposure>()
            {
                exposure.exposure_value = readback.adapted_exposure;
                exposure.ev100 = crate::ecs::resource::Exposure::ev100_from_exposure_value(
                    readback.adapted_exposure,
                );
                exposure.metered_luminance = readback.average_luminance;
            }
        }

        self.record_exposure_dump(frame, Some(readback.adapted_exposure));
    }

    fn light_referenced_exposure(&self) -> Option<crate::ecs::resource::Exposure> {
        let settings = self
            .data
            .ecs_world
            .get_resource::<crate::ecs::resource::AutoExposure>()?;
        (!settings.metering.uses_histogram())
            .then(|| settings.light_referenced_exposure(thyllore_render_core::lit_white_radiance()))
    }

    fn save_manual_exposure_if_needed(&mut self) {
        let already_saved = self
            .data
            .ecs_world
            .get_resource::<crate::ecs::resource::AutoExposure>()
            .map(|ae| ae.saved_manual_exposure.is_some())
            .unwrap_or(true);

        if already_saved {
            return;
        }

        let current_exposure = self
            .data
            .ecs_world
            .get_resource::<crate::ecs::resource::Exposure>()
            .map(|e| e.exposure_value)
            .unwrap_or(1.0);

        if let Some(mut ae) = self
            .data
            .ecs_world
            .get_resource_mut::<crate::ecs::resource::AutoExposure>()
        {
            ae.saved_manual_exposure = Some(current_exposure);
        }
    }

    fn restore_manual_exposure_if_needed(&mut self) {
        let saved = self
            .data
            .ecs_world
            .get_resource::<crate::ecs::resource::AutoExposure>()
            .and_then(|ae| ae.saved_manual_exposure);

        let restore_value = match saved {
            Some(v) => v,
            None => return,
        };

        if let Some(mut exposure) = self
            .data
            .ecs_world
            .get_resource_mut::<crate::ecs::resource::Exposure>()
        {
            exposure.exposure_value = restore_value;
        }

        if let Some(mut ae) = self
            .data
            .ecs_world
            .get_resource_mut::<crate::ecs::resource::AutoExposure>()
        {
            ae.saved_manual_exposure = None;
        }
    }

    pub unsafe fn render(&mut self, image_index: usize, draw_data: &imgui::DrawData) -> Result<()> {
        let frame_slot = self.resource::<FrameSync>().current_frame;

        Self::update_imgui_buffers(
            &self.instance,
            &self.rrdevice,
            &mut self.data,
            draw_data,
            frame_slot,
        )?;

        self.record_command_buffer(image_index, draw_data, frame_slot)?;

        let image_available = self.resource::<FrameSync>().current_image_available();
        let render_finished = self.resource::<FrameSync>().current_render_finished();
        let current_fence = self.resource::<FrameSync>().current_fence();

        let swapchain = self.resource::<SwapchainState>().swapchain.swapchain;

        let outcome = submit_and_present(
            &self.rrdevice,
            FrameSubmit {
                command_buffer: self.resource::<CommandState>().buffers.command_buffers
                    [image_index],
                image_available,
                render_finished,
                fence: current_fence,
                swapchain,
                image_index,
            },
        )?;

        if matches!(outcome, PresentOutcome::OutOfDate) || self.resized {
            self.resized = false;
            return Err(anyhow!("SWAPCHAIN_OUT_OF_DATE"));
        }

        self.resource_mut::<FrameSync>()
            .advance(MAX_FRAMES_IN_FLIGHT);
        let current_frame = self.resource::<FrameSync>().current_frame;
        self.frame = current_frame;

        Ok(())
    }

    pub unsafe fn begin_offscreen_render_pass(
        &self,
        command_buffer: CommandBuffer,
        offscreen: &crate::vulkanr::resource::OffscreenFramebuffer,
    ) {
        begin_render_pass_cleared(
            &self.rrdevice,
            command_buffer,
            offscreen.render_pass,
            offscreen.framebuffer,
            offscreen.extent(),
            [0.1, 0.1, 0.1, 1.0],
            0.0,
        );
    }

    pub unsafe fn record_3d_rendering_to_offscreen(
        &self,
        command_buffer: CommandBuffer,
        image_index: usize,
        offscreen: &crate::vulkanr::resource::OffscreenFramebuffer,
    ) -> Result<()> {
        let extent = offscreen.extent();
        set_full_viewport_and_scissor(&self.rrdevice, command_buffer, extent);

        let frame_set = self.data.graphics_resources.frame_set.sets[image_index];
        let frame_slot = self.resource::<FrameSync>().current_frame;
        let camera_pos = {
            use crate::ecs::systems::camera_systems::compute_camera_position;
            compute_camera_position(&self.resource::<Camera>())
        };

        let render_data_vec = vec![
            grid_mesh_render_data(&self.resource::<GridMeshData>()),
            gizmo_mesh_render_data(&self.resource::<GridGizmoData>()),
            gizmo_selectable_render_data(
                &self.resource::<LightGizmoData>(),
                camera_pos,
                frame_slot,
            ),
        ];
        let render_data_refs: Vec<_> = render_data_vec.iter().collect();

        crate::vulkanr::renderer::scene_renderer::render_scene_objects(
            &render_data_refs,
            command_buffer,
            image_index,
            frame_set,
            &self.data.graphics_resources.objects,
            &self.rrdevice,
            self.pipeline_storage(),
            &self.data.buffer_registry,
        );

        self.render_billboard(command_buffer, image_index);
        self.render_models(command_buffer, image_index);

        Ok(())
    }

    pub unsafe fn begin_main_render_pass(&self, command_buffer: CommandBuffer, image_index: usize) {
        let render_targets = self.resource::<RenderTargets>();
        let extent = self.resource::<SwapchainState>().swapchain.swapchain_extent;
        begin_render_pass_cleared(
            &self.rrdevice,
            command_buffer,
            render_targets.render.render_pass,
            render_targets.render.framebuffers[image_index],
            extent,
            [0.0, 0.0, 0.0, 1.0],
            0.0,
        );
    }

    unsafe fn render_models(&self, command_buffer: CommandBuffer, image_index: usize) {
        let mesh_count = self.data.graphics_resources.meshes.len();

        for i in 0..mesh_count {
            let mesh = &self.data.graphics_resources.meshes[i];

            if !mesh.render_to_gbuffer {
                continue;
            }

            let pipeline = &self.resource::<PipelineState>().model_pipeline;
            bind_graphics_mesh(
                &self.rrdevice,
                command_buffer,
                pipeline.pipeline,
                mesh.vertex_buffer.buffer,
                mesh.index_buffer.buffer,
                IndexWidth::U32,
            );

            let frame_set = self.data.graphics_resources.frame_set.sets[image_index];
            bind_graphics_descriptor_set(
                &self.rrdevice,
                command_buffer,
                pipeline.pipeline_layout,
                0,
                frame_set,
            );

            if let Some(material_id) = self.data.graphics_resources.get_material_id(i) {
                if let Some(material) = self.data.graphics_resources.materials.get(material_id) {
                    bind_graphics_descriptor_set(
                        &self.rrdevice,
                        command_buffer,
                        pipeline.pipeline_layout,
                        1,
                        material.descriptor_set,
                    );
                }
            }

            let object_set_idx = self
                .data
                .graphics_resources
                .objects
                .get_set_index(image_index, mesh.object_index);
            let object_set = self.data.graphics_resources.objects.sets[object_set_idx];
            bind_graphics_descriptor_set(
                &self.rrdevice,
                command_buffer,
                pipeline.pipeline_layout,
                2,
                object_set,
            );

            draw_indexed(
                &self.rrdevice,
                command_buffer,
                mesh.index_buffer.indices,
                0,
                0,
            );
        }
    }

    pub unsafe fn record_3d_rendering(
        &self,
        command_buffer: CommandBuffer,
        image_index: usize,
    ) -> Result<()> {
        let extent = self.resource::<SwapchainState>().swapchain.swapchain_extent;
        set_full_viewport_and_scissor(&self.rrdevice, command_buffer, extent);

        let frame_set = self.data.graphics_resources.frame_set.sets[image_index];
        let frame_slot = self.resource::<FrameSync>().current_frame;
        let camera_pos = {
            use crate::ecs::systems::camera_systems::compute_camera_position;
            compute_camera_position(&self.resource::<Camera>())
        };

        let render_data_vec = vec![
            grid_mesh_render_data(&self.resource::<GridMeshData>()),
            gizmo_mesh_render_data(&self.resource::<GridGizmoData>()),
            gizmo_selectable_render_data(
                &self.resource::<LightGizmoData>(),
                camera_pos,
                frame_slot,
            ),
        ];
        let render_data_refs: Vec<_> = render_data_vec.iter().collect();

        render_scene_objects(
            &render_data_refs,
            command_buffer,
            image_index,
            frame_set,
            &self.data.graphics_resources.objects,
            &self.rrdevice,
            self.pipeline_storage(),
            &self.data.buffer_registry,
        );

        self.render_billboard(command_buffer, image_index);

        self.render_models(command_buffer, image_index);

        Ok(())
    }

    unsafe fn render_billboard(&self, command_buffer: CommandBuffer, image_index: usize) {
        let billboard = self.resource::<BillboardData>();
        let pipeline_storage = self.pipeline_storage();

        let vertex_buffer = match self
            .data
            .buffer_registry
            .get_vertex_buffer(billboard.mesh.vertex_buffer_handle)
        {
            Some(b) => b,
            None => return,
        };
        let index_buffer = match self
            .data
            .buffer_registry
            .get_index_buffer(billboard.mesh.index_buffer_handle)
        {
            Some(b) => b,
            None => return,
        };

        let pipeline_id = match billboard.render_info.pipeline_id {
            Some(id) => id,
            None => return,
        };
        let pipeline = match pipeline_storage.get(pipeline_id) {
            Some(p) => p,
            None => return,
        };

        bind_graphics_mesh(
            &self.rrdevice,
            command_buffer,
            pipeline.pipeline,
            vertex_buffer,
            index_buffer,
            IndexWidth::U32,
        );

        let descriptor_set = billboard.render_state.descriptor_set.descriptor_sets[image_index];
        bind_graphics_descriptor_set(
            &self.rrdevice,
            command_buffer,
            pipeline.pipeline_layout,
            0,
            descriptor_set,
        );

        draw_indexed(
            &self.rrdevice,
            command_buffer,
            billboard.mesh.indices.len() as u32,
            0,
            0,
        );
    }

    pub unsafe fn record_imgui_rendering(
        &self,
        command_buffer: CommandBuffer,
        draw_data: &imgui::DrawData,
    ) -> Result<()> {
        if draw_data.total_vtx_count() == 0 || draw_data.total_idx_count() == 0 {
            return Ok(());
        }

        let frame_slot = self.resource::<FrameSync>().current_frame;

        let pipeline = self
            .data
            .imgui
            .pipeline
            .ok_or_else(|| anyhow!("ImGui pipeline not initialized"))?;
        let pipeline_layout = self
            .data
            .imgui
            .pipeline_layout
            .ok_or_else(|| anyhow!("ImGui pipeline layout not initialized"))?;
        let vertex_buffer = self.data.imgui.vertex_buffers[frame_slot]
            .as_ref()
            .map(|b| b.buffer)
            .ok_or_else(|| anyhow!("ImGui vertex buffer not initialized"))?;
        let index_buffer = self.data.imgui.index_buffers[frame_slot]
            .as_ref()
            .map(|b| b.buffer)
            .ok_or_else(|| anyhow!("ImGui index buffer not initialized"))?;

        self.setup_imgui_render_state(
            command_buffer,
            draw_data,
            pipeline,
            pipeline_layout,
            vertex_buffer,
            index_buffer,
        );

        self.record_imgui_draw_commands(command_buffer, draw_data, pipeline_layout);

        Ok(())
    }

    unsafe fn setup_imgui_render_state(
        &self,
        command_buffer: CommandBuffer,
        draw_data: &imgui::DrawData,
        pipeline: Pipeline,
        pipeline_layout: PipelineLayout,
        vertex_buffer: Buffer,
        index_buffer: Buffer,
    ) {
        bind_graphics_mesh(
            &self.rrdevice,
            command_buffer,
            pipeline,
            vertex_buffer,
            index_buffer,
            IndexWidth::U16,
        );

        let fb_width = draw_data.display_size()[0] * draw_data.framebuffer_scale()[0];
        let fb_height = draw_data.display_size()[1] * draw_data.framebuffer_scale()[1];
        set_viewport_size(&self.rrdevice, command_buffer, fb_width, fb_height);

        let scale = [
            2.0 / draw_data.display_size()[0],
            2.0 / draw_data.display_size()[1],
        ];
        let translate = [
            -1.0 - draw_data.display_pos()[0] * scale[0],
            -1.0 - draw_data.display_pos()[1] * scale[1],
        ];
        let push_constants = [scale[0], scale[1], translate[0], translate[1]];

        push_vertex_constants(
            &self.rrdevice,
            command_buffer,
            pipeline_layout,
            std::slice::from_raw_parts(
                push_constants.as_ptr() as *const u8,
                std::mem::size_of_val(&push_constants),
            ),
        );
    }

    unsafe fn record_imgui_draw_commands(
        &self,
        command_buffer: CommandBuffer,
        draw_data: &imgui::DrawData,
        pipeline_layout: PipelineLayout,
    ) {
        let viewport_texture_id = self.data.viewport.texture_id();
        let viewport_descriptor_set = self.data.viewport.descriptor_set;
        let mut bound_texture_id: Option<u64> = None;

        let mut vertex_offset: u32 = 0;
        let mut index_offset: u32 = 0;

        for draw_list in draw_data.draw_lists() {
            for cmd in draw_list.commands() {
                match cmd {
                    imgui::DrawCmd::Elements { count, cmd_params } => {
                        let texture_id = cmd_params.texture_id.id();

                        if bound_texture_id != Some(texture_id) {
                            let descriptor_set = if texture_id == viewport_texture_id {
                                Some(viewport_descriptor_set)
                            } else {
                                self.resource::<ImguiTextures>()
                                    .descriptor_set_for(texture_id)
                            };
                            let Some(descriptor_set) = descriptor_set else {
                                continue;
                            };
                            bind_graphics_descriptor_set(
                                &self.rrdevice,
                                command_buffer,
                                pipeline_layout,
                                0,
                                descriptor_set,
                            );
                            bound_texture_id = Some(texture_id);
                        }

                        let clip_rect = cmd_params.clip_rect;
                        let scissor = Rect2D::builder()
                            .offset(Offset2D {
                                x: ((clip_rect[0] - draw_data.display_pos()[0])
                                    * draw_data.framebuffer_scale()[0])
                                    .max(0.0) as i32,
                                y: ((clip_rect[1] - draw_data.display_pos()[1])
                                    * draw_data.framebuffer_scale()[1])
                                    .max(0.0) as i32,
                            })
                            .extent(Extent2D {
                                width: ((clip_rect[2] - clip_rect[0])
                                    * draw_data.framebuffer_scale()[0])
                                    as u32,
                                height: ((clip_rect[3] - clip_rect[1])
                                    * draw_data.framebuffer_scale()[1])
                                    as u32,
                            });
                        set_scissor(&self.rrdevice, command_buffer, scissor.build());

                        draw_indexed(
                            &self.rrdevice,
                            command_buffer,
                            count as u32,
                            index_offset + cmd_params.idx_offset as u32,
                            (vertex_offset + cmd_params.vtx_offset as u32) as i32,
                        );
                    }
                    _ => {}
                }
            }

            vertex_offset += draw_list.vtx_buffer().len() as u32;
            index_offset += draw_list.idx_buffer().len() as u32;
        }
    }
}
