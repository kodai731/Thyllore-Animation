pub mod deferred;
pub mod scene_renderer;

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
        let command_buffer = self.command_state().buffers.command_buffers[image_index];

        self.rrdevice
            .device
            .reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())?;

        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        self.rrdevice
            .device
            .begin_command_buffer(command_buffer, &begin_info)?;

        let use_gbuffer = self.data.raytracing.is_available()
            && self.data.viewport.offscreen.is_some();

        if use_gbuffer {
            deferred::record_gbuffer_pass(self, command_buffer, image_index)?;

            deferred::record_ray_query_pass(self, command_buffer)?;

            let has_hdr_pipeline = self.data.viewport.hdr_buffer.is_some()
                && self.data.raytracing.tonemap_pipeline.is_some();

            if has_hdr_pipeline {
                deferred::record_composite_to_hdr(self, command_buffer)?;
                deferred::record_bloom(self, command_buffer)?;
                deferred::record_tonemap_to_offscreen(self, command_buffer, image_index)?;
            } else {
                deferred::record_composite_to_offscreen(self, command_buffer, image_index)?;
            }

            self.begin_main_render_pass(command_buffer, image_index);
            self.record_imgui_rendering(command_buffer, draw_data)?;
            self.rrdevice.device.cmd_end_render_pass(command_buffer);
        } else {
            if let Some(ref offscreen) = self.data.viewport.offscreen {
                self.begin_offscreen_render_pass(command_buffer, offscreen);
                self.record_3d_rendering_to_offscreen(command_buffer, image_index, offscreen)?;
                self.rrdevice.device.cmd_end_render_pass(command_buffer);
            }

            self.begin_main_render_pass(command_buffer, image_index);
            self.record_imgui_rendering(command_buffer, draw_data)?;
            self.rrdevice.device.cmd_end_render_pass(command_buffer);
        }

        self.rrdevice.device.end_command_buffer(command_buffer)?;

        Ok(())
    }
}
