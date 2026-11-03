use cgmath::{Vector3, Vector4};

use crate::SkeletonId;

/// Bone influences stored per vertex: the four slots of `bone_indices` / `bone_weights`.
pub const INFLUENCES_PER_VERTEX: usize = 4;

#[derive(Clone, Debug)]
pub struct SkinData {
    pub skeleton_id: SkeletonId,
    pub bone_indices: Vec<Vector4<u32>>,
    pub bone_weights: Vec<Vector4<f32>>,
    pub base_positions: Vec<Vector3<f32>>,
    pub base_normals: Vec<Vector3<f32>>,
}

impl Default for SkinData {
    fn default() -> Self {
        Self {
            skeleton_id: 0,
            bone_indices: Vec::new(),
            bone_weights: Vec::new(),
            base_positions: Vec::new(),
            base_normals: Vec::new(),
        }
    }
}

impl SkinData {
    pub fn influences(&self, vertex: usize) -> impl Iterator<Item = (u32, f32)> + '_ {
        let indices = &self.bone_indices[vertex];
        let weights = &self.bone_weights[vertex];
        (0..INFLUENCES_PER_VERTEX)
            .map(move |j| (indices[j], weights[j]))
            .filter(|&(_, weight)| weight > 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn influences_skip_zero_weight_slots() {
        let skin = SkinData {
            bone_indices: vec![Vector4::new(1, 2, 3, 4)],
            bone_weights: vec![Vector4::new(0.0, 0.3, 0.0, 0.7)],
            ..Default::default()
        };

        let influences: Vec<_> = skin.influences(0).collect();
        assert_eq!(influences, [(2u32, 0.3), (4u32, 0.7)]);
    }
}
