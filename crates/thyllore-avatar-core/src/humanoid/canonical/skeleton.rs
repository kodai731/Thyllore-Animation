use std::collections::HashMap;

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

#[rustfmt::skip]
const BONE_TABLE: &[(HumanoidRole, Option<HumanoidRole>, [f64; 3], CanonicalTip)] = &[
    (HumanoidRole::Hips, None, [0.0, 0.95, 0.0], CanonicalTip::Child(HumanoidRole::Spine)),
    (HumanoidRole::Spine, Some(HumanoidRole::Hips), [0.0, 1.05, 0.0], CanonicalTip::Child(HumanoidRole::Chest)),
    (HumanoidRole::Chest, Some(HumanoidRole::Spine), [0.0, 1.20, 0.0], CanonicalTip::Child(HumanoidRole::UpperChest)),
    (HumanoidRole::UpperChest, Some(HumanoidRole::Chest), [0.0, 1.32, 0.0], CanonicalTip::Child(HumanoidRole::Neck)),
    (HumanoidRole::Neck, Some(HumanoidRole::UpperChest), [0.0, 1.45, 0.0], CanonicalTip::Child(HumanoidRole::Head)),
    (HumanoidRole::Head, Some(HumanoidRole::Neck), [0.0, 1.55, 0.0], CanonicalTip::Leaf([0.0, 0.18, 0.0])),
    (HumanoidRole::Jaw, Some(HumanoidRole::Head), [0.0, 1.53, 0.05], CanonicalTip::Leaf([0.0, -0.04, 0.02])),
    (HumanoidRole::LeftEye, Some(HumanoidRole::Head), [0.03, 1.62, 0.07], CanonicalTip::Leaf([0.0, 0.0, 0.03])),
    (HumanoidRole::LeftUpperLeg, Some(HumanoidRole::Hips), [0.09, 0.90, 0.0], CanonicalTip::Child(HumanoidRole::LeftLowerLeg)),
    (HumanoidRole::LeftLowerLeg, Some(HumanoidRole::LeftUpperLeg), [0.09, 0.50, 0.0], CanonicalTip::Child(HumanoidRole::LeftFoot)),
    (HumanoidRole::LeftFoot, Some(HumanoidRole::LeftLowerLeg), [0.09, 0.08, 0.0], CanonicalTip::Child(HumanoidRole::LeftToes)),
    (HumanoidRole::LeftToes, Some(HumanoidRole::LeftFoot), [0.09, 0.02, 0.12], CanonicalTip::Leaf([0.0, 0.0, 0.05])),
    (HumanoidRole::LeftShoulder, Some(HumanoidRole::UpperChest), [0.04, 1.40, 0.0], CanonicalTip::Child(HumanoidRole::LeftUpperArm)),
    (HumanoidRole::LeftUpperArm, Some(HumanoidRole::LeftShoulder), [0.17, 1.40, 0.0], CanonicalTip::Child(HumanoidRole::LeftLowerArm)),
    (HumanoidRole::LeftLowerArm, Some(HumanoidRole::LeftUpperArm), [0.45, 1.40, 0.0], CanonicalTip::Child(HumanoidRole::LeftHand)),
    (HumanoidRole::LeftHand, Some(HumanoidRole::LeftLowerArm), [0.70, 1.40, 0.0], CanonicalTip::Child(HumanoidRole::LeftMiddleProximal)),
    (HumanoidRole::LeftThumbProximal, Some(HumanoidRole::LeftHand), [0.73, 1.39, 0.03], CanonicalTip::Child(HumanoidRole::LeftThumbIntermediate)),
    (HumanoidRole::LeftThumbIntermediate, Some(HumanoidRole::LeftThumbProximal), [0.76, 1.39, 0.05], CanonicalTip::Child(HumanoidRole::LeftThumbDistal)),
    (HumanoidRole::LeftThumbDistal, Some(HumanoidRole::LeftThumbIntermediate), [0.78, 1.39, 0.06], CanonicalTip::Leaf([0.02, 0.0, 0.0])),
    (HumanoidRole::LeftIndexProximal, Some(HumanoidRole::LeftHand), [0.78, 1.40, 0.025], CanonicalTip::Child(HumanoidRole::LeftIndexIntermediate)),
    (HumanoidRole::LeftIndexIntermediate, Some(HumanoidRole::LeftIndexProximal), [0.82, 1.40, 0.025], CanonicalTip::Child(HumanoidRole::LeftIndexDistal)),
    (HumanoidRole::LeftIndexDistal, Some(HumanoidRole::LeftIndexIntermediate), [0.85, 1.40, 0.025], CanonicalTip::Leaf([0.02, 0.0, 0.0])),
    (HumanoidRole::LeftMiddleProximal, Some(HumanoidRole::LeftHand), [0.785, 1.40, 0.0], CanonicalTip::Child(HumanoidRole::LeftMiddleIntermediate)),
    (HumanoidRole::LeftMiddleIntermediate, Some(HumanoidRole::LeftMiddleProximal), [0.83, 1.40, 0.0], CanonicalTip::Child(HumanoidRole::LeftMiddleDistal)),
    (HumanoidRole::LeftMiddleDistal, Some(HumanoidRole::LeftMiddleIntermediate), [0.865, 1.40, 0.0], CanonicalTip::Leaf([0.02, 0.0, 0.0])),
    (HumanoidRole::LeftRingProximal, Some(HumanoidRole::LeftHand), [0.78, 1.40, -0.02], CanonicalTip::Child(HumanoidRole::LeftRingIntermediate)),
    (HumanoidRole::LeftRingIntermediate, Some(HumanoidRole::LeftRingProximal), [0.82, 1.40, -0.02], CanonicalTip::Child(HumanoidRole::LeftRingDistal)),
    (HumanoidRole::LeftRingDistal, Some(HumanoidRole::LeftRingIntermediate), [0.85, 1.40, -0.02], CanonicalTip::Leaf([0.02, 0.0, 0.0])),
    (HumanoidRole::LeftLittleProximal, Some(HumanoidRole::LeftHand), [0.77, 1.40, -0.04], CanonicalTip::Child(HumanoidRole::LeftLittleIntermediate)),
    (HumanoidRole::LeftLittleIntermediate, Some(HumanoidRole::LeftLittleProximal), [0.80, 1.40, -0.04], CanonicalTip::Child(HumanoidRole::LeftLittleDistal)),
    (HumanoidRole::LeftLittleDistal, Some(HumanoidRole::LeftLittleIntermediate), [0.825, 1.40, -0.04], CanonicalTip::Leaf([0.02, 0.0, 0.0])),
];

pub fn skeleton() -> Vec<CanonicalBone> {
    let mut bones: Vec<CanonicalBone> = Vec::with_capacity(55);
    let mut role_to_idx: HashMap<HumanoidRole, usize> = HashMap::with_capacity(55);

    for (role, parent_role, position, tip) in BONE_TABLE {
        let idx = bones.len();
        let parent = parent_role.map(|pr| *role_to_idx.get(&pr).unwrap());
        bones.push(CanonicalBone {
            role: *role,
            parent,
            position: *position,
            tip: tip.clone(),
        });
        role_to_idx.insert(*role, idx);
    }

    for (role, parent_role, position, tip) in BONE_TABLE {
        if role.side() != Some(Side::Left) {
            continue;
        }
        let new_role = role.mirrored();
        let new_pos: [f64; 3] = [-position[0], position[1], position[2]];
        let new_tip = mirror_tip(tip);
        let new_parent = parent_role.map(|pr| {
            let mirrored_parent_role = pr.mirrored();
            *role_to_idx.get(&mirrored_parent_role).unwrap()
        });
        let idx = bones.len();
        bones.push(CanonicalBone {
            role: new_role,
            parent: new_parent,
            position: new_pos,
            tip: new_tip,
        });
        role_to_idx.insert(new_role, idx);
    }

    bones
}
