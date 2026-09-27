#[derive(Clone, Debug)]
pub struct MorphWeights {
    pub weights: Vec<f32>,
}

impl MorphWeights {
    pub fn zeroed(channel_count: usize) -> Self {
        Self {
            weights: vec![0.0; channel_count],
        }
    }
}

#[derive(Clone, Debug)]
pub struct AppliedMorphWeights {
    pub weights: Vec<f32>,
}
