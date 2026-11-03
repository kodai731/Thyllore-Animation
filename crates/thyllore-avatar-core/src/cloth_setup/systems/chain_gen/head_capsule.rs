use crate::cloth_setup::components::chain_gen::{ChainGenInput, ChainGenParams};
use crate::collider::components::capsule::HeadCapsule;
use crate::collider::systems::head_capsule::estimate_head_capsule;
use crate::humanoid::components::role::HumanoidRole;

use super::attachment::attachment_or_target;
use super::frame::collect_target_vertices;

/// The head collider a generation rooted on the head stands its chains on; other roots have none.
pub fn root_head_capsule(input: &ChainGenInput, params: &ChainGenParams) -> Option<HeadCapsule> {
    if params.root != HumanoidRole::Head {
        return None;
    }
    let vertices = collect_target_vertices(input);
    estimate_head_capsule(&attachment_or_target(input, &vertices), input.root_position)
}
