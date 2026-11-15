#![cfg(test)]
#![allow(dead_code)]

mod canonical_rig;
pub(crate) mod fbx_ascii;
pub(crate) mod fixture_bones;
pub(crate) mod fixtures;
pub(crate) mod reference_pose;
pub(crate) mod rig_convention;
pub(crate) mod rig_names;
pub(crate) mod rig_nodes;
pub(crate) mod rig_positions;
pub(crate) mod role_clips;
pub(crate) mod test_humanoid;

pub(crate) use canonical_rig::{canonical_bones, CanonicalBone};
