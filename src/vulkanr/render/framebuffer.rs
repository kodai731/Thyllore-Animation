use crate::vulkanr::command::*;
use crate::vulkanr::core::device::*;
use crate::vulkanr::resource::image::*;
use crate::vulkanr::core::swapchain::*;
use crate::vulkanr::vulkan::*;
use crate::vulkanr::render::pass::{RRRender, get_depth_format};
use crate::vulkanr::render::gbuffer::RRGBuffer;
use anyhow::Result;

pub unsafe fn create_framebuffers(
    rrdevice: &RRDevice,
    rrswapchain: &RRSwapchain,
    rrrender: &mut RRRender,
) -> Result<()> {
    rrrender.framebuffers = rrswapchain
        .swapchain_image_views
        .iter()
        .map(|i| {
            let attachments = &[rrrender.color_image_view, rrrender.depth_image_view, *i];
            let create_info = vk::FramebufferCreateInfo::builder()
                .render_pass(rrrender.render_pass) // they use the same number and type of attachments.
                .attachments(attachments)
                .width(rrswapchain.swapchain_extent.width)
                .height(rrswapchain.swapchain_extent.height)
                .layers(1);
            rrdevice.device.create_framebuffer(&create_info, None)
        })
        .collect::<Result<Vec<_>, _>>()?;
    println!("created framebuffers");
    Ok(())
}

pub unsafe fn create_color_objects(
    instance: &Instance,
    rrdevice: &RRDevice,
    rrswapchain: &RRSwapchain,
    rrrender: &mut RRRender,
) -> Result<()> {
    //  this color buffer doesn't need mipmaps since it's not going to be used as a texture:
    let (color_image, color_image_memory) = create_image(
        instance,
        rrdevice,
        rrswapchain.swapchain_extent.width,
        rrswapchain.swapchain_extent.height,
        1,
        rrdevice.msaa_samples,
        rrswapchain.swapchain_format,
        vk::ImageTiling::OPTIMAL,
        vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSIENT_ATTACHMENT,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;

    rrrender.color_image = color_image;
    rrrender.color_image_memory = color_image_memory;

    rrrender.color_image_view = create_image_view(
        rrdevice,
        rrrender.color_image,
        rrswapchain.swapchain_format,
        vk::ImageAspectFlags::COLOR,
        1,
    )?;
    println!("created color objects");
    Ok(())
}

/// G-Buffer for deferred rendering and ray query
/// Stores position, normal, and shadow mask for ray traced shadows
pub unsafe fn create_gbuffer_framebuffer(
    instance: &Instance,
    rrdevice: &RRDevice,
    rrrender: &mut RRRender,
    gbuffer: &RRGBuffer,
) -> Result<()> {
    // Create depth image for G-Buffer
    let (depth_image, depth_image_memory) = create_image(
        instance,
        rrdevice,
        gbuffer.width,
        gbuffer.height,
        1,
        vk::SampleCountFlags::_1,
        get_depth_format(instance, rrdevice)?,
        vk::ImageTiling::OPTIMAL,
        vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;

    let depth_image_view = create_image_view(
        rrdevice,
        depth_image,
        get_depth_format(instance, rrdevice)?,
        vk::ImageAspectFlags::DEPTH,
        1,
    )?;

    rrrender.gbuffer_depth_image = depth_image;
    rrrender.gbuffer_depth_image_memory = depth_image_memory;
    rrrender.gbuffer_depth_image_view = depth_image_view;

    // Create framebuffer with position, normal, albedo, and depth attachments
    let attachments = [
        gbuffer.position_image_view,
        gbuffer.normal_image_view,
        gbuffer.albedo_image_view,
        depth_image_view,
    ];

    let info = vk::FramebufferCreateInfo::builder()
        .render_pass(rrrender.gbuffer_render_pass)
        .attachments(&attachments)
        .width(gbuffer.width)
        .height(gbuffer.height)
        .layers(1);

    rrrender.gbuffer_framebuffer = rrdevice.device.create_framebuffer(&info, None)?;

    log::info!("Created G-Buffer framebuffer: {}x{}", gbuffer.width, gbuffer.height);
    Ok(())
}
