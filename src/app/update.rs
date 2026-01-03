use crate::app::{App, AppData, GUIData};
use rust_rendering::scene::billboard::BillboardTransform;
use rust_rendering::vulkanr::buffer::*;
use rust_rendering::vulkanr::command::*;
use rust_rendering::vulkanr::data as vulkan_data;
use rust_rendering::vulkanr::data::*;
use rust_rendering::vulkanr::descriptor::*;
use rust_rendering::vulkanr::device::*;
use rust_rendering::vulkanr::image::*;
use rust_rendering::vulkanr::vulkan::*;
use rust_rendering::math::*;
use rust_rendering::logger::logger::*;
use rust_rendering::debugview::*;
use rust_rendering::vulkanr::raytracing::acceleration::RRAccelerationStructure;

use cgmath::{Vector2, Vector3, Deg, Matrix4, InnerSpace};
use anyhow::Result;
use std::mem::size_of;
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
        use crate::app::data::LightMoveTarget;

        if gui_data.move_light_to != LightMoveTarget::None {
            log!("========================================");
            log!("LIGHT MOVE BUTTON PRESSED: {:?}", gui_data.move_light_to);
            log!("========================================");

            let all_positions: Vec<Vector3<f32>> = if !self.data.fbx_model.fbx_data.is_empty() {
                self.data.fbx_model.fbx_data
                    .iter()
                    .flat_map(|data| data.positions.iter())
                    .cloned()
                    .collect()
            } else if !self.data.gltf_model.gltf_data.is_empty() {
                self.data.gltf_model.gltf_data
                    .iter()
                    .flat_map(|data| data.vertices.iter().map(|v| {
                        Vector3::new(
                            v.animation_position[0],
                            v.animation_position[1],
                            v.animation_position[2],
                        )
                    }))
                    .collect()
            } else {
                self.data.model_descriptor_set.rrdata
                    .iter()
                    .flat_map(|rrdata| rrdata.vertex_data.vertices.iter().map(|v| {
                        Vector3::new(v.pos.x, v.pos.y, v.pos.z)
                    }))
                    .collect()
            };

            if !all_positions.is_empty() {
                let mut min_x = f32::MAX;
                let mut max_x = f32::MIN;
                let mut min_y = f32::MAX;
                let mut max_y = f32::MIN;
                let mut min_z = f32::MAX;
                let mut max_z = f32::MIN;

                for pos in all_positions.iter() {
                    min_x = min_x.min(pos.x);
                    max_x = max_x.max(pos.x);
                    min_y = min_y.min(pos.y);
                    max_y = max_y.max(pos.y);
                    min_z = min_z.min(pos.z);
                    max_z = max_z.max(pos.z);
                }

                let size_x = (max_x - min_x).abs();
                let size_y = (max_y - min_y).abs();
                let size_z = (max_z - min_z).abs();
                let model_size = (size_x + size_y + size_z) / 3.0;

                let offset = 2.0;
                let current_pos = self.data.rt_debug_state.light_position;
                let new_light_pos = match gui_data.move_light_to {
                    LightMoveTarget::XMin => Vector3::new(min_x - offset, current_pos.y, current_pos.z),
                    LightMoveTarget::XMax => Vector3::new(max_x + offset, current_pos.y, current_pos.z),
                    LightMoveTarget::YMin => Vector3::new(current_pos.x, min_y - offset, current_pos.z),
                    LightMoveTarget::YMax => Vector3::new(current_pos.x, max_y + offset, current_pos.z),
                    LightMoveTarget::ZMin => Vector3::new(current_pos.x, current_pos.y, min_z - offset),
                    LightMoveTarget::ZMax => Vector3::new(current_pos.x, current_pos.y, max_z + offset),
                    LightMoveTarget::None => current_pos,
                };

                self.data.rt_debug_state.shadow_normal_offset = (model_size * 0.005).max(0.5);

                log!("=== LIGHT POSITION DEBUG ===");
                log!("Model size: {:.2}, Shadow normal offset: {:.2}", model_size, self.data.rt_debug_state.shadow_normal_offset);
                log!("Model bounds: X[{:.2}, {:.2}], Y[{:.2}, {:.2}], Z[{:.2}, {:.2}]",
                    min_x, max_x, min_y, max_y, min_z, max_z);
                log!("Model center: ({:.2}, {:.2}, {:.2})",
                    (min_x + max_x) / 2.0, (min_y + max_y) / 2.0, (min_z + max_z) / 2.0);
                log!("Calculated light position: ({:.2}, {:.2}, {:.2})",
                    new_light_pos.x, new_light_pos.y, new_light_pos.z);
                log!("CAMERA position: ({:.2}, {:.2}, {:.2})",
                    self.data.camera.position().x, self.data.camera.position().y, self.data.camera.position().z);

                let closest_vertex = all_positions.iter()
                    .min_by(|a, b| {
                        let dist_a = (new_light_pos - **a).magnitude();
                        let dist_b = (new_light_pos - **b).magnitude();
                        dist_a.partial_cmp(&dist_b).unwrap()
                    });
                let farthest_vertex = all_positions.iter()
                    .max_by(|a, b| {
                        let dist_a = (new_light_pos - **a).magnitude();
                        let dist_b = (new_light_pos - **b).magnitude();
                        dist_a.partial_cmp(&dist_b).unwrap()
                    });

                if let Some(closest) = closest_vertex {
                    let dist = (new_light_pos - *closest).magnitude();
                    log!("Closest vertex to light: ({:.2}, {:.2}, {:.2}), distance: {:.2}",
                        closest.x, closest.y, closest.z, dist);
                }
                if let Some(farthest) = farthest_vertex {
                    let dist = (new_light_pos - *farthest).magnitude();
                    log!("Farthest vertex from light: ({:.2}, {:.2}, {:.2}), distance: {:.2}",
                        farthest.x, farthest.y, farthest.z, dist);
                }

                match gui_data.move_light_to {
                    LightMoveTarget::XMax => {
                        log!("XMax: Light should be to the RIGHT of all vertices");
                        log!("  Light X: {:.2}, Model X range: [{:.2}, {:.2}]", new_light_pos.x, min_x, max_x);
                        if new_light_pos.x <= max_x {
                            log!("  WARNING: Light X ({:.2}) is NOT greater than max X ({:.2})!", new_light_pos.x, max_x);
                        } else {
                            log!("  OK: Light X ({:.2}) > max X ({:.2})", new_light_pos.x, max_x);
                        }
                    }
                    _ => {}
                }

                self.data.rt_debug_state.light_position = new_light_pos;

                log!("Light position SET in rt_debug_state: ({:.2}, {:.2}, {:.2})",
                    self.data.rt_debug_state.light_position.x,
                    self.data.rt_debug_state.light_position.y,
                    self.data.rt_debug_state.light_position.z);
                log!("(light_gizmo_data will be synced later in this frame)");
                log!("========================================");
            } else {
                log!("WARNING: No model positions found!");
            }

            gui_data.move_light_to = LightMoveTarget::None;
        }

        self.morphing(self.start.elapsed().as_secs_f32());

        let model = Mat4::identity();

        let mut camera_pos = self.data.camera.position();
        let mut camera_direction = self.data.camera.direction();
        let mut camera_up = self.data.camera.up();

        let mouse_pos = Vector2::new(mouse_pos[0], mouse_pos[1]);

        let camera_right = camera_up.cross(camera_direction).normalize();

        let last_view = view(camera_pos, camera_direction, camera_up);
        let base_x = camera_right;
        let base_y = camera_up;

        // Camera rotation logging counter
        static mut ROTATION_LOG_COUNTER: u32 = 0;

        // Only process camera rotation if ImGui doesn't want the mouse
        if !gui_data.imgui_wants_mouse && (gui_data.is_left_clicked || self.data.is_left_clicked) {
            self.data.light_gizmo_data.just_selected = false;

            // first clicked
            if !self.data.is_left_clicked {
                self.data.clicked_mouse_pos = [mouse_pos[0], mouse_pos[1]];
                self.data.is_left_clicked = true;

                use rust_rendering::math::coordinate_system::vulkan_projection_correction;
                let view = view(camera_pos, camera_direction, camera_up);
                let swapchain_extent = self.data.rrswapchain.swapchain_extent;
                let aspect = swapchain_extent.width as f32 / swapchain_extent.height as f32;
                let proj = vulkan_projection_correction() * cgmath::perspective(Deg(45.0), aspect, 0.1, 10000.0);
                let screen_size = Vector2::new(swapchain_extent.width as f32, swapchain_extent.height as f32);

                let (ray_origin, ray_direction) = screen_to_world_ray(mouse_pos, screen_size, view, proj);

                let light_pos = self.data.rt_debug_state.light_position;
                let distance = (light_pos - camera_pos).magnitude();
                let scale_factor = distance * 0.03;

                let billboard_clicked = if let Some(rect) = gui_data.billboard_click_rect {
                    is_point_in_rect(mouse_pos, rect)
                } else {
                    false
                };

                let center_distance = ray_to_point_distance(ray_origin, ray_direction, light_pos);

                let x_axis_start = light_pos;
                let x_axis_end = light_pos + vec3(1.0 * scale_factor, 0.0, 0.0);
                let y_axis_start = light_pos;
                let y_axis_end = light_pos + vec3(0.0, 1.0 * scale_factor, 0.0);
                let z_axis_start = light_pos;
                let z_axis_end = light_pos + vec3(0.0, 0.0, 1.0 * scale_factor);

                let x_distance = ray_to_line_segment_distance(ray_origin, ray_direction, x_axis_start, x_axis_end);
                let y_distance = ray_to_line_segment_distance(ray_origin, ray_direction, y_axis_start, y_axis_end);
                let z_distance = ray_to_line_segment_distance(ray_origin, ray_direction, z_axis_start, z_axis_end);

                let threshold = 0.05 * scale_factor;

                let mut min_distance = center_distance;
                let mut selected_axis = LightGizmoAxis::None;

                if billboard_clicked {
                    selected_axis = LightGizmoAxis::Center;
                    min_distance = 0.0;
                } else {
                    if center_distance < threshold {
                        selected_axis = LightGizmoAxis::Center;
                    }

                    if x_distance < threshold && x_distance < min_distance {
                        min_distance = x_distance;
                        selected_axis = LightGizmoAxis::X;
                    }

                    if y_distance < threshold && y_distance < min_distance {
                        min_distance = y_distance;
                        selected_axis = LightGizmoAxis::Y;
                    }

                    if z_distance < threshold && z_distance < min_distance {
                        min_distance = z_distance;
                        selected_axis = LightGizmoAxis::Z;
                    }
                }

                if selected_axis != LightGizmoAxis::None {
                    self.data.light_gizmo_data.is_selected = true;
                    self.data.light_gizmo_data.drag_axis = selected_axis;
                    self.data.light_gizmo_data.selected_axis = selected_axis;

                    let light_pos = self.data.rt_debug_state.light_position;
                    self.data.light_gizmo_data.initial_position = [light_pos.x, light_pos.y, light_pos.z];

                    let drag_depth = (light_pos - camera_pos).magnitude();
                    log!("Light gizmo selected - axis: {:?}, depth: {:.2}", selected_axis, drag_depth);

                    self.data.light_gizmo_data.just_selected = true;
                }
            }
            let clicked_mouse_pos = vec2_from_array(self.data.clicked_mouse_pos);

            if self.data.light_gizmo_data.is_selected && gui_data.is_left_clicked && !self.data.light_gizmo_data.just_selected {
                self.update_light_gizmo_position(mouse_pos, camera_pos, camera_direction, gui_data);
            } else if !self.data.light_gizmo_data.is_selected {
                let diff = mouse_pos - clicked_mouse_pos;
                let distance = Vector2::distance(mouse_pos, clicked_mouse_pos);
                gui_data.monitor_value = distance;
                if 0.001 < distance {
                    unsafe {
                        ROTATION_LOG_COUNTER += 1;
                        if ROTATION_LOG_COUNTER % 30 == 0 {
                            log!("=== Camera Rotation Debug (frame {}) ===", ROTATION_LOG_COUNTER);
                            log!("  Mouse diff: ({:.3}, {:.3})", diff.x, diff.y);
                            log!("  Before rotation:");
                            log!("    direction: ({:.3}, {:.3}, {:.3})",
                                 camera_direction.x, camera_direction.y, camera_direction.z);
                            log!("    up: ({:.3}, {:.3}, {:.3})",
                                 camera_up.x, camera_up.y, camera_up.z);
                        }
                    }

                    let (new_direction, new_up) = self.data.camera.rotate(diff);
                    camera_direction = new_direction;
                    camera_up = new_up;

                    unsafe {
                        if ROTATION_LOG_COUNTER % 30 == 0 {
                            log!("  After rotation:");
                            log!("    direction: ({:.3}, {:.3}, {:.3})",
                                 camera_direction.x, camera_direction.y, camera_direction.z);
                            log!("    up: ({:.3}, {:.3}, {:.3})",
                                 camera_up.x, camera_up.y, camera_up.z);
                        }
                    }

                    self.data.clicked_mouse_pos = [mouse_pos[0], mouse_pos[1]];
                }
            }

            if !gui_data.is_left_clicked {
                if self.data.is_left_clicked {
                    log!("Mouse released - resetting light gizmo state");
                }
                self.data.is_left_clicked = false;
                self.data.light_gizmo_data.is_selected = false;
                self.data.light_gizmo_data.drag_axis = LightGizmoAxis::None;
                self.data.light_gizmo_data.selected_axis = LightGizmoAxis::None;
                self.data.light_gizmo_data.just_selected = false;
                self.data.light_gizmo_data.initial_position = [0.0, 0.0, 0.0];
            }
        } else if gui_data.imgui_wants_mouse {
            // If ImGui wants the mouse, reset camera operation state
            self.data.is_left_clicked = false;
        }

        // Only process camera pan if ImGui doesn't want the mouse
        if !gui_data.imgui_wants_mouse && (gui_data.is_wheel_clicked || self.data.is_wheel_clicked) {
            if !self.data.is_wheel_clicked {
                self.data.clicked_mouse_pos = [mouse_pos[0], mouse_pos[1]];
                self.data.is_wheel_clicked = true;
            }
            let clicked_mouse_pos = vec2_from_array(self.data.clicked_mouse_pos);

            let diff = mouse_pos - clicked_mouse_pos;
            let distance = Vector2::distance(mouse_pos, clicked_mouse_pos);
            gui_data.monitor_value = distance;
            if 0.001 < distance {
                let pan_speed = self.data.grid.scale * 0.01;
                self.data.camera.pan_with_base(diff, base_x, base_y, pan_speed);
                camera_pos = self.data.camera.position();
                self.data.clicked_mouse_pos = [mouse_pos[0], mouse_pos[1]];
            }

            if !gui_data.is_wheel_clicked {
                self.data.is_wheel_clicked = false;
            }
        } else if gui_data.imgui_wants_mouse {
            self.data.is_wheel_clicked = false;
        }

        let view = view(camera_pos, camera_direction, camera_up);

        let camera_distance = camera_pos.magnitude();
        let base_scale = 10.0;
        self.data.grid.scale = (camera_distance / base_scale).max(1.0);

        let near_plane = (camera_distance * 0.001).max(0.1).min(10.0);
        let far_plane = (self.data.grid.scale * 1000.0).max(1000.0).min(100000.0);
        self.data.camera.set_near_plane(near_plane);
        self.data.camera.set_far_plane(far_plane);

        use rust_rendering::math::coordinate_system::vulkan_projection_correction;
        let proj = vulkan_projection_correction()
            * cgmath::perspective(
            Deg(45.0),
            self.data.rrswapchain.swapchain_extent.width as f32
                / self.data.rrswapchain.swapchain_extent.height as f32,
            self.data.camera.near_plane(),
            self.data.camera.far_plane(),
        );

        if !gui_data.imgui_wants_mouse && mouse_wheel != 0.0 {
            let zoom_speed = self.data.grid.scale * 0.5;
            self.data.camera.zoom(mouse_wheel, zoom_speed);
            camera_pos = self.data.camera.position();
        }

        let swapchain_extent = self.data.rrswapchain.swapchain_extent;
        let screen_size = Vector2::new(swapchain_extent.width as f32, swapchain_extent.height as f32);
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
            use rust_rendering::debugview::{BillboardDebugInfo, GBufferDebugInfo, log_billboard_debug_info};
            let info = BillboardDebugInfo {
                light_position: self.data.rt_debug_state.light_position,
                camera_position: self.data.camera.position(),
                camera_direction: self.data.camera.direction(),
                camera_up: self.data.camera.up(),
                near_plane: self.data.camera.near_plane(),
                far_plane: self.data.camera.far_plane(),
            };
            let gbuffer_debug_info = self.data.raytracing.gbuffer.as_ref().map(|gb| GBufferDebugInfo {
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

        let should_load_cube = gui_data.load_cube || self.data.rt_debug_state.cube_size_changed;
        if should_load_cube {
            let cube_size = self.data.rt_debug_state.cube_size;
            let cube_position = [0.0, 0.0, cube_size];
            log!("Loading cube model with size {}...", cube_size);
            match crate::app::model_loader::replace_model_with_cube(
                &self.instance,
                &self.rrdevice,
                &mut self.data,
                cube_size,
                cube_position,
            ) {
                Ok(_) => {
                    self.data.rt_debug_state.set_actual_cube_top(cube_size, cube_position);
                    log!("Cube model loaded successfully");
                }
                Err(e) => {
                    log!("Failed to load cube model: {}", e);
                }
            }
            gui_data.load_cube = false;
            self.data.rt_debug_state.cube_size_changed = false;
        }

        let ubo = UniformBufferObject { model, view, proj };

        for i in 0..self.data.model_descriptor_set.rrdata.len() {
            let rrdata = &mut self.data.model_descriptor_set.rrdata[i];
            let name = format!("model[{}]", i);
            if let Err(e) = rrdata.rruniform_buffers[image_index].update(&self.rrdevice, &ubo, &name) {
                eprintln!("Failed to update model UBO: {}", e);
            }
        }

        if let Some(ref mut gbuffer_desc) = self.data.raytracing.gbuffer_descriptor_set {
            for i in 0..gbuffer_desc.rrdata.len() {
                let rrdata = &mut gbuffer_desc.rrdata[i];
                let name = format!("gbuffer[{}]", i);
                if let Err(e) = rrdata.rruniform_buffers[image_index].update(&self.rrdevice, &ubo, &name) {
                    eprintln!("Failed to update G-Buffer UBO: {}", e);
                }
            }
        }

        if let (Some(scene_buffer), Some(scene_memory)) =
            (self.data.raytracing.scene_uniform_buffer, self.data.raytracing.scene_uniform_buffer_memory)
        {
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
                    log!("SceneUniformData UPDATE - light_position: ({:.2}, {:.2}, {:.2})",
                        light_pos.x, light_pos.y, light_pos.z);
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
                enable_distance_attenuation: if self.data.rt_debug_state.enable_distance_attenuation { 1 } else { 0 },
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
        for i in 0..self.data.grid.descriptor_set.rrdata.len() {
            let rrdata = &mut self.data.grid.descriptor_set.rrdata[i];

            let model = if i == 0 {
                model_grid
            } else {
                Mat4::identity()
            };

            let ubo_grid = UniformBufferObject { model, view, proj };
            let name = format!("grid[{}]", i);
            rrdata.rruniform_buffers[image_index].update(&self.rrdevice, &ubo_grid, &name)?;
        }

        // Gizmo用のuniform bufferを更新
        for i in 0..self.data.gizmo_data.descriptor_set.rrdata.len() {
            let rrdata = &mut self.data.gizmo_data.descriptor_set.rrdata[i];

            let model = if i == 0 {
                Mat4::identity()
            } else {
                let light_pos = self.data.rt_debug_state.light_position;
                let distance = (light_pos - camera_pos).magnitude();
                let scale_factor = distance * 0.03;
                Mat4::from_translation(light_pos) * Mat4::from_scale(scale_factor)
            };

            let ubo_gizmo = UniformBufferObject { model, view, proj };
            let name = format!("gizmo[{}]", i);
            rrdata.rruniform_buffers[image_index].update(&self.rrdevice, &ubo_gizmo, &name)?;
        }

        self.data.light_gizmo_data.sync_from_debug_state(self.data.rt_debug_state.light_position);

        self.data.light_gizmo_data.update_selection_color();
        self.data.light_gizmo_data.update_vertex_buffer(&self.rrdevice)
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
                rrdata.rruniform_buffers[image_index].update(&self.rrdevice, &ubo_billboard, &name)?;
            }
        }

        let gizmo_rotation = cgmath::Matrix3::from_cols(
            camera_right,
            camera_up_gizmo,
            camera_forward,
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

            if let Some(ref mut accel_struct) = self.data.raytracing.acceleration_structure {
                if i < accel_struct.blas_list.len() {
                    let blas = &accel_struct.blas_list[i];
                    if let Err(e) = RRAccelerationStructure::update_blas(
                        &self.instance,
                        &self.rrdevice,
                        &self.data.rrcommand_pool,
                        blas,
                        &rrdata.vertex_buffer.buffer,
                        rrdata.vertex_data.vertices.len() as u32,
                        std::mem::size_of::<vulkan_data::Vertex>() as u32,
                        &rrdata.index_buffer.buffer,
                        rrdata.vertex_data.indices.len() as u32,
                    ) {
                        eprintln!("failed to update BLAS: {}", e);
                    }
                }
            }
        }

        if let Some(ref mut accel_struct) = self.data.raytracing.acceleration_structure {
            let tlas = &accel_struct.tlas;
            if let Err(e) = RRAccelerationStructure::update_tlas(
                &self.instance,
                &self.rrdevice,
                &self.data.rrcommand_pool,
                tlas,
                &accel_struct.blas_list,
            ) {
                eprintln!("failed to update TLAS: {}", e);
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
            rrdata.delete_buffers(rrdevice);

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

            let buffer_name = format!("reload_mesh_{}", i);
            RRData::create_uniform_buffers(rrdata, &instance, &rrdevice, &data.rrswapchain, &buffer_name);

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
            rrdevice.device.bind_buffer_memory(index_buffer, index_buffer_memory, 0)?;

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
            let view = view(
                camera_pos,
                camera_direction,
                self.data.camera.up(),
            );
            use rust_rendering::math::coordinate_system::vulkan_projection_correction;
            let swapchain_extent = self.data.rrswapchain.swapchain_extent;
            let aspect = swapchain_extent.width as f32 / swapchain_extent.height as f32;
            let proj = vulkan_projection_correction() * cgmath::perspective(Deg(45.0), aspect, 0.1, 10000.0);
            let screen_size = Vector2::new(
                swapchain_extent.width as f32,
                swapchain_extent.height as f32,
            );

            let (ray_origin, ray_direction) = screen_to_world_ray(mouse_pos, screen_size, view, proj);

            log!("update_light_gizmo_position - camera_pos: ({:.2}, {:.2}, {:.2})", camera_pos.x, camera_pos.y, camera_pos.z);
            log!("update_light_gizmo_position - ray_origin: ({:.2}, {:.2}, {:.2})", ray_origin.x, ray_origin.y, ray_origin.z);
            log!("update_light_gizmo_position - ray_direction: ({:.2}, {:.2}, {:.2})", ray_direction.x, ray_direction.y, ray_direction.z);

            let light_pos = self.data.rt_debug_state.light_position;
            let plane_point = light_pos;
            let plane_normal = -camera_direction;

            let denom = plane_normal.dot(ray_direction);

            if denom.abs() > std::f32::EPSILON {
                let t = (plane_point - ray_origin).dot(plane_normal) / denom;

                log!("update_light_gizmo_position - t: {:.2}, intersection will be: ({:.2}, {:.2}, {:.2})",
                     t,
                     (ray_origin + ray_direction * t).x,
                     (ray_origin + ray_direction * t).y,
                     (ray_origin + ray_direction * t).z);

                if t >= 0.0 {
                    let intersection = ray_origin + ray_direction * t;
                    let initial_pos = vec3_from_array(self.data.light_gizmo_data.initial_position);

                    self.data.light_gizmo_data.update_position_with_constraint(
                        intersection,
                        initial_pos,
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

        log!("=== Shadow Debug Info ===");
        log!("Light position (rt_debug_state): ({:.2}, {:.2}, {:.2})", light_pos.x, light_pos.y, light_pos.z);
        log!("Light gizmo position: ({:.2}, {:.2}, {:.2})",
            self.data.light_gizmo_data.position.x,
            self.data.light_gizmo_data.position.y,
            self.data.light_gizmo_data.position.z);
        log!("Camera position: ({:.2}, {:.2}, {:.2})", camera_pos.x, camera_pos.y, camera_pos.z);

        log!("Shadow settings:");
        log!("  strength: {:.2}", self.data.rt_debug_state.shadow_strength);
        log!("  normal_offset: {:.2}", self.data.rt_debug_state.shadow_normal_offset);
        log!("  debug_view_mode: {:?}", self.data.rt_debug_state.debug_view_mode);
        log!("  distance_attenuation: {}", self.data.rt_debug_state.enable_distance_attenuation);

        if let Some(ref accel_struct) = self.data.raytracing.acceleration_structure {
            log!("Acceleration Structure:");
            log!("  BLAS count: {}", accel_struct.blas_list.len());
            for (i, blas) in accel_struct.blas_list.iter().enumerate() {
                log!("    BLAS[{}]: AS={:?}, device_addr={:#x}",
                    i, blas.acceleration_structure.is_some(), blas.device_address);
            }
            log!("  TLAS: AS={:?}", accel_struct.tlas.acceleration_structure.is_some());
        } else {
            log!("WARNING: No acceleration structure!");
        }

        log!("Vertex buffers (GPU):");
        for (i, rrdata) in self.data.model_descriptor_set.rrdata.iter().enumerate() {
            log!("  Mesh[{}]: {} vertices, {} indices",
                i, rrdata.vertex_data.vertices.len(), rrdata.vertex_data.indices.len());
            if !rrdata.vertex_data.vertices.is_empty() {
                let v = &rrdata.vertex_data.vertices[0];
                log!("    vertex[0].pos: ({:.2}, {:.2}, {:.2})", v.pos.x, v.pos.y, v.pos.z);
                log!("    vertex[0].normal: ({:.3}, {:.3}, {:.3})", v.normal.x, v.normal.y, v.normal.z);
            }
        }

        if !self.data.fbx_model.fbx_data.is_empty() {
            log!("FBX model data:");
            for (i, fbx_data) in self.data.fbx_model.fbx_data.iter().enumerate() {
                log!("  Mesh[{}]: {} positions, {} normals",
                    i, fbx_data.positions.len(), fbx_data.normals.len());
                if !fbx_data.positions.is_empty() {
                    let (min_x, max_x) = fbx_data.positions.iter().fold((f32::MAX, f32::MIN), |(min, max), p| (min.min(p.x), max.max(p.x)));
                    let (min_y, max_y) = fbx_data.positions.iter().fold((f32::MAX, f32::MIN), |(min, max), p| (min.min(p.y), max.max(p.y)));
                    let (min_z, max_z) = fbx_data.positions.iter().fold((f32::MAX, f32::MIN), |(min, max), p| (min.min(p.z), max.max(p.z)));
                    log!("    bounds: X[{:.2}, {:.2}], Y[{:.2}, {:.2}], Z[{:.2}, {:.2}]", min_x, max_x, min_y, max_y, min_z, max_z);

                    let center = Vector3::new((min_x + max_x) / 2.0, (min_y + max_y) / 2.0, (min_z + max_z) / 2.0);
                    let light_to_center = center - light_pos;
                    let dist = light_to_center.magnitude();
                    if dist > 0.001 {
                        log!("    light->center: dir=({:.3}, {:.3}, {:.3}), dist={:.2}",
                            light_to_center.x / dist, light_to_center.y / dist, light_to_center.z / dist, dist);
                    }

                    log!("    Light relative to model:");
                    log!("      X: {} (light={:.2}, range=[{:.2}, {:.2}])",
                        if light_pos.x < min_x { "LEFT" } else if light_pos.x > max_x { "RIGHT" } else { "INSIDE" },
                        light_pos.x, min_x, max_x);
                    log!("      Y: {} (light={:.2}, range=[{:.2}, {:.2}])",
                        if light_pos.y < min_y { "BELOW" } else if light_pos.y > max_y { "ABOVE" } else { "INSIDE" },
                        light_pos.y, min_y, max_y);
                    log!("      Z: {} (light={:.2}, range=[{:.2}, {:.2}])",
                        if light_pos.z < min_z { "BEHIND" } else if light_pos.z > max_z { "FRONT" } else { "INSIDE" },
                        light_pos.z, min_z, max_z);
                }
                if !fbx_data.normals.is_empty() {
                    log!("    normal[0]: ({:.3}, {:.3}, {:.3})",
                        fbx_data.normals[0].x, fbx_data.normals[0].y, fbx_data.normals[0].z);
                }
            }
        }

        log!("=========================");
    }
}
