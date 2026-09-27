use super::role::HumanoidRole;

pub const HUMANOID_CHAINS: [&[HumanoidRole]; 5] = [
    &[
        HumanoidRole::Hips,
        HumanoidRole::Spine,
        HumanoidRole::Chest,
        HumanoidRole::UpperChest,
        HumanoidRole::Neck,
        HumanoidRole::Head,
    ],
    &[
        HumanoidRole::LeftShoulder,
        HumanoidRole::LeftUpperArm,
        HumanoidRole::LeftLowerArm,
        HumanoidRole::LeftHand,
    ],
    &[
        HumanoidRole::RightShoulder,
        HumanoidRole::RightUpperArm,
        HumanoidRole::RightLowerArm,
        HumanoidRole::RightHand,
    ],
    &[
        HumanoidRole::LeftUpperLeg,
        HumanoidRole::LeftLowerLeg,
        HumanoidRole::LeftFoot,
        HumanoidRole::LeftToes,
    ],
    &[
        HumanoidRole::RightUpperLeg,
        HumanoidRole::RightLowerLeg,
        HumanoidRole::RightFoot,
        HumanoidRole::RightToes,
    ],
];
