use crate::lightning::LightningParameterOwner;
/// Branch tree grown off the main channel.
#[derive(Clone, Debug, PartialEq, thyllore_scene_core::SceneFields)]
#[params(tag = LightningParameterOwner, owner = Frame, group = "branch")]
pub struct LightningBranching {
    /// Recursion depth of the branch tree; 0 leaves the main channel alone
    #[persist(ui(min = 0.0, max = 5.0, format = "%.0f"))]
    pub depth: u32,
    #[persist(ui(min = 0.0, max = 1.0))]
    pub probability: f32,
    /// Number of branches leaving the main channel; the fraction fades the last one
    #[persist(ui(primary, min = 0.0, max = 32.0))]
    pub count: f32,
    /// Where along the main channel (0 = start, 1 = end) branches begin
    #[persist(ui(min = 0.0, max = 1.0))]
    pub zone_start: f32,
    /// Where along the main channel (0 = start, 1 = end) branches stop
    #[persist(ui(min = 0.0, max = 1.0))]
    pub zone_end: f32,
    /// Radians between a branch and its parent channel
    #[persist(ui(primary, min = 0.0, max = 1.57))]
    pub angle: f32,
    #[persist(ui(min = 0.0, max = 1.0))]
    pub length_ratio: f32,
    #[persist(ui(min = 0.0, max = 1.0))]
    pub radius_ratio: f32,
    #[persist(ui(min = 0.0, max = 1.0))]
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
