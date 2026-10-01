use thyllore_model_core::MeshMorph;

#[derive(Clone, Debug)]
pub struct MorphWeights {
    pub weights: Vec<f32>,
}

impl MorphWeights {
    pub fn from_defaults(morph: &MeshMorph) -> Self {
        Self {
            weights: morph.default_weights(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct AppliedMorphWeights {
    pub weights: Vec<f32>,
}
