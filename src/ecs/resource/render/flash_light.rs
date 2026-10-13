#[derive(Clone, Copy, Debug, Default)]
pub struct FlashLightSource {
    pub position: [f32; 3],
    pub intensity: f32,
    pub color: [f32; 3],
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ImpactDecal {
    pub position: [f32; 3],
    pub strength: f32,
    pub radius: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FlashLightState {
    pub light: Option<FlashLightSource>,
    pub impact: Option<ImpactDecal>,
}
