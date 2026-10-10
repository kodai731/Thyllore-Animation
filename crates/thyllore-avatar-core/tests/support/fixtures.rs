#![cfg(test)]

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;
use thyllore_avatar_core::humanoid::canonical::skeleton::skeleton;
use thyllore_avatar_core::humanoid::components::mapping::HumanoidMapping;
use thyllore_avatar_core::humanoid::components::role::HumanoidRole;
use thyllore_avatar_core::humanoid::components::skeleton_input::BoneInput;
use thyllore_avatar_core::humanoid::systems::hierarchy::is_ancestor;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtraParent {
    Role(HumanoidRole),
    Row(usize),
}

#[derive(Clone, Debug)]
pub struct ExtraBone {
    pub name: String,
    pub parent: ExtraParent,
    pub position: [f64; 3],
    pub tip: [f64; 3],
    pub rest_euler_degrees: [f64; 3],
}

#[derive(Clone, Copy, Deserialize, PartialEq)]
enum EntrySide {
    Left,
    Right,
}

#[derive(Deserialize)]
struct ExtraBoneEntry {
    base: String,
    side: Option<EntrySide>,
    parent: ExtraParent,
    position: [f64; 3],
    tip: [f64; 3],
    rest_euler_degrees: [f64; 3],
}

#[derive(Deserialize)]
struct ExtraBoneTable {
    entry: Vec<ExtraBoneEntry>,
}

fn load_extra_bone_entries() -> Vec<ExtraBoneEntry> {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/test_humanoid_extra_bones.toml");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
    let table: ExtraBoneTable =
        toml::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e));
    table.entry
}

fn mirror_parent(entries: &[ExtraBoneEntry], parent: ExtraParent) -> ExtraParent {
    match parent {
        ExtraParent::Role(role) => ExtraParent::Role(role.mirrored()),
        ExtraParent::Row(i) => {
            let left_position = entries[..=i]
                .iter()
                .filter(|entry| entry.side == Some(EntrySide::Left))
                .count()
                - 1;
            ExtraParent::Row(entries.len() + left_position)
        }
    }
}

pub fn extra_bones() -> Vec<ExtraBone> {
    let entries = load_extra_bone_entries();
    let mut output: Vec<ExtraBone> = Vec::new();

    for entry in &entries {
        let name = if entry.side == Some(EntrySide::Left) {
            format!("{}_L", entry.base)
        } else {
            entry.base.clone()
        };
        output.push(ExtraBone {
            name,
            parent: entry.parent,
            position: entry.position,
            tip: entry.tip,
            rest_euler_degrees: entry.rest_euler_degrees,
        });
    }

    for entry in entries
        .iter()
        .filter(|entry| entry.side == Some(EntrySide::Left))
    {
        let [x, y, z] = entry.position;
        let [tip_x, tip_y, tip_z] = entry.tip;
        let [euler_x, euler_y, euler_z] = entry.rest_euler_degrees;
        output.push(ExtraBone {
            name: format!("{}_R", entry.base),
            parent: mirror_parent(&entries, entry.parent),
            position: [-x, y, z],
            tip: [-tip_x, tip_y, tip_z],
            rest_euler_degrees: [euler_x, -euler_y, -euler_z],
        });
    }

    output
}

pub fn test_humanoid_bone_inputs() -> (Vec<BoneInput>, HumanoidMapping) {
    let canonical = skeleton();
    let mut bones: Vec<BoneInput> = canonical
        .iter()
        .enumerate()
        .map(|(i, cb)| BoneInput {
            name: cb.role.unity_name().to_string(),
            parent: cb.parent,
            rest_position: [
                cb.position[0] as f32,
                cb.position[1] as f32,
                cb.position[2] as f32,
            ],
        })
        .collect();

    for (i, eb) in extra_bones().iter().enumerate() {
        let parent_index = match eb.parent {
            ExtraParent::Role(role) => canonical
                .iter()
                .position(|cb| cb.role == role)
                .expect("role not found in skeleton"),
            ExtraParent::Row(row) => bones.len() + row,
        };
        bones.push(BoneInput {
            name: eb.name.clone(),
            parent: Some(parent_index),
            rest_position: [
                eb.position[0] as f32,
                eb.position[1] as f32,
                eb.position[2] as f32,
            ],
        });
    }

    let mut by_role = BTreeMap::new();
    for (i, cb) in canonical.iter().enumerate() {
        by_role.insert(cb.role, i);
    }

    (bones, HumanoidMapping { by_role })
}

pub fn with_swapped_upper_arms() -> (Vec<BoneInput>, HumanoidMapping) {
    let (bones, mut mapping) = test_humanoid_bone_inputs();
    let left_upper_arm_bone = *mapping
        .by_role
        .get(&HumanoidRole::LeftUpperArm)
        .expect("LeftUpperArm not in mapping");
    mapping
        .by_role
        .insert(HumanoidRole::RightUpperArm, left_upper_arm_bone);
    (bones, mapping)
}

pub fn with_bone_in_two_roles() -> (Vec<BoneInput>, HumanoidMapping) {
    let (bones, mut mapping) = test_humanoid_bone_inputs();
    let head_bone = *mapping
        .by_role
        .get(&HumanoidRole::Head)
        .expect("Head not in mapping");
    mapping.by_role.insert(HumanoidRole::Jaw, head_bone);
    (bones, mapping)
}

pub fn without_fingers() -> (Vec<BoneInput>, HumanoidMapping) {
    let (bones, mut mapping) = test_humanoid_bone_inputs();
    for role in HumanoidRole::ALL {
        let name = role.unity_name();
        if name.contains("Thumb")
            || name.contains("Index")
            || name.contains("Middle")
            || name.contains("Ring")
            || name.contains("Little")
        {
            mapping.by_role.remove(&role);
        }
    }
    (bones, mapping)
}

pub fn with_spine_below_hips() -> (Vec<BoneInput>, HumanoidMapping) {
    let (mut bones, mapping) = test_humanoid_bone_inputs();
    let spine_idx = *mapping.by_role.get(&HumanoidRole::Spine).unwrap();
    let hips_idx = *mapping.by_role.get(&HumanoidRole::Hips).unwrap();
    bones[spine_idx].rest_position[1] = bones[hips_idx].rest_position[1] - 0.1;
    (bones, mapping)
}

pub fn with_long_left_upper_arm() -> (Vec<BoneInput>, HumanoidMapping) {
    let (mut bones, mapping) = test_humanoid_bone_inputs();
    let left_upper_arm_idx = *mapping.by_role.get(&HumanoidRole::LeftUpperArm).unwrap();
    let left_lower_arm_idx = *mapping.by_role.get(&HumanoidRole::LeftLowerArm).unwrap();

    let upper_arm_length_x =
        bones[left_lower_arm_idx].rest_position[0] - bones[left_upper_arm_idx].rest_position[0];
    let shift_x = 2.0 * upper_arm_length_x;

    for &bone_index in mapping.by_role.values() {
        if bone_index == left_lower_arm_idx || is_ancestor(&bones, left_lower_arm_idx, bone_index) {
            bones[bone_index].rest_position[0] += shift_x;
        }
    }
    (bones, mapping)
}

pub fn with_asymmetric_hand() -> (Vec<BoneInput>, HumanoidMapping) {
    let (mut bones, mapping) = test_humanoid_bone_inputs();
    let left_hand_idx = *mapping.by_role.get(&HumanoidRole::LeftHand).unwrap();
    bones[left_hand_idx].rest_position[2] += 0.2;
    (bones, mapping)
}
