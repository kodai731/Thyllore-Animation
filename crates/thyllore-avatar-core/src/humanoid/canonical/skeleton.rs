use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::expression::components::side::Side;
use crate::humanoid::components::role::HumanoidRole;

#[derive(Clone, Debug)]
pub struct CanonicalBone {
    pub role: HumanoidRole,
    pub parent: Option<usize>,
    pub position: [f64; 3],
    pub tip: CanonicalTip,
}

#[derive(Clone, Debug)]
pub enum CanonicalTip {
    Child(HumanoidRole),
    Leaf([f64; 3]),
}

fn mirror_tip(tip: &CanonicalTip) -> CanonicalTip {
    match tip {
        CanonicalTip::Child(role) => CanonicalTip::Child(role.mirrored()),
        CanonicalTip::Leaf(offset) => CanonicalTip::Leaf([-offset[0], offset[1], offset[2]]),
    }
}

#[derive(Deserialize)]
struct RawSkeleton {
    bone: Vec<RawBone>,
}

#[derive(Deserialize)]
struct RawBone {
    role: String,
    #[serde(default)]
    parent: Option<String>,
    position: [f64; 3],
    #[serde(default)]
    child: Option<String>,
    #[serde(default)]
    leaf: Option<[f64; 3]>,
}

fn load() -> Vec<CanonicalBone> {
    let toml_str = include_str!("../../../data/canonical_skeleton.toml");
    let raw: RawSkeleton =
        toml::from_str(toml_str).expect("failed to parse canonical skeleton TOML");

    let mut bones: Vec<CanonicalBone> = Vec::with_capacity(55);
    let mut role_to_idx: HashMap<HumanoidRole, usize> = HashMap::with_capacity(55);

    for raw_bone in &raw.bone {
        let role = HumanoidRole::from_unity_name(&raw_bone.role)
            .unwrap_or_else(|| panic!("unknown skeleton role: {}", raw_bone.role));
        let parent = raw_bone.parent.as_ref().map(|parent_role_str| {
            let parent_role = HumanoidRole::from_unity_name(parent_role_str)
                .unwrap_or_else(|| panic!("unknown skeleton parent role: {}", parent_role_str));
            *role_to_idx.get(&parent_role).unwrap()
        });
        let tip = match (&raw_bone.child, &raw_bone.leaf) {
            (Some(child_str), None) => {
                let child_role = HumanoidRole::from_unity_name(child_str)
                    .unwrap_or_else(|| panic!("unknown skeleton child role: {}", child_str));
                CanonicalTip::Child(child_role)
            }
            (None, Some(leaf)) => CanonicalTip::Leaf(*leaf),
            _ => panic!(
                "bone {:?} must have exactly one of `child` or `leaf`",
                raw_bone.role
            ),
        };
        let idx = bones.len();
        bones.push(CanonicalBone {
            role,
            parent,
            position: raw_bone.position,
            tip,
        });
        role_to_idx.insert(role, idx);
    }

    let left_indices: Vec<usize> = bones
        .iter()
        .enumerate()
        .filter(|(_, bone)| bone.role.side() == Some(Side::Left))
        .map(|(i, _)| i)
        .collect();

    for left_idx in left_indices {
        let bone = &bones[left_idx];
        let new_role = bone.role.mirrored();
        let new_pos: [f64; 3] = [-bone.position[0], bone.position[1], bone.position[2]];
        let tip = mirror_tip(&bone.tip);
        let new_parent = bone.parent.map(|parent_idx| {
            let parent_role = bones[parent_idx].role.mirrored();
            *role_to_idx.get(&parent_role).unwrap()
        });
        let idx = bones.len();
        bones.push(CanonicalBone {
            role: new_role,
            parent: new_parent,
            position: new_pos,
            tip,
        });
        role_to_idx.insert(new_role, idx);
    }

    bones
}

pub fn skeleton() -> Vec<CanonicalBone> {
    static BONES: OnceLock<Vec<CanonicalBone>> = OnceLock::new();
    BONES.get_or_init(load).clone()
}
