/// Texel counts of the baked shadow volume along radius, height and angle for one instance.
/// Mirrored in shaders/wind/include/shadow_volume.glsl.
pub const WIND_SHADOW_VOLUME_RADIAL: u32 = 48;
pub const WIND_SHADOW_VOLUME_HEIGHT: u32 = 48;
pub const WIND_SHADOW_VOLUME_THETA: u32 = 64;
/// Instance slots packed side by side along the radius axis.
pub const WIND_SHADOW_VOLUME_SLOTS: u32 = 4;
