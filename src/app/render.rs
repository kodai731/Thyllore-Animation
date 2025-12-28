use crate::app::{App, AppData, GUIData};
use rust_rendering::vulkanr::buffer::*;
use rust_rendering::vulkanr::command::*;
use rust_rendering::vulkanr::data as vulkan_data;
use rust_rendering::vulkanr::data::*;
use rust_rendering::vulkanr::descriptor::*;
use rust_rendering::vulkanr::device::*;
use rust_rendering::vulkanr::image::*;
use rust_rendering::vulkanr::pipeline::*;
use rust_rendering::vulkanr::render::*;
use rust_rendering::vulkanr::vulkan::*;
use rust_rendering::debugview::*;
use rust_rendering::logger::logger::*;

use crate::app::init::MAX_FRAMES_IN_FLIGHT;
use cgmath::{Matrix4, SquareMatrix};
use anyhow::{anyhow, Result};
use std::mem::size_of;
use std::ptr::copy_nonoverlapping as memcpy;
use std::os::raw::c_void;
use winit::window::Window;
use vulkanalia::prelude::v1_0::*;

impl App {
    pub unsafe fn render(&mut self, window: &Window, gui_data: &mut GUIData, draw_data: &imgui::DrawData) -> Result<()> {
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
        self.record_command_buffer(image_index, gui_data, draw_data)?;

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
    pub unsafe fn record_3d_rendering(
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

        if let (Some(vertex_buffer), Some(index_buffer)) =
            (self.data.light_gizmo_data.vertex_buffer, self.data.light_gizmo_data.index_buffer) {

            self.rrdevice.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.data.gizmo_pipeline.pipeline,
            );

            self.rrdevice.device.cmd_set_line_width(command_buffer, 1.0);

            self.rrdevice.device.cmd_bind_vertex_buffers(
                command_buffer,
                0,
                &[vertex_buffer],
                &[0],
            );

            self.rrdevice.device.cmd_bind_index_buffer(
                command_buffer,
                index_buffer,
                0,
                vk::IndexType::UINT32,
            );

            let swapchain_images_len = self.data.gizmo_descriptor_set.descriptor_sets.len() /
                self.data.gizmo_descriptor_set.rrdata.len().max(1);
            let descriptor_set_index = 1 * swapchain_images_len + image_index;

            self.rrdevice.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.data.gizmo_pipeline.pipeline_layout,
                0,
                &[self.data.gizmo_descriptor_set.descriptor_sets[descriptor_set_index]],
                &[],
            );

            self.rrdevice.device.cmd_draw_indexed(
                command_buffer,
                self.data.light_gizmo_data.indices.len() as u32,
                1,
                0,
                0,
                0,
            );
        }

        Ok(())
    }

    /// Record ImGui rendering commands
    pub unsafe fn record_imgui_rendering(
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
}
