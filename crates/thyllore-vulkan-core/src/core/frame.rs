use std::rc::Rc;

use crate::command::{RRCommandBuffer, RRCommandPool};
use crate::core::device::RRDevice;
use crate::resource::GpuResource;
use crate::types::{CommandPool, Fence, Semaphore};
use crate::vulkan::*;

#[derive(Debug)]
pub enum AcquireOutcome {
    Image(usize),
    OutOfDate,
}

pub unsafe fn acquire_swapchain_image(
    rrdevice: &RRDevice,
    swapchain: vk::SwapchainKHR,
    image_available: vk::Semaphore,
) -> Result<AcquireOutcome> {
    let result = rrdevice.device.acquire_next_image_khr(
        swapchain,
        u64::MAX,
        image_available,
        vk::Fence::null(),
    );

    match result {
        Ok((image_index, _)) => Ok(AcquireOutcome::Image(image_index as usize)),
        Err(vk::ErrorCode::OUT_OF_DATE_KHR) => Ok(AcquireOutcome::OutOfDate),
        Err(e) => Err(anyhow::anyhow!(e)),
    }
}

pub struct FrameSubmit {
    pub command_buffer: vk::CommandBuffer,
    pub image_available: vk::Semaphore,
    pub render_finished: vk::Semaphore,
    pub fence: vk::Fence,
    pub swapchain: vk::SwapchainKHR,
    pub image_index: usize,
}

#[derive(Debug)]
pub enum PresentOutcome {
    Presented,
    OutOfDate,
}

pub unsafe fn submit_and_present(
    rrdevice: &RRDevice,
    frame: FrameSubmit,
) -> Result<PresentOutcome> {
    let wait_semaphores = &[frame.image_available];
    let wait_stages = &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
    let command_buffers = &[frame.command_buffer];
    let signal_semaphores = &[frame.render_finished];
    let submit_info = vk::SubmitInfo::builder()
        .wait_semaphores(wait_semaphores)
        .wait_dst_stage_mask(wait_stages)
        .command_buffers(command_buffers)
        .signal_semaphores(signal_semaphores);

    rrdevice.device.reset_fences(&[frame.fence])?;
    rrdevice
        .device
        .queue_submit(rrdevice.graphics_queue, &[submit_info], frame.fence)?;

    let swapchains = &[frame.swapchain];
    let image_indices = &[frame.image_index as u32];
    let present_info = vk::PresentInfoKHR::builder()
        .wait_semaphores(signal_semaphores)
        .swapchains(swapchains)
        .image_indices(image_indices);
    let present_result = rrdevice
        .device
        .queue_present_khr(rrdevice.present_queue, &present_info);

    match present_result {
        Ok(vk::SuccessCode::SUBOPTIMAL_KHR) | Err(vk::ErrorCode::OUT_OF_DATE_KHR) => {
            Ok(PresentOutcome::OutOfDate)
        }
        Ok(_) => Ok(PresentOutcome::Presented),
        Err(e) => Err(anyhow::anyhow!(e)),
    }
}

pub struct FrameSync {
    pub image_available: Vec<Semaphore>,
    pub render_finished: Vec<Semaphore>,
    pub in_flight: Vec<Fence>,
    pub current_frame: usize,
}

impl FrameSync {
    pub fn new(
        image_available: Vec<Semaphore>,
        render_finished: Vec<Semaphore>,
        in_flight: Vec<Fence>,
    ) -> Self {
        Self {
            image_available,
            render_finished,
            in_flight,
            current_frame: 0,
        }
    }

    pub fn advance(&mut self, max_frames: usize) {
        self.current_frame = (self.current_frame + 1) % max_frames;
    }

    pub fn current_image_available(&self) -> Semaphore {
        self.image_available[self.current_frame]
    }

    pub fn current_render_finished(&self) -> Semaphore {
        self.render_finished[self.current_frame]
    }

    pub fn current_fence(&self) -> Fence {
        self.in_flight[self.current_frame]
    }
}

impl GpuResource for FrameSync {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        let device = &rrdevice.device;
        for semaphore in self
            .image_available
            .drain(..)
            .chain(self.render_finished.drain(..))
        {
            device.destroy_semaphore(semaphore, None);
        }
        for fence in self.in_flight.drain(..) {
            device.destroy_fence(fence, None);
        }
    }

    fn is_destroyed(&self) -> bool {
        self.image_available.is_empty()
            && self.render_finished.is_empty()
            && self.in_flight.is_empty()
    }
}

pub struct CommandState {
    pub pool: Rc<RRCommandPool>,
    pub buffers: RRCommandBuffer,
}

impl CommandState {
    pub fn new(pool: Rc<RRCommandPool>, buffers: RRCommandBuffer) -> Self {
        Self { pool, buffers }
    }
}

impl GpuResource for CommandState {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        let device = &rrdevice.device;
        if !self.buffers.command_buffers.is_empty() {
            device.free_command_buffers(self.pool.command_pool, &self.buffers.command_buffers);
            self.buffers.command_buffers.clear();
        }
        if self.pool.command_pool != CommandPool::null() {
            device.destroy_command_pool(self.pool.command_pool, None);
        }
    }

    fn is_destroyed(&self) -> bool {
        self.buffers.command_buffers.is_empty()
    }
}

pub struct SurfaceState {
    pub surface: vk::SurfaceKHR,
    pub messenger: vk::DebugUtilsMessengerEXT,
}

impl SurfaceState {
    pub fn new(surface: vk::SurfaceKHR, messenger: vk::DebugUtilsMessengerEXT) -> Self {
        Self { surface, messenger }
    }
}

pub unsafe fn destroy_surface_and_instance(
    instance: &Instance,
    surface_state: Option<SurfaceState>,
) {
    if let Some(surface_state) = surface_state {
        instance.destroy_surface_khr(surface_state.surface, None);
        if surface_state.messenger != vk::DebugUtilsMessengerEXT::null() {
            instance.destroy_debug_utils_messenger_ext(surface_state.messenger, None);
        }
    }
    instance.destroy_instance(None);
}
