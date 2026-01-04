use cgmath::{Vector3, Deg};
use vulkanalia::prelude::v1_0::*;
use crate::log;
use crate::vulkanr::swapchain::RRSwapchain;
use crate::vulkanr::descriptor::RRBillboardDescriptorSet;

pub struct BillboardDebugInfo {
    pub light_position: Vector3<f32>,
    pub camera_position: Vector3<f32>,
    pub camera_direction: Vector3<f32>,
    pub camera_up: Vector3<f32>,
    pub near_plane: f32,
    pub far_plane: f32,
}

pub struct GBufferDebugInfo {
    pub position_image_view: vk::ImageView,
    pub extent_width: u32,
    pub extent_height: u32,
}

pub fn log_billboard_debug_info(
    info: &BillboardDebugInfo,
    swapchain: &RRSwapchain,
    billboard_descriptor_set: &RRBillboardDescriptorSet,
    gbuffer_info: Option<&GBufferDebugInfo>,
    gbuffer_sampler: Option<vk::Sampler>,
) {
    use crate::math::{view, coordinate_system::perspective};

    log!("=== Billboard Depth Debug Info ===");

    log!("Light position: ({:.2}, {:.2}, {:.2})",
         info.light_position.x, info.light_position.y, info.light_position.z);
    log!("Camera position: ({:.2}, {:.2}, {:.2})",
         info.camera_position.x, info.camera_position.y, info.camera_position.z);
    log!("Camera direction: ({:.3}, {:.3}, {:.3})",
         info.camera_direction.x, info.camera_direction.y, info.camera_direction.z);

    let swapchain_extent = swapchain.swapchain_extent;
    log!("Swapchain extent: {}x{}", swapchain_extent.width, swapchain_extent.height);

    if let Some(gbuffer) = gbuffer_info {
        log!("GBuffer:");
        log!("  position_image_view: {:?}", gbuffer.position_image_view);
        log!("  extent: {}x{}", gbuffer.extent_width, gbuffer.extent_height);

        let gbuffer_matches_swapchain =
            gbuffer.extent_width == swapchain_extent.width &&
            gbuffer.extent_height == swapchain_extent.height;
        log!("  GBuffer matches swapchain extent: {}", gbuffer_matches_swapchain);
    } else {
        log!("WARNING: No GBuffer!");
    }

    if let Some(sampler) = gbuffer_sampler {
        log!("GBuffer sampler: {:?}", sampler);
    } else {
        log!("WARNING: No GBuffer sampler!");
    }

    log!("Billboard Descriptor Set:");
    log!("  rrdata count: {}", billboard_descriptor_set.rrdata.len());
    log!("  descriptor_sets count: {}", billboard_descriptor_set.descriptor_sets.len());

    let view_matrix = unsafe { view(info.camera_position, info.camera_direction, info.camera_up) };
    let light_view_pos = view_matrix * cgmath::Vector4::new(
        info.light_position.x,
        info.light_position.y,
        info.light_position.z,
        1.0
    );
    let billboard_view_depth = -light_view_pos.z;

    log!("Billboard (light) in view space:");
    log!("  view_pos: ({:.2}, {:.2}, {:.2})", light_view_pos.x, light_view_pos.y, light_view_pos.z);
    log!("  view_depth (=-view_pos.z): {:.4}", billboard_view_depth);

    let aspect = swapchain_extent.width as f32 / swapchain_extent.height as f32;
    let proj = perspective(Deg(45.0), aspect, info.near_plane, info.far_plane);

    let light_clip_pos = proj * light_view_pos;
    let light_ndc = if light_clip_pos.w.abs() > 0.0001 {
        cgmath::Vector3::new(
            light_clip_pos.x / light_clip_pos.w,
            light_clip_pos.y / light_clip_pos.w,
            light_clip_pos.z / light_clip_pos.w,
        )
    } else {
        cgmath::Vector3::new(0.0, 0.0, 0.0)
    };
    log!("Billboard NDC: ({:.4}, {:.4}, {:.4})", light_ndc.x, light_ndc.y, light_ndc.z);

    let screen_x = (light_ndc.x * 0.5 + 0.5) * swapchain_extent.width as f32;
    let screen_y = (light_ndc.y * 0.5 + 0.5) * swapchain_extent.height as f32;
    log!("Billboard screen position: ({:.1}, {:.1})", screen_x, screen_y);

    let screen_uv_x = screen_x / swapchain_extent.width as f32;
    let screen_uv_y = screen_y / swapchain_extent.height as f32;
    log!("Screen UV at billboard center: ({:.4}, {:.4})", screen_uv_x, screen_uv_y);

    log!("Shader expected calculation:");
    log!("  gl_FragCoord.xy at billboard center = ({:.1}, {:.1})", screen_x, screen_y);
    log!("  positionTexSize = ({}, {})", swapchain_extent.width, swapchain_extent.height);
    log!("  screenUV = gl_FragCoord.xy / positionTexSize = ({:.4}, {:.4})", screen_uv_x, screen_uv_y);

    log!("Near plane: {:.4}, Far plane: {:.1}", info.near_plane, info.far_plane);

    log!("=================================");
}
