use crate::app::App;
use crate::hooks::batch_capture::CaptureSlot;
use crate::vulkanr::context::CommandState;
use crate::vulkanr::vulkan::*;

use anyhow::Result;
use thyllore_vulkan_core::copy_image_to_host_buffer;

impl App {
    pub unsafe fn save_screenshot(&self, image_index: usize) -> Result<String> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)?
            .as_secs();
        let path = std::path::PathBuf::from(format!("log/screenshot_{}.png", timestamp));
        self.save_screenshot_to(image_index, &path)
    }

    pub unsafe fn save_screenshot_to(
        &self,
        image_index: usize,
        path: &std::path::Path,
    ) -> Result<String> {
        self.capture_context(image_index, CaptureSlot::interactive())
            .save_screenshot_to(path)
    }

    pub unsafe fn copy_image_to_buffer(
        &self,
        image: vk::Image,
        width: u32,
        height: u32,
        image_size: vk::DeviceSize,
        layout: vk::ImageLayout,
    ) -> Result<(vk::Buffer, vk::DeviceMemory)> {
        let command_pool = self.resource::<CommandState>().pool.command_pool;
        copy_image_to_host_buffer(
            &self.instance,
            &self.rrdevice,
            self.rrdevice.graphics_queue,
            command_pool,
            image,
            width,
            height,
            image_size,
            layout,
        )
    }
}
