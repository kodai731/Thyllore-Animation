#[derive(Clone, Debug)]
pub struct BloomSettings {
    pub enabled: bool,
    pub intensity: f32,
    pub threshold: f32,
    pub knee: f32,
    pub mip_count: u32,
}

impl Default for BloomSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            intensity: 0.04,
            threshold: 1.0,
            knee: 0.5,
            mip_count: 5,
        }
    }
}
