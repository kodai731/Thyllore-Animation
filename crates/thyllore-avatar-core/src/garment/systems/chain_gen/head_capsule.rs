use crate::collider::components::capsule::HeadCapsule;
use crate::collider::systems::head_capsule::estimate_head_capsule;
use crate::garment::components::chain_gen::{ChainGenInput, ChainGenParams, ChainRole};

use super::frame::{attachment_or_target, collect_target_vertices};

/// The head collider a hair generation stands its chains on; a garment has none.
pub fn hair_head_capsule(input: &ChainGenInput, params: &ChainGenParams) -> Option<HeadCapsule> {
    match params.role {
        ChainRole::Hair => {
            let vertices = collect_target_vertices(input);
            estimate_head_capsule(&attachment_or_target(input, &vertices), input.root_position)
        }
        ChainRole::Garment => None,
    }
}
