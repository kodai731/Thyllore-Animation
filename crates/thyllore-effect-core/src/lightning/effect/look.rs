use crate::lightning::LightningParameterOwner;
/// Emission of the core and rim, the beam variant and the screen flash.
#[derive(Clone, Debug, PartialEq, thyllore_scene_core::SceneFields)]
#[params(tag = LightningParameterOwner, owner = Frame, group = "look")]
pub struct LightningLook {
    /// Radiance of the saturated channel core
    #[persist(curve, ui(primary, min = 0.0, max = 200.0, format = "%.1f"))]
    pub core_intensity: f32,
    #[persist(curve, ui(min = 0.0, max = 1.0))]
    pub core_color: [f32; 3],
    /// Radius of the rim sheath as a multiple of the core radius
    #[persist(curve, ui(min = 1.0, max = 20.0))]
    pub rim_ratio: f32,
    #[persist(curve, ui(min = 0.0, max = 100.0))]
    pub rim_intensity: f32,
    #[persist(curve, ui(primary, min = 0.0, max = 1.0))]
    pub rim_color: [f32; 3],
    /// Radius of the beam the arcs wrap around; 0 keeps a bare channel
    #[persist(curve, ui(min = 0.0, max = 10.0))]
    pub beam_radius: f32,
    #[persist(curve, ui(min = 0.0, max = 64.0, format = "%.0f"))]
    pub beam_arc_count: u32,
    /// Radiance of the ambient flash emitted while a stroke is alive; 0 disables it
    #[persist(curve, ui(min = 0.0, max = 10.0))]
    pub flash_gain: f32,
    #[persist(curve, ui(min = 0.0, max = 100.0))]
    pub flash_radius: f32,
}

impl Default for LightningLook {
    fn default() -> Self {
        Self {
            core_intensity: 40.0,
            core_color: [1.0, 1.0, 1.0],
            rim_ratio: 2.0,
            rim_intensity: 10.0,
            rim_color: [0.15, 0.6, 1.0],
            beam_radius: 0.0,
            beam_arc_count: 8,
            flash_gain: 0.0,
            flash_radius: 2.0,
        }
    }
}
