use crate::app::init::MAX_FRAMES_IN_FLIGHT;
use crate::app::{App, GUIData};
use crate::ecs::render_system;
use crate::vulkanr::command::*;
use crate::vulkanr::context::{FrameSync, SwapchainState};
use crate::vulkanr::vulkan::*;

use anyhow::{anyhow, Result};
use cgmath::{Matrix4, SquareMatrix};

impl App {
    pub unsafe fn begin_frame(&mut self, gui_data: &mut GUIData) -> Result<usize> {
        if gui_data.file_changed {
            crate::log!("Loading new model from: {}", gui_data.selected_model_path);
            self.rrdevice.device.device_wait_idle()?;

            let command_pool = self.command_state().pool.clone();
            let swapchain = self.swapchain_state().swapchain.clone();
            let rrrender = self.render_targets().render.clone();
            match Self::load_model_from_path_with_resources(
                &self.instance,
                &self.rrdevice,
                &mut self.data,
                &self.scene,
                &command_pool,
                &swapchain,
                &rrrender,
                &gui_data.selected_model_path,
            ) {
                Ok(_) => {
                    self.animation_playback_mut().model_path = gui_data.selected_model_path.clone();
                    self.animation_playback_mut().time = 0.0;
                    gui_data.load_status = format!("Loaded: {}", gui_data.selected_model_path);
                    crate::log!(
                        "Successfully loaded model: {}",
                        gui_data.selected_model_path
                    );
                }
                Err(e) => {
                    gui_data.load_status = format!("Error: {}", e);
                    crate::log!("Failed to load model: {:?}", e);
                }
            }

            gui_data.file_changed = false;
        }

        if gui_data.dump_debug_info {
            self.dump_debug_info();
            gui_data.dump_debug_info = false;
        }

        let current_fence = self.frame_sync().current_fence();
        self.rrdevice
            .device
            .wait_for_fences(&[current_fence], true, u64::MAX)?;

        let swapchain = self.swapchain_state().swapchain.swapchain;
        let image_available = self.frame_sync().current_image_available();
        let result = self.rrdevice.device.acquire_next_image_khr(
            swapchain,
            u64::MAX,
            image_available,
            vk::Fence::null(),
        );

        let image_index = match result {
            Ok((image_index, _)) => image_index as usize,
            Err(e) => return Err(anyhow!(e)),
        };

        let image_in_flight = self.resource::<SwapchainState>().images_in_flight[image_index];
        if !image_in_flight.is_null() {
            self.rrdevice
                .device
                .wait_for_fences(&[image_in_flight], true, u64::MAX)?;
        }

        let current_fence = self.frame_sync().current_fence();
        self.resource_mut::<SwapchainState>().images_in_flight[image_index] = current_fence;

        Ok(image_index)
    }

    pub unsafe fn render(
        &mut self,
        image_index: usize,
        gui_data: &mut GUIData,
        draw_data: &imgui::DrawData,
    ) -> Result<()> {
        Self::update_imgui_buffers(&self.instance, &self.rrdevice, &mut self.data, draw_data)?;

        self.record_command_buffer(image_index, gui_data, draw_data)?;

        let image_available = self.frame_sync().current_image_available();
        let render_finished = self.frame_sync().current_render_finished();
        let current_fence = self.frame_sync().current_fence();

        let wait_semaphores = &[image_available];
        let wait_stages = &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let command_buffers = &[self.command_state().buffers.command_buffers[image_index]];
        let signal_semaphores = &[render_finished];
        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(wait_semaphores)
            .wait_dst_stage_mask(wait_stages)
            .command_buffers(command_buffers)
            .signal_semaphores(signal_semaphores);

        self.rrdevice.device.reset_fences(&[current_fence])?;
        self.rrdevice.device.queue_submit(
            self.rrdevice.graphics_queue,
            &[submit_info],
            current_fence,
        )?;

        let swapchain = self.swapchain_state().swapchain.swapchain;
        let swapchains = &[swapchain];
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
        } else if let Err(e) = present_result {
            return Err(anyhow!(e));
        }

        if gui_data.take_screenshot {
            crate::log!("Taking screenshot...");
            self.save_screenshot(image_index)?;
            gui_data.take_screenshot = false;
            crate::log!("Screenshot saved!");
        }

        self.frame_sync_mut().advance(MAX_FRAMES_IN_FLIGHT);
        self.frame = self.frame_sync().current_frame;

        Ok(())
    }
    unsafe fn save_screenshot(&self, image_index: usize) -> Result<()> {
        use std::fs::File;
        use std::io::BufWriter;
        use std::time::SystemTime;

        let device = &self.rrdevice.device;
        let swapchain = &self.swapchain_state().swapchain;
        let swapchain_image = swapchain.swapchain_images[image_index];
        let extent = swapchain.swapchain_extent;
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
        let command_pool = self.command_state().pool.command_pool;
        let alloc_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(command_pool)
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
        let submit_info = vk::SubmitInfo::builder().command_buffers(&command_buffers_slice);
        device.queue_submit(
            self.rrdevice.graphics_queue,
            &[submit_info.build()],
            vk::Fence::null(),
        )?;
        device.queue_wait_idle(self.rrdevice.graphics_queue)?;

        // Map memory and read data
        let data = device.map_memory(buffer_memory, 0, image_size, vk::MemoryMapFlags::empty())?;
        let slice = std::slice::from_raw_parts(data as *const u8, image_size as usize);

        // Convert BGRA to RGBA
        let mut rgba_data = vec![0u8; (width * height * 4) as usize];
        for y in 0..height {
            for x in 0..width {
                let i = ((y * width + x) * 4) as usize;
                rgba_data[i] = slice[i + 2]; // R = B
                rgba_data[i + 1] = slice[i + 1]; // G = G
                rgba_data[i + 2] = slice[i]; // B = R
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

        crate::log!("Screenshot saved to: {}", filename);

        // Cleanup
        device.free_command_buffers(command_pool, &[command_buffer]);
        device.free_memory(buffer_memory, None);
        device.destroy_buffer(buffer, None);

        Ok(())
    }

    pub unsafe fn begin_main_render_pass(
        &self,
        command_buffer: vk::CommandBuffer,
        image_index: usize,
    ) {
        let render_area = vk::Rect2D::builder()
            .offset(vk::Offset2D::default())
            .extent(self.swapchain_state().swapchain.swapchain_extent);

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

        let render_targets = self.render_targets();
        let render_pass_info = vk::RenderPassBeginInfo::builder()
            .render_pass(render_targets.render.render_pass)
            .framebuffer(render_targets.render.framebuffers[image_index])
            .render_area(render_area)
            .clear_values(&clear_values);

        self.rrdevice.device.cmd_begin_render_pass(
            command_buffer,
            &render_pass_info,
            vk::SubpassContents::INLINE,
        );
    }

    unsafe fn render_models(&self, command_buffer: vk::CommandBuffer, image_index: usize) {
        static mut RENDER_LOG_COUNTER: u32 = 0;
        static mut PREV_MESH_COUNT: usize = 0;

        let mesh_count = self.data.graphics_resources.meshes.len();
        let mesh_count_changed = mesh_count != PREV_MESH_COUNT;
        if mesh_count_changed {
            RENDER_LOG_COUNTER = 0;
            PREV_MESH_COUNT = mesh_count;
        }

        RENDER_LOG_COUNTER += 1;
        let should_log = RENDER_LOG_COUNTER <= 3;

        if should_log {
            crate::log!("=== render_models: {} meshes ===", mesh_count);
        }

        for i in 0..mesh_count {
            let mesh = &self.data.graphics_resources.meshes[i];

            if should_log {
                crate::log!(
                    "  Mesh[{}]: vertex_buffer={:?}, indices={}, vertices={}",
                    i,
                    mesh.vertex_buffer.buffer,
                    mesh.index_buffer.indices,
                    mesh.vertex_data.vertices.len()
                );
            }

            let pipeline = &self.pipeline_state().model_pipeline;
            self.rrdevice.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.pipeline,
            );

            self.rrdevice.device.cmd_set_line_width(command_buffer, 1.0);

            self.rrdevice.device.cmd_bind_vertex_buffers(
                command_buffer,
                0,
                &[mesh.vertex_buffer.buffer],
                &[0],
            );

            self.rrdevice.device.cmd_bind_index_buffer(
                command_buffer,
                mesh.index_buffer.buffer,
                0,
                vk::IndexType::UINT32,
            );

            let frame_set = self.data.graphics_resources.frame_set.sets[image_index];
            self.rrdevice.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.pipeline_layout,
                0,
                &[frame_set],
                &[],
            );

            if let Some(material_id) = self.data.graphics_resources.get_material_id(i) {
                if let Some(material) = self.data.graphics_resources.materials.get(material_id) {
                    self.rrdevice.device.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        pipeline.pipeline_layout,
                        1,
                        &[material.descriptor_set],
                        &[],
                    );
                }
            }

            let object_set_idx = self
                .data
                .graphics_resources
                .objects
                .get_set_index(image_index, mesh.object_index);
            let object_set = self.data.graphics_resources.objects.sets[object_set_idx];
            self.rrdevice.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.pipeline_layout,
                2,
                &[object_set],
                &[],
            );

            self.rrdevice.device.cmd_draw_indexed(
                command_buffer,
                mesh.index_buffer.indices,
                1,
                0,
                0,
                0,
            );
        }
    }

    pub unsafe fn record_3d_rendering(
        &self,
        command_buffer: vk::CommandBuffer,
        image_index: usize,
    ) -> Result<()> {
        let frame_set = self.data.graphics_resources.frame_set.sets[image_index];
        let camera_pos = self.data.camera.position;

        let render_data_vec = self.scene.collect_render_data(camera_pos);
        let render_data_refs: Vec<_> = render_data_vec.iter().collect();

        render_system(
            &render_data_refs,
            command_buffer,
            image_index,
            frame_set,
            &self.data.graphics_resources.objects,
            &self.rrdevice,
        );

        self.render_models(command_buffer, image_index);

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
        let descriptor_set = self
            .data
            .imgui
            .descriptor_set
            .ok_or_else(|| anyhow!("ImGui descriptor set not initialized"))?;
        let vertex_buffer = self
            .data
            .imgui
            .vertex_buffer
            .ok_or_else(|| anyhow!("ImGui vertex buffer not initialized"))?;
        let index_buffer = self
            .data
            .imgui
            .index_buffer
            .ok_or_else(|| anyhow!("ImGui index buffer not initialized"))?;

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
        self.rrdevice
            .device
            .cmd_bind_vertex_buffers(command_buffer, 0, &[vertex_buffer], &[0]);
        self.rrdevice.device.cmd_bind_index_buffer(
            command_buffer,
            index_buffer,
            0,
            vk::IndexType::UINT16,
        );

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
        self.rrdevice
            .device
            .cmd_set_viewport(command_buffer, 0, &[viewport]);

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
                                x: ((clip_rect[0] - draw_data.display_pos[0])
                                    * draw_data.framebuffer_scale[0])
                                    .max(0.0) as i32,
                                y: ((clip_rect[1] - draw_data.display_pos[1])
                                    * draw_data.framebuffer_scale[1])
                                    .max(0.0) as i32,
                            })
                            .extent(vk::Extent2D {
                                width: ((clip_rect[2] - clip_rect[0])
                                    * draw_data.framebuffer_scale[0])
                                    as u32,
                                height: ((clip_rect[3] - clip_rect[1])
                                    * draw_data.framebuffer_scale[1])
                                    as u32,
                            });
                        self.rrdevice
                            .device
                            .cmd_set_scissor(command_buffer, 0, &[scissor]);

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
