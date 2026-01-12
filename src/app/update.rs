use crate::app::model_loader::replace_model_with_cube;
use crate::app::{App, AppData, GUIData};
use crate::debugview::*;
use crate::math::*;
use crate::scene::billboard::BillboardTransform;
use crate::scene::render_resource::{FrameUBO, ObjectUBO};
use crate::vulkanr::buffer::*;
use crate::vulkanr::data::*;
use crate::vulkanr::device::*;
use crate::vulkanr::vulkan::*;

use anyhow::Result;
use cgmath::{Deg, InnerSpace, Matrix4, Vector2, Vector3};
use std::mem::size_of;
use vulkanalia::prelude::v1_0::*;

impl App {
    pub(crate) unsafe fn update_uniform_buffer(
        &mut self,
        image_index: usize,
        mouse_pos: [f32; 2],
        mouse_wheel: f32,
        gui_data: &mut GUIData,
    ) -> Result<()> {
        use crate::app::data::LightMoveTarget;

        if gui_data.move_light_to != LightMoveTarget::None {
            let all_positions: Vec<Vector3<f32>> = if !self.data.render_resources.meshes.is_empty()
            {
                self.data
                    .render_resources
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
                self.data.camera.position(),
                gui_data.move_light_to,
            );
            gui_data.move_light_to = LightMoveTarget::None;
        }

        let time = self.start.elapsed().as_secs_f32();

        if let Err(e) = self.data.render_resources.update_animations(
            time,
            &self.instance,
            &self.rrdevice,
            &self.data.rrcommand_pool,
            &mut self.data.raytracing.acceleration_structure,
        ) {
            eprintln!("failed to update animations: {}", e);
        }

        let model = Mat4::identity();

        gui_data.mouse_pos = mouse_pos;
        gui_data.mouse_wheel = mouse_wheel;
        gui_data.update();

        let mouse_pos = Vector2::new(mouse_pos[0], mouse_pos[1]);

        let camera_pos = self.data.camera.position();
        let camera_direction = self.data.camera.direction();
        let camera_up = self.data.camera.up();

        if !gui_data.imgui_wants_mouse && gui_data.is_left_clicked {
            self.data.light_gizmo_data.just_selected = false;

            let is_first_click = gui_data.clicked_mouse_pos.is_none();
            if is_first_click {
                gui_data.clicked_mouse_pos = Some([mouse_pos[0], mouse_pos[1]]);

                let swapchain_extent = self.data.rrswapchain.swapchain_extent;
                self.data.light_gizmo_data.try_select(
                    mouse_pos,
                    camera_pos,
                    camera_direction,
                    camera_up,
                    (swapchain_extent.width, swapchain_extent.height),
                    self.data.rt_debug_state.light_position,
                    gui_data.billboard_click_rect,
                );
            }
            let clicked_mouse_pos: Vector2<f32> = gui_data
                .clicked_mouse_pos
                .map(|p| p.to_vec2().into())
                .unwrap_or(mouse_pos);

            if self.data.light_gizmo_data.is_selected
                && gui_data.is_left_clicked
                && !self.data.light_gizmo_data.just_selected
            {
                self.update_light_gizmo_position(mouse_pos, camera_pos, camera_direction, gui_data);
            }
        } else if !gui_data.is_wheel_clicked {
            if gui_data.clicked_mouse_pos.is_some() {
                crate::log!("Mouse released - resetting light gizmo state");
                self.data.light_gizmo_data.set_default();
            }
        }

        if !self.data.light_gizmo_data.is_selected {
            self.data.camera.update(gui_data, self.data.grid.scale);
        }

        let camera_pos = self.data.camera.position();
        let camera_direction = self.data.camera.direction();
        let camera_up = self.data.camera.up();
        let view = view(camera_pos, camera_direction, camera_up);

        let camera_distance = camera_pos.magnitude();
        let base_scale = 10.0;
        //self.data.grid.scale = (camera_distance / base_scale).max(1.0);
        self.data.grid.scale = 1.0;

        let near_plane = (camera_distance * 0.001).max(0.1).min(10.0);
        let far_plane = (self.data.grid.scale * 1000.0).max(1000.0).min(100000.0);
        self.data.camera.set_near_plane(near_plane);
        self.data.camera.set_far_plane(far_plane);

        use crate::math::coordinate_system::perspective;
        let proj = perspective(
            Deg(45.0),
            self.data.rrswapchain.swapchain_extent.width as f32
                / self.data.rrswapchain.swapchain_extent.height as f32,
            self.data.camera.near_plane(),
            self.data.camera.far_plane(),
        );

        let swapchain_extent = self.data.rrswapchain.swapchain_extent;
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
                camera_position: self.data.camera.position(),
                camera_direction: self.data.camera.direction(),
                camera_up: self.data.camera.up(),
                near_plane: self.data.camera.near_plane(),
                far_plane: self.data.camera.far_plane(),
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
                &self.data.rrswapchain,
                &self.data.billboard.descriptor_set,
                gbuffer_debug_info.as_ref(),
                self.data.raytracing.gbuffer_sampler,
            );
            gui_data.debug_billboard_depth = false;
        }

        if self.data.rt_debug_state.should_load_cube(gui_data) {
            let cube_size = self.data.rt_debug_state.cube_size;
            let cube_position = [0.0, 0.0, 0.0];
            if let Err(e) = replace_model_with_cube(
                &self.instance,
                &self.rrdevice,
                &mut self.data,
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

        let ubo = UniformBufferObject { model, view, proj };

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
                .render_resources
                .frame_set
                .update(&self.rrdevice, image_index, &frame_ubo)
        {
            eprintln!("Failed to update FrameUBO: {}", e);
        }

        if let Err(e) =
            self.data
                .render_resources
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

        let model_grid = Mat4::from_scale(self.data.grid.scale);
        let grid_object_ubo = ObjectUBO { model: model_grid };
        if let Err(e) = self.data.render_resources.objects.update(
            &self.rrdevice,
            image_index,
            self.data.grid.object_index,
            &grid_object_ubo,
        ) {
            eprintln!("Failed to update Grid ObjectUBO: {}", e);
        }

        let gizmo_object_ubo = ObjectUBO {
            model: Mat4::identity(),
        };
        if let Err(e) = self.data.render_resources.objects.update(
            &self.rrdevice,
            image_index,
            self.data.gizmo_data.object_index,
            &gizmo_object_ubo,
        ) {
            eprintln!("Failed to update Gizmo ObjectUBO: {}", e);
        }

        let light_pos = self.data.rt_debug_state.light_position;
        let distance = (light_pos - camera_pos).magnitude();
        let scale_factor = distance * 0.03;
        let light_gizmo_model = Mat4::from_translation(light_pos) * Mat4::from_scale(scale_factor);
        let light_gizmo_object_ubo = ObjectUBO {
            model: light_gizmo_model,
        };
        if let Err(e) = self.data.render_resources.objects.update(
            &self.rrdevice,
            image_index,
            self.data.light_gizmo_data.object_index,
            &light_gizmo_object_ubo,
        ) {
            eprintln!("Failed to update LightGizmo ObjectUBO: {}", e);
        }

        self.data
            .light_gizmo_data
            .sync_from_debug_state(self.data.rt_debug_state.light_position);

        self.data.light_gizmo_data.update_selection_color();
        self.data
            .light_gizmo_data
            .update_vertex_buffer(&self.rrdevice)
            .expect("Failed to update light gizmo vertex buffer");

        let (camera_right, camera_up_gizmo, camera_forward) = get_camera_axes_from_view(view);

        // ビルボード用のuniform bufferを更新
        let light_pos = self.data.light_gizmo_data.position;

        if self.data.billboard.transform.is_none() {
            self.data.billboard.transform = Some(BillboardTransform::new(light_pos));
        }

        if let Some(ref mut billboard_transform) = self.data.billboard.transform {
            billboard_transform.set_position(light_pos);
            billboard_transform.update_look_at(camera_pos, camera_up);

            for i in 0..self.data.billboard.descriptor_set.rrdata.len() {
                let rrdata = &mut self.data.billboard.descriptor_set.rrdata[i];

                let ubo_billboard = UniformBufferObject {
                    model: billboard_transform.model_matrix,
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

        // Gizmoの頂点を更新
        self.data.gizmo_data.update_rotation(&gizmo_rotation);

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
                crate::log!(
                    "  Origin: ({:.3}, {:.3}, {:.3})",
                    self.data.gizmo_data.vertices[0].pos[0],
                    self.data.gizmo_data.vertices[0].pos[1],
                    self.data.gizmo_data.vertices[0].pos[2]
                );
                crate::log!(
                    "  X-axis (red): ({:.3}, {:.3}, {:.3})",
                    self.data.gizmo_data.vertices[1].pos[0],
                    self.data.gizmo_data.vertices[1].pos[1],
                    self.data.gizmo_data.vertices[1].pos[2]
                );
                crate::log!(
                    "  Y-axis (green): ({:.3}, {:.3}, {:.3})",
                    self.data.gizmo_data.vertices[2].pos[0],
                    self.data.gizmo_data.vertices[2].pos[1],
                    self.data.gizmo_data.vertices[2].pos[2]
                );
                crate::log!(
                    "  Z-axis (blue): ({:.3}, {:.3}, {:.3})",
                    self.data.gizmo_data.vertices[3].pos[0],
                    self.data.gizmo_data.vertices[3].pos[1],
                    self.data.gizmo_data.vertices[3].pos[2]
                );
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
        let vertex_buffer_size =
            (size_of::<GizmoVertex>() * self.data.gizmo_data.vertices.len()) as u64;
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
        std::ptr::copy_nonoverlapping(
            self.data.gizmo_data.vertices.as_ptr(),
            data_ptr.cast(),
            self.data.gizmo_data.vertices.len(),
        );
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
        self.rrdevice
            .device
            .free_memory(staging_buffer_memory, None);

        self.data.gizmo_data.vertex_buffer = Some(vertex_buffer);
        self.data.gizmo_data.vertex_buffer_memory = Some(vertex_buffer_memory);

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
            let view = view(camera_pos, camera_direction, self.data.camera.up());
            use crate::math::coordinate_system::perspective;
            let swapchain_extent = self.data.rrswapchain.swapchain_extent;
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
                    let initial_pos = self.data.light_gizmo_data.initial_position.to_vec3();

                    self.data.light_gizmo_data.update_position_with_constraint(
                        intersection,
                        initial_pos.into(),
                        gui_data.is_ctrl_pressed,
                    );

                    self.data.rt_debug_state.light_position = self.data.light_gizmo_data.position;
                }
            }
        }
    }

    pub(crate) fn log_shadow_debug_info(&self) {
        let light_pos = self.data.rt_debug_state.light_position;
        let camera_pos = self.data.camera.position();

        crate::log!("=== Shadow Debug Info ===");
        crate::log!(
            "Light position (rt_debug_state): ({:.2}, {:.2}, {:.2})",
            light_pos.x,
            light_pos.y,
            light_pos.z
        );
        crate::log!(
            "Light gizmo position: ({:.2}, {:.2}, {:.2})",
            self.data.light_gizmo_data.position.x,
            self.data.light_gizmo_data.position.y,
            self.data.light_gizmo_data.position.z
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
        for (i, mesh) in self.data.render_resources.meshes.iter().enumerate() {
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
