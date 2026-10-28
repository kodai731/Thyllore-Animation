use crate::core::device::RRDevice;
use crate::types::*;
use crate::vulkan::*;
use anyhow::Result;

pub unsafe fn begin_frame_command_buffer(
    rrdevice: &RRDevice,
    command_buffer: CommandBuffer,
) -> Result<()> {
    let command_buffer = command_buffer.raw();
    rrdevice
        .device()
        .reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())?;

    let begin_info =
        vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

    rrdevice
        .device()
        .begin_command_buffer(command_buffer, &begin_info)?;

    Ok(())
}

pub unsafe fn end_frame_command_buffer(
    rrdevice: &RRDevice,
    command_buffer: CommandBuffer,
) -> Result<()> {
    Ok(rrdevice.device().end_command_buffer(command_buffer.raw())?)
}

pub unsafe fn end_render_pass(rrdevice: &RRDevice, command_buffer: CommandBuffer) {
    rrdevice.device().cmd_end_render_pass(command_buffer.raw());
}

pub unsafe fn begin_render_pass_cleared(
    rrdevice: &RRDevice,
    command_buffer: CommandBuffer,
    render_pass: RenderPass,
    framebuffer: Framebuffer,
    extent: Extent2D,
    clear_color: [f32; 4],
    clear_depth: f32,
) {
    let extent = extent.raw();
    let render_area = vk::Rect2D {
        offset: vk::Offset2D { x: 0, y: 0 },
        extent,
    };

    let color_clear_value = vk::ClearValue {
        color: vk::ClearColorValue {
            float32: clear_color,
        },
    };
    let depth_clear_value = vk::ClearValue {
        depth_stencil: vk::ClearDepthStencilValue {
            depth: clear_depth,
            stencil: 0,
        },
    };
    let clear_values = [color_clear_value, depth_clear_value];

    let render_pass_info = vk::RenderPassBeginInfo::builder()
        .render_pass(render_pass.raw())
        .framebuffer(framebuffer.raw())
        .render_area(render_area)
        .clear_values(&clear_values);

    rrdevice.device().cmd_begin_render_pass(
        command_buffer.raw(),
        &render_pass_info,
        vk::SubpassContents::INLINE,
    );
}

pub unsafe fn set_full_viewport_and_scissor(
    rrdevice: &RRDevice,
    command_buffer: CommandBuffer,
    extent: Extent2D,
) {
    let command_buffer = command_buffer.raw();
    let extent = extent.raw();
    let viewport = vk::Viewport {
        x: 0.0,
        y: 0.0,
        width: extent.width as f32,
        height: extent.height as f32,
        min_depth: 0.0,
        max_depth: 1.0,
    };
    rrdevice
        .device()
        .cmd_set_viewport(command_buffer, 0, &[viewport]);

    let scissor = vk::Rect2D {
        offset: vk::Offset2D { x: 0, y: 0 },
        extent,
    };
    rrdevice
        .device()
        .cmd_set_scissor(command_buffer, 0, &[scissor]);
}

pub unsafe fn clear_shadow_mask_to_lit(
    rrdevice: &RRDevice,
    command_buffer: CommandBuffer,
    shadow_mask_image: Image,
) {
    let command_buffer = command_buffer.raw();
    let shadow_mask_image = shadow_mask_image.raw();
    let subresource_range = vk::ImageSubresourceRange {
        aspect_mask: vk::ImageAspectFlags::COLOR,
        base_mip_level: 0,
        level_count: 1,
        base_array_layer: 0,
        layer_count: 1,
    };

    let to_transfer = vk::ImageMemoryBarrier::builder()
        .src_access_mask(vk::AccessFlags::empty())
        .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
        .old_layout(vk::ImageLayout::UNDEFINED)
        .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
        .image(shadow_mask_image)
        .subresource_range(subresource_range)
        .build();

    rrdevice.device().cmd_pipeline_barrier(
        command_buffer,
        vk::PipelineStageFlags::TOP_OF_PIPE,
        vk::PipelineStageFlags::TRANSFER,
        vk::DependencyFlags::empty(),
        &[] as &[vk::MemoryBarrier],
        &[] as &[vk::BufferMemoryBarrier],
        &[to_transfer],
    );

    let clear_value = vk::ClearColorValue {
        float32: [1.0, 1.0, 1.0, 1.0],
    };
    rrdevice.device().cmd_clear_color_image(
        command_buffer,
        shadow_mask_image,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        &clear_value,
        &[subresource_range],
    );

    let to_shader_read = vk::ImageMemoryBarrier::builder()
        .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
        .dst_access_mask(vk::AccessFlags::SHADER_READ)
        .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
        .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
        .image(shadow_mask_image)
        .subresource_range(subresource_range)
        .build();

    rrdevice.device().cmd_pipeline_barrier(
        command_buffer,
        vk::PipelineStageFlags::TRANSFER,
        vk::PipelineStageFlags::FRAGMENT_SHADER,
        vk::DependencyFlags::empty(),
        &[] as &[vk::MemoryBarrier],
        &[] as &[vk::BufferMemoryBarrier],
        &[to_shader_read],
    );
}

pub unsafe fn record_picked_texel_copy(
    rrdevice: &RRDevice,
    command_buffer: CommandBuffer,
    object_id_image: Image,
    position_image: Image,
    staging_buffer: Buffer,
    pixel: (u32, u32),
) {
    let command_buffer = command_buffer.raw();
    let object_id_image = object_id_image.raw();
    let position_image = position_image.raw();
    let staging_buffer = staging_buffer.raw();
    let (px, py) = pixel;

    let subresource_range = vk::ImageSubresourceRange {
        aspect_mask: vk::ImageAspectFlags::COLOR,
        base_mip_level: 0,
        level_count: 1,
        base_array_layer: 0,
        layer_count: 1,
    };

    let barrier_to_transfer = vk::ImageMemoryBarrier::builder()
        .old_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
        .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(object_id_image)
        .subresource_range(subresource_range)
        .src_access_mask(vk::AccessFlags::SHADER_READ)
        .dst_access_mask(vk::AccessFlags::TRANSFER_READ);

    // The position image stays in GENERAL for the deferred pass, so it only needs the
    // write-before-read dependency, not a layout change.
    let position_to_transfer = vk::ImageMemoryBarrier::builder()
        .old_layout(vk::ImageLayout::GENERAL)
        .new_layout(vk::ImageLayout::GENERAL)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(position_image)
        .subresource_range(subresource_range)
        .src_access_mask(vk::AccessFlags::SHADER_READ)
        .dst_access_mask(vk::AccessFlags::TRANSFER_READ);

    rrdevice.device().cmd_pipeline_barrier(
        command_buffer,
        vk::PipelineStageFlags::FRAGMENT_SHADER,
        vk::PipelineStageFlags::TRANSFER,
        vk::DependencyFlags::empty(),
        &[] as &[vk::MemoryBarrier],
        &[] as &[vk::BufferMemoryBarrier],
        &[barrier_to_transfer.build(), position_to_transfer.build()],
    );

    let picked_texel = |buffer_offset: DeviceSize| {
        vk::BufferImageCopy::builder()
            .buffer_offset(buffer_offset)
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            })
            .image_offset(vk::Offset3D {
                x: px as i32,
                y: py as i32,
                z: 0,
            })
            .image_extent(vk::Extent3D {
                width: 1,
                height: 1,
                depth: 1,
            })
            .build()
    };

    rrdevice.device().cmd_copy_image_to_buffer(
        command_buffer,
        object_id_image,
        vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
        staging_buffer,
        &[picked_texel(0)],
    );

    rrdevice.device().cmd_copy_image_to_buffer(
        command_buffer,
        position_image,
        vk::ImageLayout::GENERAL,
        staging_buffer,
        &[picked_texel(crate::resource::READBACK_POSITION_OFFSET)],
    );

    let barrier_to_shader = vk::ImageMemoryBarrier::builder()
        .old_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
        .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(object_id_image)
        .subresource_range(subresource_range)
        .src_access_mask(vk::AccessFlags::TRANSFER_READ)
        .dst_access_mask(vk::AccessFlags::SHADER_READ);

    let position_to_shader = vk::ImageMemoryBarrier::builder()
        .old_layout(vk::ImageLayout::GENERAL)
        .new_layout(vk::ImageLayout::GENERAL)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(position_image)
        .subresource_range(subresource_range)
        .src_access_mask(vk::AccessFlags::TRANSFER_READ)
        .dst_access_mask(vk::AccessFlags::SHADER_READ);

    rrdevice.device().cmd_pipeline_barrier(
        command_buffer,
        vk::PipelineStageFlags::TRANSFER,
        vk::PipelineStageFlags::FRAGMENT_SHADER,
        vk::DependencyFlags::empty(),
        &[] as &[vk::MemoryBarrier],
        &[] as &[vk::BufferMemoryBarrier],
        &[barrier_to_shader.build(), position_to_shader.build()],
    );
}
