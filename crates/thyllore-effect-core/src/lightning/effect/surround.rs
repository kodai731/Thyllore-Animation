use crate::lightning::LightningParameterOwner;

#[derive(Clone, Debug, PartialEq, thyllore_scene_core::SceneFields)]
#[params(tag = LightningParameterOwner, owner = Frame, group = "surround")]
pub struct LightningSurround {
    /// Intensity of the point light emitted at the discharge; 0 disables it
    #[persist(ui(min = 0.0, max = 10.0, format = "%.2f"))]
    pub light_gain: f32,
    #[persist(ui(min = 0.0, max = 1.0, format = "%.2f"))]
    pub light_color: [f32; 3],
    /// Where along the bolt (0=start, 1=end) the light is placed
    #[persist(ui(min = 0.0, max = 1.0, format = "%.2f"))]
    pub light_height_fraction: f32,
    /// Radius of the impact decal in meters; 0 disables it
    #[persist(ui(min = 0.0, max = 10.0, format = "%.2f"))]
    pub impact_radius: f32,
    /// Time constant of the impact glow exponential decay in seconds
    #[persist(ui(min = 0.01, max = 5.0, format = "%.2f"))]
    pub impact_decay: f32,
    /// Speed of sound in m/s for thunder delay calculation
    #[persist(ui(min = 1.0, max = 1000.0, format = "%.1f"))]
    pub thunder_speed: f32,
}

impl Default for LightningSurround {
    fn default() -> Self {
        Self {
            light_gain: 0.0,
            light_color: [0.85, 0.92, 1.0],
            light_height_fraction: 0.5,
            impact_radius: 0.0,
            impact_decay: 0.5,
            thunder_speed: 343.0,
        }
    }
}
