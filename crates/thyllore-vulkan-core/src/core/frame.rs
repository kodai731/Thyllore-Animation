use crate::core::device::RRDevice;
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
