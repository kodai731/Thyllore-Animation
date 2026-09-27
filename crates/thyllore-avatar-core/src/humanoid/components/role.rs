use serde::{Deserialize, Serialize};

use crate::expression::components::side::Side;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum HumanoidRole {
    Hips,
    LeftUpperLeg,
    RightUpperLeg,
    LeftLowerLeg,
    RightLowerLeg,
    LeftFoot,
    RightFoot,
    Spine,
    Chest,
    UpperChest,
    Neck,
    Head,
    LeftShoulder,
    RightShoulder,
    LeftUpperArm,
    RightUpperArm,
    LeftLowerArm,
    RightLowerArm,
    LeftHand,
    RightHand,
    LeftToes,
    RightToes,
    LeftEye,
    RightEye,
    Jaw,
    LeftThumbProximal,
    LeftThumbIntermediate,
    LeftThumbDistal,
    LeftIndexProximal,
    LeftIndexIntermediate,
    LeftIndexDistal,
    LeftMiddleProximal,
    LeftMiddleIntermediate,
    LeftMiddleDistal,
    LeftRingProximal,
    LeftRingIntermediate,
    LeftRingDistal,
    LeftLittleProximal,
    LeftLittleIntermediate,
    LeftLittleDistal,
    RightThumbProximal,
    RightThumbIntermediate,
    RightThumbDistal,
    RightIndexProximal,
    RightIndexIntermediate,
    RightIndexDistal,
    RightMiddleProximal,
    RightMiddleIntermediate,
    RightMiddleDistal,
    RightRingProximal,
    RightRingIntermediate,
    RightRingDistal,
    RightLittleProximal,
    RightLittleIntermediate,
    RightLittleDistal,
}

impl HumanoidRole {
    pub const ALL: [HumanoidRole; 55] = [
        HumanoidRole::Hips,
        HumanoidRole::LeftUpperLeg,
        HumanoidRole::RightUpperLeg,
        HumanoidRole::LeftLowerLeg,
        HumanoidRole::RightLowerLeg,
        HumanoidRole::LeftFoot,
        HumanoidRole::RightFoot,
        HumanoidRole::Spine,
        HumanoidRole::Chest,
        HumanoidRole::UpperChest,
        HumanoidRole::Neck,
        HumanoidRole::Head,
        HumanoidRole::LeftShoulder,
        HumanoidRole::RightShoulder,
        HumanoidRole::LeftUpperArm,
        HumanoidRole::RightUpperArm,
        HumanoidRole::LeftLowerArm,
        HumanoidRole::RightLowerArm,
        HumanoidRole::LeftHand,
        HumanoidRole::RightHand,
        HumanoidRole::LeftToes,
        HumanoidRole::RightToes,
        HumanoidRole::LeftEye,
        HumanoidRole::RightEye,
        HumanoidRole::Jaw,
        HumanoidRole::LeftThumbProximal,
        HumanoidRole::LeftThumbIntermediate,
        HumanoidRole::LeftThumbDistal,
        HumanoidRole::LeftIndexProximal,
        HumanoidRole::LeftIndexIntermediate,
        HumanoidRole::LeftIndexDistal,
        HumanoidRole::LeftMiddleProximal,
        HumanoidRole::LeftMiddleIntermediate,
        HumanoidRole::LeftMiddleDistal,
        HumanoidRole::LeftRingProximal,
        HumanoidRole::LeftRingIntermediate,
        HumanoidRole::LeftRingDistal,
        HumanoidRole::LeftLittleProximal,
        HumanoidRole::LeftLittleIntermediate,
        HumanoidRole::LeftLittleDistal,
        HumanoidRole::RightThumbProximal,
        HumanoidRole::RightThumbIntermediate,
        HumanoidRole::RightThumbDistal,
        HumanoidRole::RightIndexProximal,
        HumanoidRole::RightIndexIntermediate,
        HumanoidRole::RightIndexDistal,
        HumanoidRole::RightMiddleProximal,
        HumanoidRole::RightMiddleIntermediate,
        HumanoidRole::RightMiddleDistal,
        HumanoidRole::RightRingProximal,
        HumanoidRole::RightRingIntermediate,
        HumanoidRole::RightRingDistal,
        HumanoidRole::RightLittleProximal,
        HumanoidRole::RightLittleIntermediate,
        HumanoidRole::RightLittleDistal,
    ];
}

pub const REQUIRED: [HumanoidRole; 15] = [
    HumanoidRole::Hips,
    HumanoidRole::Spine,
    HumanoidRole::Head,
    HumanoidRole::LeftUpperArm,
    HumanoidRole::RightUpperArm,
    HumanoidRole::LeftLowerArm,
    HumanoidRole::RightLowerArm,
    HumanoidRole::LeftHand,
    HumanoidRole::RightHand,
    HumanoidRole::LeftUpperLeg,
    HumanoidRole::RightUpperLeg,
    HumanoidRole::LeftLowerLeg,
    HumanoidRole::RightLowerLeg,
    HumanoidRole::LeftFoot,
    HumanoidRole::RightFoot,
];

impl HumanoidRole {
    pub fn side(self) -> Option<Side> {
        match self {
            Self::Hips
            | Self::Spine
            | Self::Chest
            | Self::UpperChest
            | Self::Neck
            | Self::Head
            | Self::Jaw => None,
            Self::LeftUpperLeg
            | Self::LeftLowerLeg
            | Self::LeftFoot
            | Self::LeftShoulder
            | Self::LeftUpperArm
            | Self::LeftLowerArm
            | Self::LeftHand
            | Self::LeftToes
            | Self::LeftEye
            | Self::LeftThumbProximal
            | Self::LeftThumbIntermediate
            | Self::LeftThumbDistal
            | Self::LeftIndexProximal
            | Self::LeftIndexIntermediate
            | Self::LeftIndexDistal
            | Self::LeftMiddleProximal
            | Self::LeftMiddleIntermediate
            | Self::LeftMiddleDistal
            | Self::LeftRingProximal
            | Self::LeftRingIntermediate
            | Self::LeftRingDistal
            | Self::LeftLittleProximal
            | Self::LeftLittleIntermediate
            | Self::LeftLittleDistal => Some(Side::Left),
            Self::RightUpperLeg
            | Self::RightLowerLeg
            | Self::RightFoot
            | Self::RightShoulder
            | Self::RightUpperArm
            | Self::RightLowerArm
            | Self::RightHand
            | Self::RightToes
            | Self::RightEye
            | Self::RightThumbProximal
            | Self::RightThumbIntermediate
            | Self::RightThumbDistal
            | Self::RightIndexProximal
            | Self::RightIndexIntermediate
            | Self::RightIndexDistal
            | Self::RightMiddleProximal
            | Self::RightMiddleIntermediate
            | Self::RightMiddleDistal
            | Self::RightRingProximal
            | Self::RightRingIntermediate
            | Self::RightRingDistal
            | Self::RightLittleProximal
            | Self::RightLittleIntermediate
            | Self::RightLittleDistal => Some(Side::Right),
        }
    }
}
