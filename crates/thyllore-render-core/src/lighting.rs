use cgmath::Vector3;

use crate::luminance_statistics::rec709_luminance;

/// The one editor light: white, with the constant ambient term the composite pass adds.
pub const DIRECT_LIGHT_COLOR: Vector3<f32> = Vector3::new(1.0, 1.0, 1.0);
pub const AMBIENT_INTENSITY: f32 = 0.3;

/// Radiance of a white diffuse surface facing the light: ambient plus the light's luminance.
pub fn lit_white_radiance() -> f32 {
    AMBIENT_INTENSITY
        + rec709_luminance([
            DIRECT_LIGHT_COLOR.x,
            DIRECT_LIGHT_COLOR.y,
            DIRECT_LIGHT_COLOR.z,
        ])
}
