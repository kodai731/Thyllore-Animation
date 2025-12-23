use super::command::*;
use super::device::*;
use super::image::*;
use super::swapchain::*;
use super::vulkan::*;
#[derive(Clone, Debug, Default)]
pub struct RRRender {
    // Main render pass (for final presentation)
    pub render_pass: vk::RenderPass,
    pub framebuffers: Vec<vk::Framebuffer>,
    pub depth_image: vk::Image,
    pub depth_image_memory: vk::DeviceMemory,
    pub depth_image_view: vk::ImageView,
    pub color_image: vk::Image,
    pub color_image_view: vk::ImageView,
    pub color_image_memory: vk::DeviceMemory,

    // G-Buffer render pass (for deferred rendering)
    pub gbuffer_render_pass: vk::RenderPass,
    pub gbuffer_framebuffer: vk::Framebuffer,
    pub gbuffer_depth_image: vk::Image,
    pub gbuffer_depth_image_memory: vk::DeviceMemory,
    pub gbuffer_depth_image_view: vk::ImageView,
}

impl RRRender {
    pub unsafe fn new(
        instance: &Instance,
        rrdevice: &RRDevice,
        rrswapchain: &RRSwapchain,
        rrcommand_pool: &RRCommandPool,
    ) -> Self {
        println!("start to create render ...");
        let mut rrrender = RRRender::default();
        if let Err(e) = create_render_pass(instance, rrdevice, rrswapchain, &mut rrrender) {
            eprintln!("Create render pass failed {:?}", e);
        }
        if let Err(e) = create_depth_objects(
            instance,
            rrdevice,
            rrswapchain,
            rrcommand_pool,
            &mut rrrender,
        ) {
            eprintln!("Create render pass failed {:?}", e);
        }
        if let Err(e) = create_color_objects(instance, rrdevice, rrswapchain, &mut rrrender) {
            eprintln!("Create color objects failed {:?}", e);
        }
        if let Err(e) = create_framebuffers(rrdevice, rrswapchain, &mut rrrender) {
            eprintln!("Create framebuffers failed {:?}", e);
        }
        println!("created render pass {:?}", rrrender);
        rrrender
    }
}

unsafe fn create_render_pass(
    instance: &Instance,
    rrdevice: &RRDevice,
    rrswapchain: &RRSwapchain,
    rrrender: &mut RRRender,
) -> Result<()> {
    // we need to tell Vulkan about the framebuffer attachments that will be used while rendering.
    // We need to specify how many color and depth buffers there will be, how many samples to use for each of them and how their contents should be handled throughout the rendering operations.
    // All of this information is wrapped in a render pass object
    let color_attachment = vk::AttachmentDescription::builder()
        .format(rrswapchain.swapchain_format)
        .samples(rrdevice.msaa_samples)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE) // for stencil buffer
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE) // for stencil buffer
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

    // That's because multisampled images cannot be presented directly.
    // We first need to resolve them to a regular image.
    let color_resolve_attachment = vk::AttachmentDescription::builder()
        .format(rrswapchain.swapchain_format)
        .samples(vk::SampleCountFlags::_1)
        .load_op(vk::AttachmentLoadOp::DONT_CARE)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

    //  Subpasses are subsequent rendering operations that depend on the contents of framebuffers in previous passes
    let color_attachment_ref = vk::AttachmentReference::builder()
        .attachment(0)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

    let color_resolve_attachement_ref = vk::AttachmentReference::builder()
        .attachment(2)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

    // The index of the attachment in this array is directly referenced from the fragment shader with the layout(location = 0) out vec4 outColor directive!
    let color_attachments = &[color_attachment_ref];
    let resolve_attachments = &[color_resolve_attachement_ref];

    let depth_stencil_attachment = vk::AttachmentDescription::builder()
        .format(get_depth_format(instance, rrdevice)?)
        .samples(rrdevice.msaa_samples)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::DONT_CARE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

    let depth_stencil_attachment_ref = vk::AttachmentReference::builder()
        .attachment(1)
        .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

    let subpass = vk::SubpassDescription::builder()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(color_attachments)
        .depth_stencil_attachment(&depth_stencil_attachment_ref)
        .resolve_attachments(resolve_attachments);

    // The subpasses in a render pass automatically take care of image layout transitions.
    // These transitions are controlled by subpass dependencies, which specify memory and execution dependencies between subpasses
    // The depth image is first accessed in the early fragment test pipeline stage
    // and because we have a load operation that clears, we should specify the access mask for writes.
    let dependency = vk::SubpassDependency::builder()
        .src_subpass(vk::SUBPASS_EXTERNAL)
        .dst_subpass(0)
        .src_stage_mask(
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
        ) //  wait for the swapchain to finish reading from the image before we can access it
        .src_access_mask(vk::AccessFlags::empty())
        .dst_stage_mask(
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
        )
        .dst_access_mask(
            vk::AccessFlags::COLOR_ATTACHMENT_WRITE
                | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
        );

    let attachments = &[
        color_attachment,
        depth_stencil_attachment,
        color_resolve_attachment,
    ];
    let subpasses = &[subpass];
    let dependencies = &[dependency];
    let info = vk::RenderPassCreateInfo::builder()
        .attachments(attachments)
        .subpasses(subpasses)
        .dependencies(dependencies);

    rrrender.render_pass = rrdevice.device.create_render_pass(&info, None)?;
    println!("render pass {:?}", rrrender);
    Ok(())
}

unsafe fn get_depth_format(instance: &Instance, rrdevice: &RRDevice) -> Result<vk::Format> {
    let candidates = &[
        vk::Format::D32_SFLOAT,
        vk::Format::D32_SFLOAT_S8_UINT,
        vk::Format::D24_UNORM_S8_UINT,
    ];

    get_suppoted_format(
        instance,
        rrdevice,
        candidates,
        vk::ImageTiling::OPTIMAL,
        vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT,
    )
}

unsafe fn get_suppoted_format(
    instance: &Instance,
    rrdevice: &RRDevice,
    candidates: &[vk::Format],
    tiling: vk::ImageTiling,
    features: vk::FormatFeatureFlags,
) -> Result<vk::Format> {
    candidates
        .iter()
        .cloned()
        .find(|f| {
            let properties =
                instance.get_physical_device_format_properties(rrdevice.physical_device, *f);
            match tiling {
                vk::ImageTiling::LINEAR => properties.linear_tiling_features.contains(features),
                vk::ImageTiling::OPTIMAL => properties.optimal_tiling_features.contains(features),
                _ => false,
            }
        })
        .ok_or_else(|| anyhow!("Failed to find supported format"))
}

unsafe fn create_depth_objects(
    instance: &Instance,
    rrdevice: &RRDevice,
    rrswapchain: &RRSwapchain,
    rrcommand_buffer: &RRCommandPool,
    rrrender: &mut RRRender,
) -> Result<()> {
    // The stencil component is used for stencil tests, which is an additional test that can be combined with depth testing.
    let format = get_depth_format(instance, rrdevice)?;
    let (depth_image, depth_image_memory) = create_image(
        instance,
        rrdevice,
        rrswapchain.swapchain_extent.width,
        rrswapchain.swapchain_extent.height,
        1,
        rrdevice.msaa_samples,
        format,
        vk::ImageTiling::OPTIMAL,
        vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;
    rrrender.depth_image = depth_image;
    rrrender.depth_image_memory = depth_image_memory;
    rrrender.depth_image_view = create_image_view(
        rrdevice,
        rrrender.depth_image,
        format,
        vk::ImageAspectFlags::DEPTH,
        1,
    )?;

    transition_image_layout(
        rrdevice,
        rrdevice.graphics_queue,
        rrcommand_buffer.command_pool,
        rrrender.depth_image,
        format,
        vk::ImageLayout::UNDEFINED,
        vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        1,
    )?;
    println!("created depth object");
    Ok(())
}

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
#[derive(Clone, Debug, Default)]
pub struct RRGBuffer {
    // World space position (RGB = xyz, A = unused)
    pub position_image: vk::Image,
    pub position_image_memory: vk::DeviceMemory,
    pub position_image_view: vk::ImageView,

    // World space normal (RGB = xyz, A = unused)
    pub normal_image: vk::Image,
    pub normal_image_memory: vk::DeviceMemory,
    pub normal_image_view: vk::ImageView,

    // Shadow mask (R = shadow factor, 0.0 = shadowed, 1.0 = lit)
    pub shadow_mask_image: vk::Image,
    pub shadow_mask_image_memory: vk::DeviceMemory,
    pub shadow_mask_image_view: vk::ImageView,

    pub width: u32,
    pub height: u32,
}

impl RRGBuffer {
    /// Create G-Buffer images for the given resolution
    pub unsafe fn new(
        instance: &Instance,
        rrdevice: &RRDevice,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        // Create position buffer (RGBA32F for high precision world coordinates)
        let (position_image, position_image_memory) = create_image(
            instance,
            rrdevice,
            width,
            height,
            1, // mip_levels
            vk::SampleCountFlags::_1,
            vk::Format::R32G32B32A32_SFLOAT,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::STORAGE,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        let position_image_view = create_image_view(
            rrdevice,
            position_image,
            vk::Format::R32G32B32A32_SFLOAT,
            vk::ImageAspectFlags::COLOR,
            1,
        )?;

        // Create normal buffer (RGBA32F for normals)
        let (normal_image, normal_image_memory) = create_image(
            instance,
            rrdevice,
            width,
            height,
            1,
            vk::SampleCountFlags::_1,
            vk::Format::R32G32B32A32_SFLOAT,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::STORAGE,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        let normal_image_view = create_image_view(
            rrdevice,
            normal_image,
            vk::Format::R32G32B32A32_SFLOAT,
            vk::ImageAspectFlags::COLOR,
            1,
        )?;

        // Create shadow mask buffer (R32F for shadow factor)
        let (shadow_mask_image, shadow_mask_image_memory) = create_image(
            instance,
            rrdevice,
            width,
            height,
            1,
            vk::SampleCountFlags::_1,
            vk::Format::R32_SFLOAT,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        let shadow_mask_image_view = create_image_view(
            rrdevice,
            shadow_mask_image,
            vk::Format::R32_SFLOAT,
            vk::ImageAspectFlags::COLOR,
            1,
        )?;

        log::info!(
            "Created G-Buffer: {}x{} (position, normal, shadow mask)",
            width,
            height
        );

        Ok(Self {
            position_image,
            position_image_memory,
            position_image_view,
            normal_image,
            normal_image_memory,
            normal_image_view,
            shadow_mask_image,
            shadow_mask_image_memory,
            shadow_mask_image_view,
            width,
            height,
        })
    }

    /// Destroy all G-Buffer resources
    pub unsafe fn destroy(&mut self, device: &vulkanalia::Device) {
        device.destroy_image_view(self.position_image_view, None);
        device.destroy_image(self.position_image, None);
        device.free_memory(self.position_image_memory, None);

        device.destroy_image_view(self.normal_image_view, None);
        device.destroy_image(self.normal_image, None);
        device.free_memory(self.normal_image_memory, None);

        device.destroy_image_view(self.shadow_mask_image_view, None);
        device.destroy_image(self.shadow_mask_image, None);
        device.free_memory(self.shadow_mask_image_memory, None);

        log::info!("Destroyed G-Buffer");
    }

    /// Transition all G-Buffer images to the appropriate layouts
    pub unsafe fn transition_layouts(
        &self,
        rrdevice: &RRDevice,
        command_pool: vk::CommandPool,
    ) -> Result<()> {
        // Transition position and normal to COLOR_ATTACHMENT_OPTIMAL for rendering
        transition_image_layout(
            rrdevice,
            rrdevice.graphics_queue,
            command_pool,
            self.position_image,
            vk::Format::R32G32B32A32_SFLOAT,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            1,
        )?;

        transition_image_layout(
            rrdevice,
            rrdevice.graphics_queue,
            command_pool,
            self.normal_image,
            vk::Format::R32G32B32A32_SFLOAT,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            1,
        )?;

        // Transition shadow mask to GENERAL for compute shader access
        transition_image_layout(
            rrdevice,
            rrdevice.graphics_queue,
            command_pool,
            self.shadow_mask_image,
            vk::Format::R32_SFLOAT,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::GENERAL,
            1,
        )?;

        Ok(())
    }
}

/// Create G-Buffer render pass with MRT (Multiple Render Targets)
pub unsafe fn create_gbuffer_render_pass(
    instance: &Instance,
    rrdevice: &RRDevice,
    rrrender: &mut RRRender,
) -> Result<()> {
    // Attachment 0: Position (RGBA32F)
    let position_attachment = vk::AttachmentDescription::builder()
        .format(vk::Format::R32G32B32A32_SFLOAT)
        .samples(vk::SampleCountFlags::_1)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::GENERAL) // For compute shader read
        .build();

    // Attachment 1: Normal (RGBA32F)
    let normal_attachment = vk::AttachmentDescription::builder()
        .format(vk::Format::R32G32B32A32_SFLOAT)
        .samples(vk::SampleCountFlags::_1)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::GENERAL) // For compute shader read
        .build();

    // Attachment 2: Depth
    let depth_attachment = vk::AttachmentDescription::builder()
        .format(get_depth_format(instance, rrdevice)?)
        .samples(vk::SampleCountFlags::_1)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::DONT_CARE) // Don't need to keep depth after G-Buffer pass
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
        .build();

    // Color attachment references
    let position_attachment_ref = vk::AttachmentReference::builder()
        .attachment(0)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .build();

    let normal_attachment_ref = vk::AttachmentReference::builder()
        .attachment(1)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .build();

    let color_attachments = [position_attachment_ref, normal_attachment_ref];

    // Depth attachment reference
    let depth_attachment_ref = vk::AttachmentReference::builder()
        .attachment(2)
        .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

    // Subpass
    let subpass = vk::SubpassDescription::builder()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(&color_attachments)
        .depth_stencil_attachment(&depth_attachment_ref);

    // Subpass dependencies
    let dependency = vk::SubpassDependency::builder()
        .src_subpass(vk::SUBPASS_EXTERNAL)
        .dst_subpass(0)
        .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS)
        .src_access_mask(vk::AccessFlags::empty())
        .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS)
        .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE);

    let attachments = [position_attachment, normal_attachment, depth_attachment];
    let subpasses = [subpass];
    let dependencies = [dependency];

    let info = vk::RenderPassCreateInfo::builder()
        .attachments(&attachments)
        .subpasses(&subpasses)
        .dependencies(&dependencies);

    rrrender.gbuffer_render_pass = rrdevice.device.create_render_pass(&info, None)?;

    log::info!("Created G-Buffer render pass");
    Ok(())
}

/// Create G-Buffer framebuffer
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

    // Create framebuffer with position, normal, and depth attachments
    let attachments = [
        gbuffer.position_image_view,
        gbuffer.normal_image_view,
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
