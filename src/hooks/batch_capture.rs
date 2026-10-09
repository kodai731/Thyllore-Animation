use std::path::{Path, PathBuf};

use anyhow::{ensure, Result};
use vulkanalia::prelude::v1_0::*;

use crate::ecs::world::{Resource, World};
use crate::vulkanr::context::SwapchainState;
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;
use thyllore_vulkan_core::resource::hdr_buffer::HdrBuffer;
use thyllore_vulkan_core::resource::raytracing_data::RayTracingData;
use thyllore_vulkan_core::resource::{copy_image_to_host_buffer, write_host_buffer_bgra_png};

/// Which capture of the schedule is being taken; `sequence_dir` is set in sequence mode.
#[derive(Clone, Debug, PartialEq)]
pub struct CaptureSlot {
    pub index: u32,
    pub count: u32,
    pub sequence_dir: Option<PathBuf>,
}

impl CaptureSlot {
    pub fn interactive() -> Self {
        Self {
            index: 0,
            count: 1,
            sequence_dir: None,
        }
    }
}

/// Everything a readback at a finished frame needs; the GPU is idle while it exists.
pub struct CaptureContext<'a> {
    pub instance: &'a Instance,
    pub device: &'a RRDevice,
    pub command_pool: vk::CommandPool,
    pub world: &'a World,
    pub graphics: &'a GraphicsResources,
    pub raytracing: &'a RayTracingData,
    pub hdr: Option<&'a HdrBuffer>,
    pub image_index: usize,
    pub slot: CaptureSlot,
}

impl CaptureContext<'_> {
    pub unsafe fn copy_image_to_buffer(
        &self,
        image: vk::Image,
        width: u32,
        height: u32,
        image_size: vk::DeviceSize,
        layout: vk::ImageLayout,
    ) -> Result<(vk::Buffer, vk::DeviceMemory)> {
        copy_image_to_host_buffer(
            self.instance,
            self.device,
            self.device.graphics_queue,
            self.command_pool,
            image,
            width,
            height,
            image_size,
            layout,
        )
    }

    /// Saves this frame's swapchain image as PNG and returns the absolute path written.
    pub unsafe fn save_screenshot_to(&self, path: &Path) -> Result<String> {
        let swapchain = &self.world.resource::<SwapchainState>().swapchain;
        let image = swapchain.swapchain_images[self.image_index];
        let extent = swapchain.swapchain_extent;
        let image_size = (extent.width * extent.height * 4) as vk::DeviceSize;

        let (buffer, buffer_memory) = self.copy_image_to_buffer(
            image,
            extent.width,
            extent.height,
            image_size,
            vk::ImageLayout::PRESENT_SRC_KHR,
        )?;
        let written = write_host_buffer_bgra_png(
            &self.device.device,
            buffer_memory,
            image_size,
            extent.width,
            extent.height,
            path,
        );
        self.device.device.free_memory(buffer_memory, None);
        self.device.device.destroy_buffer(buffer, None);
        written?;

        let absolute = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        let saved = absolute.to_string_lossy().to_string();
        log!("Screenshot saved to: {}", saved);
        Ok(saved)
    }
}

/// A readback requested for the batch capture frame. The request is the resource itself: a dump
/// action or a startup flag inserts it, and the capture phase runs every registered type that is
/// present in `World`. Implemented next to the dump in `src/debugview/`, registered with
/// `batch_capture!(Type)`.
pub trait BatchCapture: Resource + std::fmt::Debug {
    unsafe fn capture(&self, ctx: &CaptureContext) -> Result<()>;
}

pub type BatchCaptureFn = unsafe fn(&CaptureContext) -> Result<()>;

pub type TypeNameFn = fn() -> &'static str;

/// One registered request type, named by its type name.
#[derive(Clone, Copy)]
pub struct BatchCaptureHook {
    pub name: TypeNameFn,
    pub run: BatchCaptureFn,
}

inventory::collect!(BatchCaptureHook);

pub unsafe fn run_capture<T: BatchCapture>(ctx: &CaptureContext) -> Result<()> {
    match ctx.world.get_resource::<T>() {
        Some(request) => request.capture(ctx),
        None => Ok(()),
    }
}

#[macro_export]
macro_rules! batch_capture {
    ($request:ty) => {
        inventory::submit! {
            $crate::hooks::batch_capture::BatchCaptureHook {
                name: std::any::type_name::<$request>,
                run: $crate::hooks::batch_capture::run_capture::<$request>,
            }
        }
    };
}

/// Every registered request type, sorted by name.
pub struct BatchCaptureHooks {
    entries: Vec<(&'static str, BatchCaptureFn)>,
}

impl BatchCaptureHooks {
    pub fn collect() -> Result<Self> {
        let mut entries: Vec<(&'static str, BatchCaptureFn)> = inventory::iter::<BatchCaptureHook>
            .into_iter()
            .map(|hook| ((hook.name)(), hook.run))
            .collect();
        entries.sort_by_key(|(name, _)| *name);
        for pair in entries.windows(2) {
            ensure!(
                pair[0].0 != pair[1].0,
                "batch capture {} registered twice",
                pair[0].0
            );
        }
        Ok(Self { entries })
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.entries.iter().map(|(name, _)| *name).collect()
    }

    /// Runs every requested capture; a failing dump is logged and never stops the screenshot.
    pub unsafe fn run_all(&self, ctx: &CaptureContext) {
        for (name, run) in &self.entries {
            if let Err(error) = run(ctx) {
                log_warn!("batch capture {} failed: {:?}", name, error);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registered_hooks_are_unique_and_sorted() {
        let hooks = BatchCaptureHooks::collect().unwrap();
        let names = hooks.names();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(names, sorted);
    }
}
