#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FlashLighting {
    pub light_position: [f32; 3],
    pub light_intensity: f32,
    pub light_color: [f32; 3],
    pub impact_position: [f32; 3],
    pub impact_radius: f32,
    pub impact_strength: f32,
}
