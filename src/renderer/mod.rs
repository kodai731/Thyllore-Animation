pub mod deferred;

use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::app::App;

impl App {
    pub unsafe fn record_command_buffer(
        &mut self,
        image_index: usize,
        gui_data: &mut crate::app::GUIData,
        draw_data: &imgui::DrawData,
    ) -> Result<()> {
        let command_buffer = self.data.rrcommand_buffer.command_buffers[image_index];

        self.rrdevice
            .device
            .reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())?;

        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        self.rrdevice
            .device
            .begin_command_buffer(command_buffer, &begin_info)?;

        let use_ray_tracing = self.data.raytracing.is_available();

        static mut RAY_TRACING_LOG_ONCE: bool = false;
        unsafe {
            if !RAY_TRACING_LOG_ONCE {
                if use_ray_tracing {
                    crate::log!("=== Using Ray Tracing Rendering Path ===");
                }
                RAY_TRACING_LOG_ONCE = true;
            }
        }

        if use_ray_tracing {
            deferred::record_gbuffer_pass(self, command_buffer, image_index)?;
            deferred::record_ray_query_pass(self, command_buffer)?;
            deferred::record_composite_pass(self, command_buffer, image_index, draw_data)?;
        } else {
            self.begin_main_render_pass(command_buffer, image_index);
            self.record_imgui_rendering(command_buffer, draw_data)?;
            self.rrdevice.device.cmd_end_render_pass(command_buffer);
        }

        self.rrdevice.device.end_command_buffer(command_buffer)?;

        Ok(())
    }

}
