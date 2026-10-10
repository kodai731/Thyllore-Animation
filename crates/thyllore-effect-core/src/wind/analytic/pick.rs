use crate::wind::analytic::slang::{wind_clamp_ray_to_cone_slang, wind_envelope_radius_slang};
use crate::wind::WindUBO;
use cgmath::{InnerSpace, Matrix4, Vector3};

const PICK_T_MAX: f32 = 1e4;

/// Distance along the world ray at which it enters the tornado envelope, if it does.
pub fn pick_wind(
    ray_origin: Vector3<f32>,
    ray_direction: Vector3<f32>,
    inverse_model: Matrix4<f32>,
    ubo: &WindUBO,
) -> Option<f32> {
    let local_origin = (inverse_model * ray_origin.extend(1.0)).truncate();
    let local_direction = (inverse_model * ray_direction.extend(0.0))
        .truncate()
        .normalize();

    wind_clamp_ray_to_cone_slang(ubo, local_origin, local_direction, 0.0, PICK_T_MAX)
        .map(|(t_near, _t_far)| t_near.max(0.0))
}

/// Corners of the local axis-aligned box enclosing the envelope cone frustum.
pub fn wind_local_bounds_corners(ubo: &WindUBO) -> [Vector3<f32>; 8] {
    let radius = wind_envelope_radius_slang(ubo, 0.0).max(wind_envelope_radius_slang(ubo, 1.0));
    let height = ubo.shape[0];
    let mut corners = [Vector3::new(0.0, 0.0, 0.0); 8];
    for (index, corner) in corners.iter_mut().enumerate() {
        corner.x = if index & 1 == 0 { -radius } else { radius };
        corner.y = if index & 2 == 0 { 0.0 } else { height };
        corner.z = if index & 4 == 0 { -radius } else { radius };
    }
    corners
}
