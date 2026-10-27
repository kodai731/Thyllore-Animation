use crate::app::{App, AppData};
use crate::ecs::run_frame;
use crate::ecs::FrameContext;
use crate::vulkanr::device::RRDevice;

use anyhow::Result;
use thyllore_vulkan_core::resource::{HostBuffer, HostBufferUsage};
use thyllore_vulkan_core::types::{DeviceSize, Instance};

impl App {
    pub unsafe fn update(&mut self, image_index: usize) -> Result<()> {
        let fixed_delta = self
            .resource::<crate::ecs::resource::FrameClock>()
            .fixed_delta_seconds();
        let (time, delta_time) = match fixed_delta {
            Some(delta_time) => (self.last_update_time + delta_time, delta_time),
            None => {
                let time = self.start.elapsed().as_secs_f32();
                (time, time - self.last_update_time)
            }
        };
        self.last_update_time = time;
        self.last_frame_interval = delta_time;

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
                frame_slot: self.frame % crate::app::init::MAX_FRAMES_IN_FLIGHT,
                swapchain_extent: viewport_extent,
                graphics: &mut self.data.graphics_resources,
                raytracing: &mut self.data.raytracing,
                buffer_registry: &mut self.data.buffer_registry,
                world: &mut self.data.ecs_world,
                assets: &mut self.data.ecs_assets,
                onion_skin_gpu: &mut self.data.onion_skin_gpu,
            };

            run_frame(&mut ctx)?;
        }

        Ok(())
    }

    pub unsafe fn update_imgui_buffers(
        instance: &Instance,
        rrdevice: &RRDevice,
        data: &mut AppData,
        draw_data: &imgui::DrawData,
        frame_slot: usize,
    ) -> Result<()> {
        if draw_data.total_vtx_count() == 0 || draw_data.total_idx_count() == 0 {
            return Ok(());
        }

        let vtx_buffer_size = (draw_data.total_vtx_count() as usize
            * std::mem::size_of::<imgui::DrawVert>()) as DeviceSize;
        let idx_buffer_size = (draw_data.total_idx_count() as usize
            * std::mem::size_of::<imgui::DrawIdx>()) as DeviceSize;

        let needs_vertex_resize = data.imgui.vertex_buffers[frame_slot]
            .as_ref()
            .map_or(true, |b| vtx_buffer_size > b.size());
        let needs_index_resize = data.imgui.index_buffers[frame_slot]
            .as_ref()
            .map_or(true, |b| idx_buffer_size > b.size());

        if needs_vertex_resize || needs_index_resize {
            rrdevice.wait_idle()?;
        }

        if needs_vertex_resize {
            resize_imgui_vertex_buffer(instance, rrdevice, data, vtx_buffer_size, frame_slot)?;
        }

        if needs_index_resize {
            resize_imgui_index_buffer(instance, rrdevice, data, idx_buffer_size, frame_slot)?;
        }

        upload_imgui_vertex_data(rrdevice, data, draw_data, vtx_buffer_size, frame_slot)?;
        upload_imgui_index_data(rrdevice, data, draw_data, idx_buffer_size, frame_slot)?;

        Ok(())
    }

    pub unsafe fn apply_app_commands(&mut self) {
        crate::app::command::apply_queued_commands(self);

        self.spawn_pending_debug_primitives();
    }
}

unsafe fn resize_imgui_vertex_buffer(
    instance: &Instance,
    rrdevice: &RRDevice,
    data: &mut AppData,
    vtx_buffer_size: DeviceSize,
    frame_slot: usize,
) -> Result<()> {
    if let Some(ref mut old) = data.imgui.vertex_buffers[frame_slot] {
        old.destroy(rrdevice);
    }

    let vertex_buffer =
        HostBuffer::create(instance, rrdevice, vtx_buffer_size, HostBufferUsage::Vertex)?;
    data.imgui.vertex_buffers[frame_slot] = Some(vertex_buffer);

    Ok(())
}

unsafe fn resize_imgui_index_buffer(
    instance: &Instance,
    rrdevice: &RRDevice,
    data: &mut AppData,
    idx_buffer_size: DeviceSize,
    frame_slot: usize,
) -> Result<()> {
    if let Some(ref mut old) = data.imgui.index_buffers[frame_slot] {
        old.destroy(rrdevice);
    }

    let index_buffer =
        HostBuffer::create(instance, rrdevice, idx_buffer_size, HostBufferUsage::Index)?;
    data.imgui.index_buffers[frame_slot] = Some(index_buffer);

    Ok(())
}

unsafe fn upload_imgui_vertex_data(
    rrdevice: &RRDevice,
    data: &AppData,
    draw_data: &imgui::DrawData,
    vtx_buffer_size: DeviceSize,
    frame_slot: usize,
) -> Result<()> {
    let Some(vertex_buffer) = &data.imgui.vertex_buffers[frame_slot] else {
        return Ok(());
    };

    vertex_buffer.write_with(rrdevice, vtx_buffer_size, |slice| {
        let mut offset = 0;
        for draw_list in draw_data.draw_lists() {
            let vtx_buffer = draw_list.vtx_buffer();
            let vtx_size = (vtx_buffer.len() * std::mem::size_of::<imgui::DrawVert>()) as usize;
            std::ptr::copy_nonoverlapping(
                vtx_buffer.as_ptr() as *const u8,
                slice.as_mut_ptr().add(offset),
                vtx_size,
            );
            offset += vtx_size;
        }
    })
}

unsafe fn upload_imgui_index_data(
    rrdevice: &RRDevice,
    data: &AppData,
    draw_data: &imgui::DrawData,
    idx_buffer_size: DeviceSize,
    frame_slot: usize,
) -> Result<()> {
    let Some(index_buffer) = &data.imgui.index_buffers[frame_slot] else {
        return Ok(());
    };

    index_buffer.write_with(rrdevice, idx_buffer_size, |slice| {
        let mut offset = 0;
        for draw_list in draw_data.draw_lists() {
            let idx_buffer = draw_list.idx_buffer();
            let idx_size = (idx_buffer.len() * std::mem::size_of::<imgui::DrawIdx>()) as usize;
            std::ptr::copy_nonoverlapping(
                idx_buffer.as_ptr() as *const u8,
                slice.as_mut_ptr().add(offset),
                idx_size,
            );
            offset += idx_size;
        }
    })
}
