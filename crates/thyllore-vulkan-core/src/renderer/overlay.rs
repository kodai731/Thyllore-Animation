use vulkanalia::prelude::v1_0::*;

/// What a fullscreen overlay pass finds in its color attachment when it begins.
#[derive(Clone, Copy, Debug)]
pub enum OverlayAttachmentLoad {
    Keep,
    Clear([f32; 4]),
}

pub unsafe fn begin_overlay_render_pass(
    device: &Device,
    cmd: vk::CommandBuffer,
    render_pass: vk::RenderPass,
    framebuffer: vk::Framebuffer,
    render_area: vk::Rect2D,
    load: OverlayAttachmentLoad,
) {
    let clear_values = match load {
        OverlayAttachmentLoad::Keep => vec![],
        OverlayAttachmentLoad::Clear(color) => vec![vk::ClearValue {
            color: vk::ClearColorValue { float32: color },
        }],
    };
    let render_pass_info = vk::RenderPassBeginInfo::builder()
        .render_pass(render_pass)
        .framebuffer(framebuffer)
        .render_area(render_area)
        .clear_values(&clear_values);
    device.cmd_begin_render_pass(cmd, &render_pass_info, vk::SubpassContents::INLINE);
}

pub unsafe fn set_full_viewport(device: &Device, cmd: vk::CommandBuffer, extent: vk::Extent2D) {
    let viewport = vk::Viewport::builder()
        .x(0.0)
        .y(0.0)
        .width(extent.width as f32)
        .height(extent.height as f32)
        .min_depth(0.0)
        .max_depth(1.0);
    device.cmd_set_viewport(cmd, 0, &[viewport]);
}

pub unsafe fn draw_fullscreen_triangle(device: &Device, cmd: vk::CommandBuffer) {
    device.cmd_draw(cmd, 3, 1, 0, 0);
}
