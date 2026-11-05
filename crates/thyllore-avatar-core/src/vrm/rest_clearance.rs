use std::collections::BTreeMap;

use cgmath::{Matrix4, Vector3};
use thyllore_anim_core::spring_bone::{spring_bone_initialize, SpringSpace};
use thyllore_anim_core::{compute_pose_global_transforms, create_pose_from_rest, Skeleton};

use crate::humanoid::components::skeleton_input::BoneInput;
use crate::vrchat::sidecar_spring::{spring_setup_with_collider_ids, BoneRestFrame, SidecarSpring};

/// A collider the VRM file shrinks because a joint rests inside it; radii in model metres.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct VrmColliderShrink {
    pub collider: String,
    pub chain: String,
    pub bone: String,
    pub from_m: f32,
    pub to_m: f32,
}

/// The rest tolerance of every enabled chain projected onto the radii VRM carries, in model metres:
/// the hit radius of each joint by chain and bone, and the radius of each shrunk collider.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VrmRestClearance {
    hit_radii: BTreeMap<(String, String), f32>,
    collider_radii: BTreeMap<String, f32>,
    pub shrinks: Vec<VrmColliderShrink>,
}

impl VrmRestClearance {
    pub fn hit_radius(&self, chain: &str, bone: &str) -> Option<f32> {
        self.hit_radii
            .get(&(chain.to_string(), bone.to_string()))
            .copied()
    }

    pub fn collider_radius(&self, collider: &str) -> Option<f32> {
        self.collider_radii.get(collider).copied()
    }
}

/// Runs the engine's rest initialisation on the sidecar in model space so the VRM file, the preview
/// and the engine share one projection.
pub fn vrm_rest_clearance(spring: &SidecarSpring, bones: &[BoneInput]) -> VrmRestClearance {
    let skeleton = rest_skeleton(bones);
    let (setup, collider_ids) =
        spring_setup_with_collider_ids(spring, bones, &BoneRestFrame::identity_for(bones.len()));
    let rest_globals = compute_pose_global_transforms(&skeleton, &create_pose_from_rest(&skeleton));
    let simulation =
        spring_bone_initialize(&setup, &skeleton, &rest_globals, SpringSpace::IDENTITY);

    let enabled_chains = setup.chains.iter().filter(|chain| chain.enabled);
    let hit_radii = enabled_chains
        .zip(&simulation.chain_states)
        .flat_map(|(chain, state)| {
            chain
                .joints
                .iter()
                .zip(&state.joint_states)
                .map(|(joint, joint_state)| {
                    let bone = skeleton.bones[joint.bone_id as usize].name.clone();
                    ((chain.name.clone(), bone), joint_state.hit_radius)
                })
        })
        .collect();

    let collider_name = |id: u32| {
        collider_ids
            .iter()
            .find(|(_, candidate)| *candidate == id)
            .map(|(name, _)| name.clone())
    };
    let chain_name = |id: u32| {
        setup
            .chains
            .iter()
            .find(|chain| chain.id == id)
            .map(|chain| chain.name.clone())
    };
    let shrinks: Vec<VrmColliderShrink> = simulation
        .collider_shrinks
        .iter()
        .filter_map(|shrink| {
            Some(VrmColliderShrink {
                collider: collider_name(shrink.collider_id)?,
                chain: chain_name(shrink.chain_id)?,
                bone: skeleton.bones.get(shrink.bone_id as usize)?.name.clone(),
                from_m: shrink.from,
                to_m: shrink.to,
            })
        })
        .collect();
    let collider_radii = shrinks
        .iter()
        .map(|shrink| (shrink.collider.clone(), shrink.to_m))
        .collect();

    VrmRestClearance {
        hit_radii,
        collider_radii,
        shrinks,
    }
}

fn rest_skeleton(bones: &[BoneInput]) -> Skeleton {
    let mut skeleton = Skeleton::new("vrm_rest");
    for bone in bones {
        let bone_id = skeleton.add_bone(&bone.name, bone.parent.map(|parent| parent as u32));
        let parent_position = bone
            .parent
            .and_then(|parent| bones.get(parent))
            .map(|parent| Vector3::from(parent.rest_position))
            .unwrap_or(Vector3::new(0.0, 0.0, 0.0));
        skeleton.bones[bone_id as usize].local_transform =
            Matrix4::from_translation(Vector3::from(bone.rest_position) - parent_position);
    }
    skeleton
}
