use cgmath::{Matrix4, Vector3, Vector4};
use vulkanalia::prelude::v1_0::*;

use crate::ecs::resource::ProjectionData;
use crate::ecs::World;

const SCISSOR_MARGIN_PX: f32 = 2.0;

#[derive(Clone, Copy)]
struct ScreenBounds {
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
}

impl ScreenBounds {
    fn of_point(x: f32, y: f32) -> Self {
        ScreenBounds {
            min_x: x,
            min_y: y,
            max_x: x,
            max_y: y,
        }
    }

    fn include(self, x: f32, y: f32) -> Self {
        ScreenBounds {
            min_x: self.min_x.min(x),
            min_y: self.min_y.min(y),
            max_x: self.max_x.max(x),
            max_y: self.max_y.max(y),
        }
    }
}

pub(crate) fn full_extent_scissor(extent: vk::Extent2D) -> vk::Rect2D {
    vk::Rect2D::builder()
        .offset(vk::Offset2D { x: 0, y: 0 })
        .extent(extent)
        .build()
}

/// Projects local-space bound corners to a screen scissor. Returns the full extent when the
/// projection is unavailable or a corner is behind the camera, and `None` when every corner is
/// behind the camera or the projected bounds are empty.
pub(crate) fn compute_bounds_scissor(
    world: &World,
    extent: vk::Extent2D,
    model: &Matrix4<f32>,
    corners: impl IntoIterator<Item = Vector3<f32>>,
) -> Option<vk::Rect2D> {
    let projection = world.get_resource::<ProjectionData>();
    compute_projected_bounds_scissor(projection.as_deref(), extent, model, corners)
}

pub(crate) fn compute_projected_bounds_scissor(
    projection: Option<&ProjectionData>,
    extent: vk::Extent2D,
    model: &Matrix4<f32>,
    corners: impl IntoIterator<Item = Vector3<f32>>,
) -> Option<vk::Rect2D> {
    let Some(projection) = projection else {
        return Some(full_extent_scissor(extent));
    };
    let model_view_proj = projection.proj * projection.view * model;
    let clip_corners: Vec<Vector4<f32>> = corners
        .into_iter()
        .map(|corner| model_view_proj * cgmath::vec4(corner.x, corner.y, corner.z, 1.0))
        .collect();

    let corners_behind_camera = clip_corners.iter().filter(|clip| clip.w <= 0.0).count();
    if !clip_corners.is_empty() && corners_behind_camera == clip_corners.len() {
        return None;
    }
    if corners_behind_camera > 0 {
        return Some(full_extent_scissor(extent));
    }

    let mut screen_bounds: Option<ScreenBounds> = None;
    for clip in clip_corners {
        let screen_x = (clip.x / clip.w + 1.0) * 0.5 * extent.width as f32;
        let screen_y = (clip.y / clip.w + 1.0) * 0.5 * extent.height as f32;
        screen_bounds = Some(match screen_bounds {
            None => ScreenBounds::of_point(screen_x, screen_y),
            Some(bounds) => bounds.include(screen_x, screen_y),
        });
    }
    let bounds = screen_bounds?;

    let min_x = (bounds.min_x - SCISSOR_MARGIN_PX).clamp(0.0, extent.width as f32);
    let min_y = (bounds.min_y - SCISSOR_MARGIN_PX).clamp(0.0, extent.height as f32);
    let max_x = (bounds.max_x + SCISSOR_MARGIN_PX).clamp(0.0, extent.width as f32);
    let max_y = (bounds.max_y + SCISSOR_MARGIN_PX).clamp(0.0, extent.height as f32);
    if max_x - min_x < 1.0 || max_y - min_y < 1.0 {
        return None;
    }

    Some(
        vk::Rect2D::builder()
            .offset(vk::Offset2D {
                x: min_x as i32,
                y: min_y as i32,
            })
            .extent(vk::Extent2D {
                width: (max_x - min_x).ceil() as u32,
                height: (max_y - min_y).ceil() as u32,
            })
            .build(),
    )
}
