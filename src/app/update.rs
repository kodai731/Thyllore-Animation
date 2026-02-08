use crate::app::{App, AppData, FrameContext, GUIData};
use crate::ecs::{
    gizmo_update_or_create_vertical_line_buffers, gizmo_update_vertical_lines, run_frame,
};
use crate::vulkanr::device::RRDevice;
use crate::vulkanr::vulkan::*;
use crate::vulkanr::VulkanBackend;

use anyhow::Result;

impl App {
    pub unsafe fn update(&mut self, image_index: usize, gui_data: &mut GUIData) -> Result<()> {
        let time = self.start.elapsed().as_secs_f32();
        let delta_time = 1.0 / 60.0;

        let viewport_extent = (
            self.data.viewport.width.max(1),
            self.data.viewport.height.max(1),
        );
        let command_pool = self
            .data
            .ecs_world
            .resource::<crate::vulkanr::context::CommandState>()
            .pool
            .clone();

        {
            let mut ctx = FrameContext {
                instance: &self.instance,
                device: &self.rrdevice,
                command_pool,
                time,
                delta_time,
                image_index,
                swapchain_extent: viewport_extent,
                graphics: &mut self.data.graphics_resources,
                raytracing: &mut self.data.raytracing,
                buffer_registry: &mut self.data.buffer_registry,
                world: &mut self.data.ecs_world,
                assets: &self.data.ecs_assets,
                gui_data,
            };

            run_frame(&mut ctx)?;
        }

        self.process_debug_commands(gui_data)?;
        self.update_vertical_lines()?;

        Ok(())
    }

    unsafe fn process_debug_commands(&self, gui_data: &mut GUIData) -> Result<()> {
        if gui_data.debug_billboard_depth {
            self.log_billboard_debug_info();
            gui_data.debug_billboard_depth = false;
        }

        Ok(())
    }

    fn log_billboard_debug_info(&self) {
        use crate::debugview::{log_billboard_debug_info, BillboardDebugInfo, GBufferDebugInfo};
        use crate::ecs::systems::camera_systems::{
            compute_camera_direction, compute_camera_position,
            compute_camera_up,
        };

        let camera = self.camera();
        let rt_debug = self.rt_debug_state();
        let info = BillboardDebugInfo {
            light_position: rt_debug.light_position,
            camera_position: compute_camera_position(&camera),
            camera_direction: compute_camera_direction(&camera),
            camera_up: compute_camera_up(&camera),
            near_plane: camera.near_plane,
            fov_y: camera.fov_y,
        };

        let gbuffer_debug_info = self
            .data
            .raytracing
            .gbuffer
            .as_ref()
            .map(|gb| GBufferDebugInfo {
                position_image_view: gb.position_image_view,
                extent_width: gb.width,
                extent_height: gb.height,
            });

        let swapchain = &self
            .data
            .ecs_world
            .resource::<crate::vulkanr::context::SwapchainState>()
            .swapchain;

        log_billboard_debug_info(
            &info,
            swapchain,
            &self.billboard().render_state.descriptor_set,
            gbuffer_debug_info.as_ref(),
            self.data.raytracing.gbuffer_sampler,
        );
    }

    unsafe fn update_vertical_lines(&mut self) -> Result<()> {
        let model_tops: Vec<cgmath::Vector3<f32>> = Vec::new();

        {
            let mut gizmo = self.light_gizmo_mut();
            let position = gizmo.position.clone();
            gizmo_update_vertical_lines(&mut gizmo.vertical_lines, &position, &model_tops);
        }

        let command_pool = self
            .data
            .ecs_world
            .resource::<crate::vulkanr::context::CommandState>()
            .pool
            .clone();

        let mut backend = VulkanBackend::new(
            &self.instance,
            &self.rrdevice,
            command_pool,
            &mut self.data.graphics_resources,
            &mut self.data.raytracing,
            &mut self.data.buffer_registry,
        );

        let mut gizmo = self
            .data
            .ecs_world
            .resource_mut::<crate::debugview::gizmo::LightGizmoData>();
        gizmo_update_or_create_vertical_line_buffers(&mut gizmo.vertical_lines, &mut backend)?;

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

        let vtx_buffer_size = (draw_data.total_vtx_count as usize
            * std::mem::size_of::<imgui::DrawVert>())
            as vk::DeviceSize;
        let idx_buffer_size = (draw_data.total_idx_count as usize
            * std::mem::size_of::<imgui::DrawIdx>())
            as vk::DeviceSize;

        let needs_vertex_resize =
            data.imgui.vertex_buffer.is_none() || vtx_buffer_size > data.imgui.vertex_buffer_size;
        let needs_index_resize =
            data.imgui.index_buffer.is_none() || idx_buffer_size > data.imgui.index_buffer_size;

        if needs_vertex_resize || needs_index_resize {
            rrdevice.device.device_wait_idle()?;
        }

        if needs_vertex_resize {
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
                .memory_type_index(crate::vulkanr::vulkan::get_memory_type_index(
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

        if needs_index_resize {
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
                .memory_type_index(crate::vulkanr::vulkan::get_memory_type_index(
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

    pub(crate) fn log_shadow_debug_info(&self) {
        use crate::ecs::systems::camera_systems::compute_camera_position;

        let rt_debug = self.rt_debug_state();
        let camera = self.camera();
        let cam_pos = compute_camera_position(&camera);

        crate::log!("=== Shadow Debug Info ===");
        crate::log!(
            "Light position (rt_debug_state): ({:.2}, {:.2}, {:.2})",
            rt_debug.light_position.x,
            rt_debug.light_position.y,
            rt_debug.light_position.z
        );
        crate::log!(
            "Light gizmo position: ({:.2}, {:.2}, {:.2})",
            self.light_gizmo().position.position.x,
            self.light_gizmo().position.position.y,
            self.light_gizmo().position.position.z
        );
        crate::log!(
            "Camera position: ({:.2}, {:.2}, {:.2})",
            cam_pos.x,
            cam_pos.y,
            cam_pos.z
        );

        crate::log!("Shadow settings:");
        crate::log!("  strength: {:.2}", rt_debug.shadow_strength);
        crate::log!("  normal_offset: {:.2}", rt_debug.shadow_normal_offset);
        crate::log!("  debug_view_mode: {:?}", rt_debug.debug_view_mode);
        crate::log!(
            "  distance_attenuation: {}",
            rt_debug.enable_distance_attenuation
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
