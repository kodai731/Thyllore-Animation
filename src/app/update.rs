use crate::app::model_loader::replace_model_with_cube;
use crate::app::{App, AppData, GUIData};
use crate::debugview::*;
use crate::ecs::{
    animation_time_system, billboard_transform_set_position, billboard_transform_update_look_at,
    camera_input_system, gizmo_reset_selection, gizmo_sync_position, gizmo_try_select,
    gizmo_update_or_create_vertical_line_buffers, gizmo_update_position_with_constraint,
    gizmo_update_rotation, gizmo_update_selection_color, gizmo_update_vertex_buffer,
    gizmo_update_vertical_lines, transform_propagation_system, update_object_ubo_system,
    CameraState, GizmoVertex,
};
use crate::math::*;
use crate::scene::billboard::BillboardTransform;
use crate::scene::graphics_resource::FrameUBO;
use crate::vulkanr::buffer::*;
use crate::vulkanr::data::*;
use crate::vulkanr::device::*;
use crate::vulkanr::vulkan::*;

use anyhow::Result;
use cgmath::{Deg, InnerSpace, Matrix4, SquareMatrix, Vector2, Vector3};
use std::mem::size_of;
use vulkanalia::prelude::v1_0::*;

impl App {
    unsafe fn update_uniform_buffer(
        &mut self,
        image_index: usize,
        mouse_pos: [f32; 2],
        mouse_wheel: f32,
        gui_data: &mut GUIData,
    ) -> Result<()> {
        use crate::app::data::LightMoveTarget;

        if gui_data.move_light_to != LightMoveTarget::None {
            let all_positions: Vec<Vector3<f32>> =
                if !self.data.graphics_resources.meshes.is_empty() {
                    self.data
                        .graphics_resources
                        .meshes
                        .iter()
                        .flat_map(|mesh| {
                            mesh.vertex_data
                                .vertices
                                .iter()
                                .map(|v| Vector3::new(v.pos.x, v.pos.y, v.pos.z))
                        })
                        .collect()
                } else {
                    Vec::new()
                };

            self.data.rt_debug_state.update_light_position(
                &all_positions,
                self.data.camera.position,
                gui_data.move_light_to,
            );
            gui_data.move_light_to = LightMoveTarget::None;
        }

        let time = self.start.elapsed().as_secs_f32();

        let command_pool = self.command_state().pool.clone();
        {
            let playback = self.data.ecs_world.resource_mut::<crate::ecs::AnimationPlayback>();
            if let Err(e) = self.data.graphics_resources.update_animations(
                time,
                playback,
                &self.instance,
                &self.rrdevice,
                command_pool.as_ref(),
                &mut self.data.raytracing.acceleration_structure,
            ) {
                eprintln!("failed to update animations: {}", e);
            }
        }

        let delta_time = 1.0 / 60.0;
        animation_time_system(&mut self.data.ecs_world, delta_time, &self.data.ecs_assets);
        transform_propagation_system(&mut self.data.ecs_world);

        let model = Mat4::identity();

        gui_data.mouse_pos = mouse_pos;
        gui_data.mouse_wheel = mouse_wheel;
        gui_data.update();

        let mouse_pos = Vector2::new(mouse_pos[0], mouse_pos[1]);

        let camera_pos = self.data.camera.position;
        let camera_direction = self.data.camera.direction;
        let camera_up = self.data.camera.up;

        if !gui_data.imgui_wants_mouse && gui_data.is_left_clicked {
            self.scene.light_gizmo_mut().draggable.just_selected = false;

            let is_first_click = gui_data.clicked_mouse_pos.is_none();
            if is_first_click {
                gui_data.clicked_mouse_pos = Some([mouse_pos[0], mouse_pos[1]]);

                let swapchain_extent = self.swapchain_state().swapchain.swapchain_extent;
                {
                    let mut gizmo_ref = self.scene.light_gizmo_mut();
                    let light_gizmo = &mut *gizmo_ref;
                    let position = light_gizmo.position.clone();
                    gizmo_try_select(
                        &position,
                        &mut light_gizmo.selectable,
                        &mut light_gizmo.draggable,
                        mouse_pos,
                        camera_pos,
                        camera_direction,
                        camera_up,
                        (swapchain_extent.width, swapchain_extent.height),
                        gui_data.billboard_click_rect,
                    );
                }
            }

            let (is_selected, just_selected) = {
                let gizmo = self.scene.light_gizmo();
                (gizmo.selectable.is_selected, gizmo.draggable.just_selected)
            };
            if is_selected && gui_data.is_left_clicked && !just_selected {
                self.update_light_gizmo_position(mouse_pos, camera_pos, camera_direction, gui_data);
            }
        } else if !gui_data.is_wheel_clicked {
            if gui_data.clicked_mouse_pos.is_some() {
                crate::log!("Mouse released - resetting light gizmo state");
                let mut gizmo = self.scene.light_gizmo_mut();
                gizmo.selectable.is_selected = false;
                gizmo.selectable.selected_axis = crate::ecs::GizmoAxis::None;
                gizmo.draggable.drag_axis = crate::ecs::GizmoAxis::None;
                gizmo.draggable.just_selected = false;
                gizmo.draggable.initial_position = Vector3::new(0.0, 0.0, 0.0);
            }
        }

        if !self.scene.light_gizmo().selectable.is_selected {
            let grid_scale = self.scene.grid_mut().scale;
            camera_input_system(&mut self.data.camera, gui_data, grid_scale);
        }

        let camera_pos = self.data.camera.position;
        let camera_direction = self.data.camera.direction;
        let camera_up = self.data.camera.up;
        let view = view(camera_pos, camera_direction, camera_up);

        let camera_distance = camera_pos.magnitude();
        self.scene.grid_mut().scale = 1.0;

        self.data.camera.near_plane = (camera_distance * 0.001).max(0.1).min(10.0);
        self.data.camera.far_plane = (self.scene.grid_mut().scale * 1000.0)
            .max(1000.0)
            .min(100000.0);

        use crate::math::coordinate_system::perspective;
        let swapchain_extent = self.swapchain_state().swapchain.swapchain_extent;
        let proj = perspective(
            Deg(45.0),
            swapchain_extent.width as f32 / swapchain_extent.height as f32,
            self.data.camera.near_plane,
            self.data.camera.far_plane,
        );
        let screen_size = Vector2::new(
            swapchain_extent.width as f32,
            swapchain_extent.height as f32,
        );
        let light_pos = self.data.rt_debug_state.light_position;
        let billboard_world_size = 0.5;
        let billboard_ndc_scale = 0.1;

        gui_data.billboard_click_rect = calculate_billboard_click_rect(
            light_pos,
            screen_size,
            view,
            proj,
            billboard_world_size,
            billboard_ndc_scale,
        );

        if gui_data.debug_shadow_info {
            self.log_shadow_debug_info();
            gui_data.debug_shadow_info = false;
        }

        if gui_data.debug_billboard_depth {
            use crate::debugview::{
                log_billboard_debug_info, BillboardDebugInfo, GBufferDebugInfo,
            };
            let info = BillboardDebugInfo {
                light_position: self.data.rt_debug_state.light_position,
                camera_position: self.data.camera.position,
                camera_direction: self.data.camera.direction,
                camera_up: self.data.camera.up,
                near_plane: self.data.camera.near_plane,
                far_plane: self.data.camera.far_plane,
            };
            let gbuffer_debug_info =
                self.data
                    .raytracing
                    .gbuffer
                    .as_ref()
                    .map(|gb| GBufferDebugInfo {
                        position_image_view: gb.position_image_view,
                        extent_width: gb.width,
                        extent_height: gb.height,
                    });
            log_billboard_debug_info(
                &info,
                &self.swapchain_state().swapchain,
                &self.scene.billboard().descriptor_set,
                gbuffer_debug_info.as_ref(),
                self.data.raytracing.gbuffer_sampler,
            );
            gui_data.debug_billboard_depth = false;
        }

        if self.data.rt_debug_state.should_load_cube(gui_data) {
            let cube_size = self.data.rt_debug_state.cube_size;
            let cube_position = [0.0, 0.0, 0.0];
            let command_pool = self.command_state().pool.clone();
            let swapchain = self.swapchain_state().swapchain.clone();
            if let Err(e) = replace_model_with_cube(
                &self.instance,
                &self.rrdevice,
                &mut self.data,
                &self.scene,
                &command_pool,
                &swapchain,
                cube_size,
                cube_position,
            ) {
                crate::log!("Failed to replace model with cube: {:?}", e);
            } else {
                self.data
                    .rt_debug_state
                    .set_actual_cube_top(cube_size, cube_position);
            }
            self.data.rt_debug_state.finish_cube_load();
            gui_data.load_cube = false;
        }

        let light_pos = self.data.rt_debug_state.light_position;
        let frame_ubo = FrameUBO {
            view,
            proj,
            camera_pos: cgmath::Vector4::new(camera_pos.x, camera_pos.y, camera_pos.z, 1.0),
            light_pos: cgmath::Vector4::new(light_pos.x, light_pos.y, light_pos.z, 1.0),
            light_color: cgmath::Vector4::new(1.0, 1.0, 1.0, 1.0),
        };
        if let Err(e) =
            self.data
                .graphics_resources
                .frame_set
                .update(&self.rrdevice, image_index, &frame_ubo)
        {
            eprintln!("Failed to update FrameUBO: {}", e);
        }

        if let Err(e) =
            self.data
                .graphics_resources
                .update_objects(&self.rrdevice, image_index, model)
        {
            eprintln!("Failed to update ObjectUBO: {}", e);
        }

        if let (Some(scene_buffer), Some(scene_memory)) = (
            self.data.raytracing.scene_uniform_buffer,
            self.data.raytracing.scene_uniform_buffer_memory,
        ) {
            let light_pos = &self.data.rt_debug_state.light_position;

            static mut SCENE_UNIFORM_LOG_COUNTER: u32 = 0;
            static mut PREV_LIGHT_POS: [f32; 3] = [0.0, 0.0, 0.0];
            unsafe {
                SCENE_UNIFORM_LOG_COUNTER += 1;
                let current = [light_pos.x, light_pos.y, light_pos.z];
                let changed = (current[0] - PREV_LIGHT_POS[0]).abs() > 0.1
                    || (current[1] - PREV_LIGHT_POS[1]).abs() > 0.1
                    || (current[2] - PREV_LIGHT_POS[2]).abs() > 0.1;

                if changed || SCENE_UNIFORM_LOG_COUNTER % 60 == 0 {
                    crate::log!(
                        "SceneUniformData UPDATE - light_position: ({:.2}, {:.2}, {:.2})",
                        light_pos.x,
                        light_pos.y,
                        light_pos.z
                    );
                    PREV_LIGHT_POS = current;
                }
            }

            let scene_data = SceneUniformData {
                light_position: Vec4::new(light_pos.x, light_pos.y, light_pos.z, 1.0),
                light_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
                view,
                proj,
                debug_mode: self.data.rt_debug_state.debug_view_mode.as_int(),
                shadow_strength: self.data.rt_debug_state.shadow_strength,
                enable_distance_attenuation: if self.data.rt_debug_state.enable_distance_attenuation
                {
                    1
                } else {
                    0
                },
                _padding: 0,
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

        let render_data_vec = self.scene.collect_render_data(camera_pos);
        let render_data_refs: Vec<_> = render_data_vec.iter().collect();

        if let Err(e) = update_object_ubo_system(
            &render_data_refs,
            image_index,
            &self.data.graphics_resources.objects,
            &self.rrdevice,
        ) {
            eprintln!("Failed to update object UBOs: {}", e);
        }

        gizmo_sync_position(
            &mut self.scene.light_gizmo_mut().position,
            self.data.rt_debug_state.light_position,
        );

        {
            let mut light_gizmo = self.scene.light_gizmo_mut();
            let selectable = light_gizmo.selectable.clone();
            gizmo_update_selection_color(&mut light_gizmo.mesh, &selectable);
        }
        gizmo_update_vertex_buffer(&self.scene.light_gizmo().mesh, &self.rrdevice)
            .expect("Failed to update light gizmo vertex buffer");

        let (camera_right, camera_up_gizmo, camera_forward) = get_camera_axes_from_view(view);

        let light_pos = self.scene.light_gizmo().position.position;

        {
            let mut billboard = self.scene.billboard_mut();
            if billboard.transform.is_none() {
                billboard.transform = Some(BillboardTransform::new(light_pos));
            }

            if let Some(ref mut billboard_transform) = billboard.transform {
                billboard_transform_set_position(billboard_transform, light_pos);
                billboard_transform_update_look_at(billboard_transform, camera_pos, camera_up);
            }

            let model_matrix = billboard
                .transform
                .as_ref()
                .map(|t| t.model_matrix)
                .unwrap_or(Matrix4::identity());

            for i in 0..billboard.descriptor_set.rrdata.len() {
                let rrdata = &mut billboard.descriptor_set.rrdata[i];

                let ubo_billboard = UniformBufferObject {
                    model: model_matrix,
                    view,
                    proj,
                };

                let name = format!("billboard[{}]", i);
                rrdata.rruniform_buffers[image_index].update(
                    &self.rrdevice,
                    &ubo_billboard,
                    &name,
                )?;
            }
        }

        let gizmo_rotation =
            cgmath::Matrix3::from_cols(camera_right, camera_up_gizmo, camera_forward);

        gizmo_update_rotation(&mut self.scene.gizmo_mut().mesh, &gizmo_rotation);

        // Gizmo方向確認用ログ（60フレームごと）
        static mut GIZMO_LOG_COUNTER: u32 = 0;
        unsafe {
            GIZMO_LOG_COUNTER += 1;
            if GIZMO_LOG_COUNTER % 60 == 0 {
                crate::log!(
                    "=== Gizmo Direction Debug (frame {}) ===",
                    GIZMO_LOG_COUNTER
                );
                crate::log!("Camera state:");
                crate::log!(
                    "  position: ({:.3}, {:.3}, {:.3})",
                    camera_pos.x,
                    camera_pos.y,
                    camera_pos.z
                );
                crate::log!(
                    "  direction: ({:.3}, {:.3}, {:.3})",
                    camera_direction.x,
                    camera_direction.y,
                    camera_direction.z
                );
                crate::log!(
                    "  up: ({:.3}, {:.3}, {:.3})",
                    camera_up.x,
                    camera_up.y,
                    camera_up.z
                );

                crate::log!(
                    "  right: ({:.3}, {:.3}, {:.3})",
                    camera_right.x,
                    camera_right.y,
                    camera_right.z
                );

                crate::log!("Gizmo rotation matrix (from camera vectors):");
                crate::log!(
                    "  X-axis (red):   [{:.3}, {:.3}, {:.3}] = camera right",
                    gizmo_rotation.x.x,
                    gizmo_rotation.x.y,
                    gizmo_rotation.x.z
                );
                crate::log!(
                    "  Y-axis (green): [{:.3}, {:.3}, {:.3}] = camera up",
                    gizmo_rotation.y.x,
                    gizmo_rotation.y.y,
                    gizmo_rotation.y.z
                );
                crate::log!(
                    "  Z-axis (blue):  [{:.3}, {:.3}, {:.3}] = camera direction",
                    gizmo_rotation.z.x,
                    gizmo_rotation.z.y,
                    gizmo_rotation.z.z
                );

                crate::log!("Gizmo vertices (after rotation):");
                let gizmo = self.scene.gizmo();
                crate::log!(
                    "  Origin: ({:.3}, {:.3}, {:.3})",
                    gizmo.mesh.vertices[0].pos[0],
                    gizmo.mesh.vertices[0].pos[1],
                    gizmo.mesh.vertices[0].pos[2]
                );
                crate::log!(
                    "  X-axis (red): ({:.3}, {:.3}, {:.3})",
                    gizmo.mesh.vertices[1].pos[0],
                    gizmo.mesh.vertices[1].pos[1],
                    gizmo.mesh.vertices[1].pos[2]
                );
                crate::log!(
                    "  Y-axis (green): ({:.3}, {:.3}, {:.3})",
                    gizmo.mesh.vertices[2].pos[0],
                    gizmo.mesh.vertices[2].pos[1],
                    gizmo.mesh.vertices[2].pos[2]
                );
                crate::log!(
                    "  Z-axis (blue): ({:.3}, {:.3}, {:.3})",
                    gizmo.mesh.vertices[3].pos[0],
                    gizmo.mesh.vertices[3].pos[1],
                    gizmo.mesh.vertices[3].pos[2]
                );
            }
        }

        let (old_vertex_buffer, old_vertex_buffer_memory, vertices_len, vertices_ptr) = {
            let gizmo = self.scene.gizmo();
            (
                gizmo.mesh.vertex_buffer,
                gizmo.mesh.vertex_buffer_memory,
                gizmo.mesh.vertices.len(),
                gizmo.mesh.vertices.as_ptr(),
            )
        };

        if let Some(vb) = old_vertex_buffer {
            self.rrdevice.device.destroy_buffer(vb, None);
        }
        if let Some(vbm) = old_vertex_buffer_memory {
            self.rrdevice.device.free_memory(vbm, None);
        }

        let vertex_buffer_size = (size_of::<GizmoVertex>() * vertices_len) as u64;
        let (staging_buffer, staging_buffer_memory) = create_buffer(
            &self.instance,
            &self.rrdevice,
            vertex_buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let data_ptr = self.rrdevice.device.map_memory(
            staging_buffer_memory,
            0,
            vertex_buffer_size,
            vk::MemoryMapFlags::empty(),
        )?;
        std::ptr::copy_nonoverlapping(vertices_ptr, data_ptr.cast(), vertices_len);
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
            self.command_state().pool.as_ref(),
            staging_buffer,
            vertex_buffer,
            vertex_buffer_size,
        )?;

        self.rrdevice.device.destroy_buffer(staging_buffer, None);
        self.rrdevice
            .device
            .free_memory(staging_buffer_memory, None);

        {
            let mut gizmo = self.scene.gizmo_mut();
            gizmo.mesh.vertex_buffer = Some(vertex_buffer);
            gizmo.mesh.vertex_buffer_memory = Some(vertex_buffer_memory);
        }

        Ok(())
    }

    pub unsafe fn update(&mut self, image_index: usize, gui_data: &mut GUIData) -> Result<()> {
        self.update_uniform_buffer(
            image_index,
            gui_data.mouse_pos,
            gui_data.mouse_wheel,
            gui_data,
        )?;

        let model_tops: Vec<cgmath::Vector3<f32>> = self
            .data
            .rt_debug_state
            .get_cube_top()
            .into_iter()
            .collect();

        static mut CUBE_DEBUG_COUNTER: u32 = 0;
        CUBE_DEBUG_COUNTER += 1;
        if CUBE_DEBUG_COUNTER % 60 == 1 {
            crate::log!("=== Cube Position Debug (frame {}) ===", CUBE_DEBUG_COUNTER);
            crate::log!("model_tops from get_cube_top(): {:?}", model_tops);

            if !self.data.graphics_resources.meshes.is_empty() {
                for (mesh_idx, mesh) in self.data.graphics_resources.meshes.iter().enumerate() {
                    if !mesh.vertex_data.vertices.is_empty() {
                        let mut min_x = f32::MAX;
                        let mut max_x = f32::MIN;
                        let mut min_y = f32::MAX;
                        let mut max_y = f32::MIN;
                        let mut min_z = f32::MAX;
                        let mut max_z = f32::MIN;

                        for v in &mesh.vertex_data.vertices {
                            min_x = min_x.min(v.pos.x);
                            max_x = max_x.max(v.pos.x);
                            min_y = min_y.min(v.pos.y);
                            max_y = max_y.max(v.pos.y);
                            min_z = min_z.min(v.pos.z);
                            max_z = max_z.max(v.pos.z);
                        }

                        let center_x = (min_x + max_x) / 2.0;
                        let center_z = (min_z + max_z) / 2.0;

                        crate::log!("Mesh[{}] vertex_data bounds:", mesh_idx);
                        crate::log!(
                            "  X: [{:.2}, {:.2}], Y: [{:.2}, {:.2}], Z: [{:.2}, {:.2}]",
                            min_x,
                            max_x,
                            min_y,
                            max_y,
                            min_z,
                            max_z
                        );
                        crate::log!(
                            "  Top center: ({:.2}, {:.2}, {:.2})",
                            center_x,
                            max_y,
                            center_z
                        );
                        crate::log!("  vertex count: {}", mesh.vertex_data.vertices.len());
                    }
                }
            } else {
                crate::log!("graphics_resources.meshes is empty!");
            }
            crate::log!("=====================================");
        }

        {
            let mut gizmo = self.scene.light_gizmo_mut();
            let position = gizmo.position.clone();
            gizmo_update_vertical_lines(&mut gizmo.vertical_lines, &position, &model_tops);
        }
        gizmo_update_or_create_vertical_line_buffers(
            &mut self.scene.light_gizmo_mut().vertical_lines,
            &self.instance,
            &self.rrdevice,
        )?;

        Ok(())
    }

    pub unsafe fn update_imgui_buffers(
        instance: &Instance,
        rrdevice: &RRDevice,
        data: &mut AppData,
        draw_data: &imgui::DrawData,
    ) -> Result<()> {
        if draw_data.total_vtx_count == 0 || draw_data.total_idx_count == 0 {
            return Ok(());
        }

        // Calculate required buffer sizes
        let vtx_buffer_size = (draw_data.total_vtx_count as usize
            * std::mem::size_of::<imgui::DrawVert>())
            as vk::DeviceSize;
        let idx_buffer_size = (draw_data.total_idx_count as usize
            * std::mem::size_of::<imgui::DrawIdx>())
            as vk::DeviceSize;

        // Create or resize vertex buffer if needed
        if data.imgui.vertex_buffer.is_none() || vtx_buffer_size > data.imgui.vertex_buffer_size {
            if let Some(buffer) = data.imgui.vertex_buffer {
                rrdevice.device.destroy_buffer(buffer, None);
            }
            if let Some(memory) = data.imgui.vertex_buffer_memory {
                rrdevice.device.free_memory(memory, None);
            }

            let buffer_info = vk::BufferCreateInfo::builder()
                .size(vtx_buffer_size)
                .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);

            let vertex_buffer = rrdevice.device.create_buffer(&buffer_info, None)?;
            let mem_requirements = rrdevice
                .device
                .get_buffer_memory_requirements(vertex_buffer);

            let mem_alloc_info = vk::MemoryAllocateInfo::builder()
                .allocation_size(mem_requirements.size)
                .memory_type_index(get_memory_type_index(
                    instance,
                    rrdevice.physical_device,
                    vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                    mem_requirements,
                )?);

            let vertex_buffer_memory = rrdevice.device.allocate_memory(&mem_alloc_info, None)?;
            rrdevice
                .device
                .bind_buffer_memory(vertex_buffer, vertex_buffer_memory, 0)?;

            data.imgui.vertex_buffer = Some(vertex_buffer);
            data.imgui.vertex_buffer_memory = Some(vertex_buffer_memory);
            data.imgui.vertex_buffer_size = vtx_buffer_size;
        }

        // Create or resize index buffer if needed
        if data.imgui.index_buffer.is_none() || idx_buffer_size > data.imgui.index_buffer_size {
            if let Some(buffer) = data.imgui.index_buffer {
                rrdevice.device.destroy_buffer(buffer, None);
            }
            if let Some(memory) = data.imgui.index_buffer_memory {
                rrdevice.device.free_memory(memory, None);
            }

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
            rrdevice
                .device
                .bind_buffer_memory(index_buffer, index_buffer_memory, 0)?;

            data.imgui.index_buffer = Some(index_buffer);
            data.imgui.index_buffer_memory = Some(index_buffer_memory);
            data.imgui.index_buffer_size = idx_buffer_size;
        }

        // Upload vertex data
        if let Some(vertex_buffer_memory) = data.imgui.vertex_buffer_memory {
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
        if let Some(index_buffer_memory) = data.imgui.index_buffer_memory {
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

    fn update_light_gizmo_position(
        &mut self,
        mouse_pos: Vector2<f32>,
        camera_pos: Vector3<f32>,
        camera_direction: Vector3<f32>,
        gui_data: &GUIData,
    ) {
        unsafe {
            let view = view(camera_pos, camera_direction, self.data.camera.up);
            use crate::math::coordinate_system::perspective;
            let swapchain_extent = self.swapchain_state().swapchain.swapchain_extent;
            let aspect = swapchain_extent.width as f32 / swapchain_extent.height as f32;
            let proj = perspective(Deg(45.0), aspect, 0.1, 10000.0);
            let screen_size = Vector2::new(
                swapchain_extent.width as f32,
                swapchain_extent.height as f32,
            );

            let (ray_origin, ray_direction) =
                screen_to_world_ray(mouse_pos, screen_size, view, proj);

            crate::log!(
                "update_light_gizmo_position - camera_pos: ({:.2}, {:.2}, {:.2})",
                camera_pos.x,
                camera_pos.y,
                camera_pos.z
            );
            crate::log!(
                "update_light_gizmo_position - ray_origin: ({:.2}, {:.2}, {:.2})",
                ray_origin.x,
                ray_origin.y,
                ray_origin.z
            );
            crate::log!(
                "update_light_gizmo_position - ray_direction: ({:.2}, {:.2}, {:.2})",
                ray_direction.x,
                ray_direction.y,
                ray_direction.z
            );

            let light_pos = self.data.rt_debug_state.light_position;
            let plane_point = light_pos;
            let plane_normal = -camera_direction;

            let denom = plane_normal.dot(ray_direction);

            if denom.abs() > std::f32::EPSILON {
                let t = (plane_point - ray_origin).dot(plane_normal) / denom;

                crate::log!("update_light_gizmo_position - t: {:.2}, intersection will be: ({:.2}, {:.2}, {:.2})",
                     t,
                     (ray_origin + ray_direction * t).x,
                     (ray_origin + ray_direction * t).y,
                     (ray_origin + ray_direction * t).z);

                if t >= 0.0 {
                    let intersection = ray_origin + ray_direction * t;

                    {
                        let mut gizmo = self.scene.light_gizmo_mut();
                        let draggable = gizmo.draggable.clone();
                        gizmo_update_position_with_constraint(
                            &mut gizmo.position,
                            intersection,
                            &draggable,
                            gui_data.is_ctrl_pressed,
                        );
                    }

                    self.data.rt_debug_state.light_position =
                        self.scene.light_gizmo().position.position;
                }
            }
        }
    }

    pub(crate) fn log_shadow_debug_info(&self) {
        let light_pos = self.data.rt_debug_state.light_position;
        let camera_pos = self.data.camera.position;

        crate::log!("=== Shadow Debug Info ===");
        crate::log!(
            "Light position (rt_debug_state): ({:.2}, {:.2}, {:.2})",
            light_pos.x,
            light_pos.y,
            light_pos.z
        );
        crate::log!(
            "Light gizmo position: ({:.2}, {:.2}, {:.2})",
            self.scene.light_gizmo().position.position.x,
            self.scene.light_gizmo().position.position.y,
            self.scene.light_gizmo().position.position.z
        );
        crate::log!(
            "Camera position: ({:.2}, {:.2}, {:.2})",
            camera_pos.x,
            camera_pos.y,
            camera_pos.z
        );

        crate::log!("Shadow settings:");
        crate::log!(
            "  strength: {:.2}",
            self.data.rt_debug_state.shadow_strength
        );
        crate::log!(
            "  normal_offset: {:.2}",
            self.data.rt_debug_state.shadow_normal_offset
        );
        crate::log!(
            "  debug_view_mode: {:?}",
            self.data.rt_debug_state.debug_view_mode
        );
        crate::log!(
            "  distance_attenuation: {}",
            self.data.rt_debug_state.enable_distance_attenuation
        );

        if let Some(ref accel_struct) = self.data.raytracing.acceleration_structure {
            crate::log!("Acceleration Structure:");
            crate::log!("  BLAS count: {}", accel_struct.blas_list.len());
            for (i, blas) in accel_struct.blas_list.iter().enumerate() {
                crate::log!(
                    "    BLAS[{}]: AS={:?}, device_addr={:#x}",
                    i,
                    blas.acceleration_structure.is_some(),
                    blas.device_address
                );
            }
            crate::log!(
                "  TLAS: AS={:?}",
                accel_struct.tlas.acceleration_structure.is_some()
            );
        } else {
            crate::log!("WARNING: No acceleration structure!");
        }

        crate::log!("Vertex buffers (GPU):");
        for (i, mesh) in self.data.graphics_resources.meshes.iter().enumerate() {
            crate::log!(
                "  Mesh[{}]: {} vertices, {} indices",
                i,
                mesh.vertex_data.vertices.len(),
                mesh.vertex_data.indices.len()
            );
            if !mesh.vertex_data.vertices.is_empty() {
                let v = &mesh.vertex_data.vertices[0];
                crate::log!(
                    "    vertex[0].pos: ({:.2}, {:.2}, {:.2})",
                    v.pos.x,
                    v.pos.y,
                    v.pos.z
                );
                crate::log!(
                    "    vertex[0].normal: ({:.3}, {:.3}, {:.3})",
                    v.normal.x,
                    v.normal.y,
                    v.normal.z
                );
            }
        }

        crate::log!("=========================");
    }
}
