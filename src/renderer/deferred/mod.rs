use anyhow::{anyhow, Result};
use vulkanalia::prelude::v1_0::*;

use crate::app::App;

pub unsafe fn record_gbuffer_pass(
    app: &App,
    command_buffer: vk::CommandBuffer,
    image_index: usize,
) -> Result<()> {
    let gbuffer = app.data.gbuffer.as_ref()
        .ok_or_else(|| anyhow!("G-Buffer not initialized"))?;
    let gbuffer_pipeline = app.data.gbuffer_pipeline.as_ref()
        .ok_or_else(|| anyhow!("G-Buffer pipeline not initialized"))?;
    let gbuffer_descriptor_set = app.data.gbuffer_descriptor_set.as_ref()
        .ok_or_else(|| anyhow!("G-Buffer descriptor set not initialized"))?;

    let render_area = vk::Rect2D::builder()
        .offset(vk::Offset2D::default())
        .extent(vk::Extent2D {
            width: gbuffer.width,
            height: gbuffer.height,
        });

    let position_clear = vk::ClearValue {
        color: vk::ClearColorValue {
            float32: [0.0, 0.0, 0.0, 0.0],
        },
    };
    let normal_clear = vk::ClearValue {
        color: vk::ClearColorValue {
            float32: [0.0, 0.0, 0.0, 0.0],
        },
    };
    let albedo_clear = vk::ClearValue {
        color: vk::ClearColorValue {
            float32: [0.0, 0.0, 0.0, 0.0],
        },
    };
    let depth_clear = vk::ClearValue {
        depth_stencil: vk::ClearDepthStencilValue {
            depth: 1.0,
            stencil: 0,
        },
    };
    let clear_values = [position_clear, normal_clear, albedo_clear, depth_clear];

    let render_pass_info = vk::RenderPassBeginInfo::builder()
        .render_pass(app.data.rrrender.gbuffer_render_pass)
        .framebuffer(app.data.rrrender.gbuffer_framebuffer)
        .render_area(render_area)
        .clear_values(&clear_values);

    app.rrdevice.device.cmd_begin_render_pass(
        command_buffer,
        &render_pass_info,
        vk::SubpassContents::INLINE,
    );

    app.rrdevice.device.cmd_bind_pipeline(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        gbuffer_pipeline.pipeline,
    );

    let viewport = vk::Viewport::builder()
        .x(0.0)
        .y(0.0)
        .width(gbuffer.width as f32)
        .height(gbuffer.width as f32)
        .min_depth(0.0)
        .max_depth(1.0);

    let scissor = vk::Rect2D::builder()
        .offset(vk::Offset2D { x: 0, y: 0 })
        .extent(vk::Extent2D {
            width: gbuffer.width,
            height: gbuffer.height,
        });

    app.rrdevice.device.cmd_set_viewport(command_buffer, 0, &[viewport]);
    app.rrdevice.device.cmd_set_scissor(command_buffer, 0, &[scissor]);

    for i in 0..gbuffer_descriptor_set.rrdata.len() {
        let rrdata = &gbuffer_descriptor_set.rrdata[i];

        app.rrdevice.device.cmd_bind_vertex_buffers(
            command_buffer,
            0,
            &[rrdata.vertex_buffer.buffer],
            &[0],
        );

        app.rrdevice.device.cmd_bind_index_buffer(
            command_buffer,
            rrdata.index_buffer.buffer,
            0,
            vk::IndexType::UINT32,
        );

        let swapchain_images_len = gbuffer_descriptor_set.descriptor_sets.len() /
            gbuffer_descriptor_set.rrdata.len().max(1);
        let descriptor_set_index = i * swapchain_images_len + image_index;

        app.rrdevice.device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            gbuffer_pipeline.pipeline_layout,
            0,
            &[gbuffer_descriptor_set.descriptor_sets[descriptor_set_index]],
            &[],
        );

        app.rrdevice.device.cmd_draw_indexed(
            command_buffer,
            rrdata.index_buffer.indices,
            1,
            0,
            0,
            0,
        );
    }

    app.rrdevice.device.cmd_end_render_pass(command_buffer);

    Ok(())
}

pub unsafe fn record_ray_query_pass(
    app: &App,
    command_buffer: vk::CommandBuffer,
) -> Result<()> {
    let gbuffer = app.data.gbuffer.as_ref()
        .ok_or_else(|| anyhow!("G-Buffer not initialized"))?;
    let ray_query_pipeline = app.data.ray_query_pipeline.as_ref()
        .ok_or_else(|| anyhow!("Ray Query pipeline not initialized"))?;
    let ray_query_descriptor = app.data.ray_query_descriptor.as_ref()
        .ok_or_else(|| anyhow!("Ray Query descriptor set not initialized"))?;

    let image_barriers = [
        vk::ImageMemoryBarrier::builder()
            .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .dst_access_mask(vk::AccessFlags::SHADER_READ)
            .old_layout(vk::ImageLayout::GENERAL)
            .new_layout(vk::ImageLayout::GENERAL)
            .image(gbuffer.position_image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .build(),
        vk::ImageMemoryBarrier::builder()
            .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .dst_access_mask(vk::AccessFlags::SHADER_READ)
            .old_layout(vk::ImageLayout::GENERAL)
            .new_layout(vk::ImageLayout::GENERAL)
            .image(gbuffer.normal_image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .build(),
        vk::ImageMemoryBarrier::builder()
            .src_access_mask(vk::AccessFlags::empty())
            .dst_access_mask(vk::AccessFlags::SHADER_WRITE)
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::GENERAL)
            .image(gbuffer.shadow_mask_image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .build(),
    ];

    app.rrdevice.device.cmd_pipeline_barrier(
        command_buffer,
        vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        vk::PipelineStageFlags::COMPUTE_SHADER,
        vk::DependencyFlags::empty(),
        &[] as &[vk::MemoryBarrier],
        &[] as &[vk::BufferMemoryBarrier],
        &image_barriers,
    );

    app.rrdevice.device.cmd_bind_pipeline(
        command_buffer,
        vk::PipelineBindPoint::COMPUTE,
        ray_query_pipeline.pipeline,
    );

    app.rrdevice.device.cmd_bind_descriptor_sets(
        command_buffer,
        vk::PipelineBindPoint::COMPUTE,
        ray_query_pipeline.pipeline_layout,
        0,
        &[ray_query_descriptor.descriptor_set],
        &[],
    );

    let group_count_x = (gbuffer.width + 15) / 16;
    let group_count_y = (gbuffer.height + 15) / 16;
    app.rrdevice.device.cmd_dispatch(command_buffer, group_count_x, group_count_y, 1);

    let shadow_barrier = vk::ImageMemoryBarrier::builder()
        .src_access_mask(vk::AccessFlags::SHADER_WRITE)
        .dst_access_mask(vk::AccessFlags::SHADER_READ)
        .old_layout(vk::ImageLayout::GENERAL)
        .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
        .image(gbuffer.shadow_mask_image)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        })
        .build();

    app.rrdevice.device.cmd_pipeline_barrier(
        command_buffer,
        vk::PipelineStageFlags::COMPUTE_SHADER,
        vk::PipelineStageFlags::FRAGMENT_SHADER,
        vk::DependencyFlags::empty(),
        &[] as &[vk::MemoryBarrier],
        &[] as &[vk::BufferMemoryBarrier],
        &[shadow_barrier],
    );

    Ok(())
}

pub unsafe fn record_composite_pass(
    app: &mut App,
    command_buffer: vk::CommandBuffer,
    image_index: usize,
    draw_data: &imgui::DrawData,
) -> Result<()> {
    let composite_pipeline = app.data.composite_pipeline.as_ref()
        .ok_or_else(|| anyhow!("Composite pipeline not initialized"))?;
    let composite_descriptor = app.data.composite_descriptor.as_ref()
        .ok_or_else(|| anyhow!("Composite descriptor set not initialized"))?;

    let render_area = vk::Rect2D::builder()
        .offset(vk::Offset2D::default())
        .extent(app.data.rrswapchain.swapchain_extent);

    let color_clear_value = vk::ClearValue {
        color: vk::ClearColorValue {
            float32: [0.0, 0.0, 0.0, 1.0],
        },
    };
    let depth_clear_value = vk::ClearValue {
        depth_stencil: vk::ClearDepthStencilValue {
            depth: 1.0,
            stencil: 0,
        },
    };
    let clear_values = [color_clear_value, depth_clear_value];

    let render_pass_info = vk::RenderPassBeginInfo::builder()
        .render_pass(app.data.rrrender.render_pass)
        .framebuffer(app.data.rrrender.framebuffers[image_index])
        .render_area(render_area)
        .clear_values(&clear_values);

    app.rrdevice.device.cmd_begin_render_pass(
        command_buffer,
        &render_pass_info,
        vk::SubpassContents::INLINE,
    );

    app.rrdevice.device.cmd_bind_pipeline(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        composite_pipeline.pipeline,
    );

    let viewport = vk::Viewport::builder()
        .x(0.0)
        .y(0.0)
        .width(app.data.rrswapchain.swapchain_extent.width as f32)
        .height(app.data.rrswapchain.swapchain_extent.height as f32)
        .min_depth(0.0)
        .max_depth(1.0);

    let scissor = vk::Rect2D::builder()
        .offset(vk::Offset2D { x: 0, y: 0 })
        .extent(app.data.rrswapchain.swapchain_extent);

    app.rrdevice.device.cmd_set_viewport(command_buffer, 0, &[viewport]);
    app.rrdevice.device.cmd_set_scissor(command_buffer, 0, &[scissor]);

    app.rrdevice.device.cmd_bind_descriptor_sets(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        composite_pipeline.pipeline_layout,
        0,
        &[composite_descriptor.descriptor_set],
        &[],
    );

    use rust_rendering::debugview::DebugViewMode;
    let debug_view_mode_value = match app.data.rt_debug_state.debug_view_mode {
        DebugViewMode::Final => 0,
        DebugViewMode::Position => 1,
        DebugViewMode::Normal => 2,
        DebugViewMode::ShadowMask => 3,
    };

    let push_constants = [debug_view_mode_value];
    let push_constant_bytes = std::slice::from_raw_parts(
        push_constants.as_ptr() as *const u8,
        std::mem::size_of_val(&push_constants),
    );

    app.rrdevice.device.cmd_push_constants(
        command_buffer,
        composite_pipeline.pipeline_layout,
        vk::ShaderStageFlags::FRAGMENT,
        0,
        push_constant_bytes,
    );

    app.rrdevice.device.cmd_draw(command_buffer, 3, 1, 0, 0);

    // Draw grid
    app.rrdevice.device.cmd_bind_pipeline(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        app.data.grid_pipeline.pipeline,
    );

    app.rrdevice.device.cmd_set_line_width(command_buffer, 1.0);

    app.rrdevice.device.cmd_bind_vertex_buffers(
        command_buffer,
        0,
        &[app.data.grid_vertex_buffer.buffer],
        &[0],
    );

    app.rrdevice.device.cmd_bind_index_buffer(
        command_buffer,
        app.data.grid_index_buffer.buffer,
        0,
        vk::IndexType::UINT32,
    );

    let swapchain_images_len = app.data.grid_descriptor_set.descriptor_sets.len() /
        app.data.grid_descriptor_set.rrdata.len().max(1);
    let descriptor_set_index = image_index;

    app.rrdevice.device.cmd_bind_descriptor_sets(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        app.data.grid_pipeline.pipeline_layout,
        0,
        &[app.data.grid_descriptor_set.descriptor_sets[descriptor_set_index]],
        &[],
    );

    app.rrdevice.device.cmd_draw_indexed(
        command_buffer,
        app.data.grid_index_buffer.indices,
        1,
        0,
        0,
        0,
    );

    // Draw Gizmo
    if let (Some(vertex_buffer), Some(index_buffer)) =
        (app.data.gizmo_data.vertex_buffer, app.data.gizmo_data.index_buffer) {

        app.rrdevice.device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            app.data.gizmo_pipeline.pipeline,
        );

        app.rrdevice.device.cmd_set_line_width(command_buffer, 1.0);

        app.rrdevice.device.cmd_bind_vertex_buffers(
            command_buffer,
            0,
            &[vertex_buffer],
            &[0],
        );

        app.rrdevice.device.cmd_bind_index_buffer(
            command_buffer,
            index_buffer,
            0,
            vk::IndexType::UINT32,
        );

        let descriptor_set_index = image_index;

        app.rrdevice.device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            app.data.gizmo_pipeline.pipeline_layout,
            0,
            &[app.data.gizmo_descriptor_set.descriptor_sets[descriptor_set_index]],
            &[],
        );

        app.rrdevice.device.cmd_draw_indexed(
            command_buffer,
            app.data.gizmo_data.indices.len() as u32,
            1,
            0,
            0,
            0,
        );
    }

    // Draw ImGui on top
    app.record_imgui_rendering(command_buffer, draw_data)?;

    app.rrdevice.device.cmd_end_render_pass(command_buffer);

    Ok(())
}
