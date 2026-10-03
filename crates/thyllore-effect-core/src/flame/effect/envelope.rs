use crate::flame::ParameterOwner;

/// Height envelope of the emission: peak height, base level and tail length.
#[derive(Clone, Copy, Debug, PartialEq, thyllore_scene_core::SceneFields)]
#[params(tag = ParameterOwner, owner = Shape)]
pub struct FlameEnvelope {
    #[persist]
    pub peak: f32,
    #[persist]
    pub base: f32,
    #[persist]
    pub tail: f32,
}

impl Default for FlameEnvelope {
    fn default() -> Self {
        Self {
            peak: 0.25,
            base: 0.05,
            tail: 1.25,
        }
    }
}
