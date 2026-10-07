use crate::expression::components::side::Side;
use crate::humanoid::components::geometry_warning::GeometryWarning;
use crate::humanoid::components::humanoid_frame::HumanoidFrame;
use crate::humanoid::components::mapping::HumanoidMapping;
use crate::humanoid::components::role::HumanoidRole;
use crate::humanoid::components::skeleton_input::BoneInput;

const SYMMETRY_TOLERANCE: f32 = 0.05;
const MIN_LENGTH_RATIO: f32 = 0.5;
const MAX_LENGTH_RATIO: f32 = 2.0;

#[rustfmt::skip]
const ASCENDING_CHAINS: &[&[HumanoidRole]] = &[
    &[HumanoidRole::Hips, HumanoidRole::Spine, HumanoidRole::Chest, HumanoidRole::UpperChest, HumanoidRole::Neck, HumanoidRole::Head],
    &[HumanoidRole::LeftFoot, HumanoidRole::LeftLowerLeg, HumanoidRole::LeftUpperLeg],
    &[HumanoidRole::RightFoot, HumanoidRole::RightLowerLeg, HumanoidRole::RightUpperLeg],
];

#[rustfmt::skip]
const LENGTH_RATIO_LIMBS: &[[HumanoidRole; 3]] = &[
    [HumanoidRole::LeftUpperArm, HumanoidRole::LeftLowerArm, HumanoidRole::LeftHand],
    [HumanoidRole::RightUpperArm, HumanoidRole::RightLowerArm, HumanoidRole::RightHand],
    [HumanoidRole::LeftUpperLeg, HumanoidRole::LeftLowerLeg, HumanoidRole::LeftFoot],
    [HumanoidRole::RightUpperLeg, HumanoidRole::RightLowerLeg, HumanoidRole::RightFoot],
];

pub fn check_mapping_geometry(
    mapping: &HumanoidMapping,
    bones: &[BoneInput],
    frame: &HumanoidFrame,
) -> Vec<GeometryWarning> {
    let mut warnings = Vec::new();
    check_symmetry(mapping, bones, frame, &mut warnings);
    check_ascending_chains(mapping, bones, frame, &mut warnings);
    check_length_ratios(mapping, bones, &mut warnings);
    warnings
}

fn character_position(
    mapping: &HumanoidMapping,
    bones: &[BoneInput],
    frame: &HumanoidFrame,
    role: HumanoidRole,
) -> Option<[f32; 3]> {
    let &bone_index = mapping.by_role.get(&role)?;
    Some(frame.to_character(bones[bone_index].rest_position))
}

fn measure_hips_height(
    mapping: &HumanoidMapping,
    bones: &[BoneInput],
    frame: &HumanoidFrame,
) -> Option<f32> {
    let hips = character_position(mapping, bones, frame, HumanoidRole::Hips)?;
    let left_foot = character_position(mapping, bones, frame, HumanoidRole::LeftFoot)?;
    let right_foot = character_position(mapping, bones, frame, HumanoidRole::RightFoot)?;
    let hips_height = hips[1] - left_foot[1].min(right_foot[1]);
    (hips_height > 0.0).then_some(hips_height)
}

fn check_symmetry(
    mapping: &HumanoidMapping,
    bones: &[BoneInput],
    frame: &HumanoidFrame,
    warnings: &mut Vec<GeometryWarning>,
) {
    let Some(hips_height) = measure_hips_height(mapping, bones, frame) else {
        return;
    };

    for left_role in HumanoidRole::ALL
        .iter()
        .copied()
        .filter(|role| role.side() == Some(Side::Left))
    {
        let right_role = left_role.mirrored();
        let (Some(left), Some(right)) = (
            character_position(mapping, bones, frame, left_role),
            character_position(mapping, bones, frame, right_role),
        ) else {
            continue;
        };

        let distance = (left[1] - right[1]).hypot(left[2] - right[2]);
        if distance / hips_height > SYMMETRY_TOLERANCE {
            warnings.push(GeometryWarning::Asymmetric {
                left: left_role,
                right: right_role,
                distance,
            });
        }
    }
}

fn check_ascending_chains(
    mapping: &HumanoidMapping,
    bones: &[BoneInput],
    frame: &HumanoidFrame,
    warnings: &mut Vec<GeometryWarning>,
) {
    for chain in ASCENDING_CHAINS {
        let mapped: Vec<(HumanoidRole, f32)> = chain
            .iter()
            .filter_map(|&role| {
                character_position(mapping, bones, frame, role).map(|position| (role, position[1]))
            })
            .collect();

        for pair in mapped.windows(2) {
            let [(lower, lower_height), (upper, upper_height)] = [pair[0], pair[1]];
            if upper_height <= lower_height {
                warnings.push(GeometryWarning::NotAscending { lower, upper });
            }
        }
    }
}

fn check_length_ratios(
    mapping: &HumanoidMapping,
    bones: &[BoneInput],
    warnings: &mut Vec<GeometryWarning>,
) {
    for &[upper, lower, end] in LENGTH_RATIO_LIMBS {
        let (Some(&upper_index), Some(&lower_index), Some(&end_index)) = (
            mapping.by_role.get(&upper),
            mapping.by_role.get(&lower),
            mapping.by_role.get(&end),
        ) else {
            continue;
        };

        let upper_length = measure_distance(
            bones[upper_index].rest_position,
            bones[lower_index].rest_position,
        );
        let lower_length = measure_distance(
            bones[lower_index].rest_position,
            bones[end_index].rest_position,
        );
        if lower_length <= 0.0 {
            continue;
        }

        let ratio = upper_length / lower_length;
        if !(MIN_LENGTH_RATIO..=MAX_LENGTH_RATIO).contains(&ratio) {
            warnings.push(GeometryWarning::LengthRatio {
                upper,
                lower,
                ratio,
            });
        }
    }
}

fn measure_distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    let delta = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt()
}
