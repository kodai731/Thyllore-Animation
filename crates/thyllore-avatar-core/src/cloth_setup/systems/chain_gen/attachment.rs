use std::borrow::Cow;

use cgmath::Vector3;
use thyllore_model_core::BoneId;

use crate::cloth_setup::components::chain_gen::{ChainGenInput, SkinSurface};

fn heaviest_bone(influences: &[(BoneId, f32)]) -> Option<BoneId> {
    influences
        .iter()
        .filter(|(_, weight)| *weight > 0.0)
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(bone, _)| *bone)
}

/// The skin vertices `root_bone` drives more than any other bone: the surface the chains hang from.
pub fn root_attachment_positions(skins: &[SkinSurface], root_bone: BoneId) -> Vec<Vector3<f32>> {
    skins
        .iter()
        .flat_map(|skin| skin.positions.iter().zip(&skin.influences))
        .filter(|(_, influences)| heaviest_bone(influences) == Some(root_bone))
        .map(|(position, _)| *position)
        .collect()
}

/// The attachment surface when the input has skins, otherwise the target vertices themselves.
pub(crate) fn attachment_or_target<'a>(
    input: &ChainGenInput<'a>,
    vertices: &[u32],
) -> Cow<'a, [Vector3<f32>]> {
    if input.skins.is_empty() {
        return Cow::Owned(
            vertices
                .iter()
                .map(|&vertex| input.positions[vertex as usize])
                .collect(),
        );
    }
    Cow::Owned(root_attachment_positions(input.skins, input.root_bone))
}
