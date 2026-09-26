/// Branch tree grown off the main channel.
#[derive(Clone, Debug, PartialEq)]
pub struct LightningBranching {
    pub depth: u32,
    pub probability: f32,
    pub count: f32,
    pub zone_start: f32,
    pub zone_end: f32,
    pub angle: f32,
    pub length_ratio: f32,
    pub radius_ratio: f32,
    pub intensity_ratio: f32,
}

impl Default for LightningBranching {
    fn default() -> Self {
        Self {
            depth: 2,
            probability: 0.3,
            count: 8.0,
            zone_start: 0.0,
            zone_end: 1.0,
            angle: 0.6,
            length_ratio: 0.5,
            radius_ratio: 0.6,
            intensity_ratio: 0.5,
        }
    }
}
