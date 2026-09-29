#![allow(dead_code)]

pub mod canonical_rig;
pub mod fbx_ascii;
pub mod fixture_bones;
pub mod reference_pose;
pub mod rig_convention;
pub mod rig_names;
pub mod rig_nodes;
pub mod rig_positions;

pub use canonical_rig::{canonical_bones, CanonicalBone};
