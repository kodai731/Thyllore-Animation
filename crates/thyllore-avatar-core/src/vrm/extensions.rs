use std::collections::BTreeMap;

use serde_json::Value;

use super::humanoid::{build_vrmc_vrm, VRMC_VRM};
use super::meta::VrmMeta;
use super::spring_bone::{build_vrmc_spring_bone, SpringBoneInput, VRMC_SPRING_BONE};
use crate::humanoid::components::skeleton_input::BoneInput;
use crate::vrchat::sidecar::AvatarSidecar;
use crate::vrchat::sidecar_spring::BoneRestFrame;

pub const THYLLORE_AVATAR: &str = "THYLLORE_avatar";

/// Everything the VRM writer needs besides the GLB: the sidecar, the skeleton it was authored on
/// (rest positions in metres, rest frames in bone order), the glTF node of each bone and the meta.
pub struct VrmInput<'a> {
    pub sidecar: &'a AvatarSidecar,
    pub bones: &'a [BoneInput],
    pub frames: &'a [BoneRestFrame],
    pub node_of_bone: &'a BTreeMap<String, u32>,
    pub meta: &'a VrmMeta,
}

/// The glTF root extensions of a `.vrm`, in the order they are listed in `extensionsUsed`.
#[derive(Clone, Debug, PartialEq)]
pub struct VrmExtensions {
    pub entries: Vec<(&'static str, Value)>,
}

impl VrmExtensions {
    pub fn names(&self) -> Vec<&'static str> {
        self.entries.iter().map(|(name, _)| *name).collect()
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.entries
            .iter()
            .find(|(candidate, _)| *candidate == name)
            .map(|(_, value)| value)
    }
}

/// The standard layer (`VRMC_vrm`, `VRMC_springBone`) and the vendor layer carrying the whole sidecar.
pub fn build_vrm_extensions(input: VrmInput<'_>) -> anyhow::Result<VrmExtensions> {
    let vrmc_vrm = build_vrmc_vrm(&input.sidecar.humanoid, input.node_of_bone, input.meta)?;
    let vrmc_spring_bone = build_vrmc_spring_bone(SpringBoneInput {
        spring: &input.sidecar.spring,
        bones: input.bones,
        frames: input.frames,
        node_of_bone: input.node_of_bone,
    })?;
    let thyllore_avatar = serde_json::to_value(input.sidecar)?;

    Ok(VrmExtensions {
        entries: vec![
            (VRMC_VRM, vrmc_vrm),
            (VRMC_SPRING_BONE, vrmc_spring_bone),
            (THYLLORE_AVATAR, thyllore_avatar),
        ],
    })
}
