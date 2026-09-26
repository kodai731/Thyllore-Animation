/// Emission of the core and rim, the beam variant and the screen flash.
#[derive(Clone, Debug, PartialEq)]
pub struct LightningLook {
    pub core_intensity: f32,
    pub core_color: [f32; 3],
    pub rim_ratio: f32,
    pub rim_intensity: f32,
    pub rim_color: [f32; 3],
    pub beam_radius: f32,
    pub beam_arc_count: u32,
    pub flash_gain: f32,
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
