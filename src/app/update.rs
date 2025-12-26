use crate::app::{App, AppData, GUIData};
use rust_rendering::vulkanr::buffer::*;
use rust_rendering::vulkanr::command::*;
use rust_rendering::vulkanr::data as vulkan_data;
use rust_rendering::vulkanr::data::*;
use rust_rendering::vulkanr::descriptor::*;
use rust_rendering::vulkanr::device::*;
use rust_rendering::vulkanr::image::*;
use rust_rendering::vulkanr::vulkan::*;
use rust_rendering::math::math::*;
use rust_rendering::logger::logger::*;
use rust_rendering::debugview::*;

use cgmath::{Vector2, Vector3, Deg, Rad, Matrix4, Matrix3, InnerSpace};
use anyhow::Result;
use std::mem::size_of;
use std::ptr::copy_nonoverlapping as memcpy;
use std::os::raw::c_void;
use vulkanalia::prelude::v1_0::*;

impl App {
    pub(crate) unsafe fn update_uniform_buffer(
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

        // Only process camera rotation if ImGui doesn't want the mouse
        if !gui_data.imgui_wants_mouse && (gui_data.is_left_clicked || self.data.is_left_clicked) {
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
        } else if gui_data.imgui_wants_mouse {
            // If ImGui wants the mouse, reset camera operation state
            self.data.is_left_clicked = false;
        }

        // Only process camera pan if ImGui doesn't want the mouse
        if !gui_data.imgui_wants_mouse && (gui_data.is_wheel_clicked || self.data.is_wheel_clicked) {
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
        } else if gui_data.imgui_wants_mouse {
            // If ImGui wants the mouse, reset camera operation state
            self.data.is_wheel_clicked = false;
        }

        // Only process mouse wheel zoom if ImGui doesn't want the mouse
        if !gui_data.imgui_wants_mouse && mouse_wheel != 0.0 {
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
            let light_pos = &self.data.rt_debug_state.light_position;
            let scene_data = SceneUniformData {
                light_position: Vec4::new(light_pos.x, light_pos.y, light_pos.z, 1.0),
                light_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
                view,
                proj,
                debug_mode: self.data.rt_debug_state.debug_view_mode.as_int(),
                shadow_strength: self.data.rt_debug_state.shadow_strength,
                _padding: [0; 2],
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
                (size_of::<vulkan_data::Vertex>() * vertices.len()) as vk::DeviceSize,
                vertices.as_ptr() as *const c_void,
                vertices.len(),
            ) {
                eprintln!("failed to update vertex buffer: {}", e);
            }
        }
    }
    pub(crate) unsafe fn reload_model_data_buffer(
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
                (size_of::<vulkan_data::Vertex>() * rrdata.vertex_data.vertices.len())
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
}
