use crate::humanoid::components::character_frame::CharacterFrame;
use crate::humanoid::components::mapping::HumanoidMapping;
use crate::humanoid::components::role::HumanoidRole;
use crate::humanoid::components::skeleton_input::BoneInput;

pub fn derive_character_frame(
    mapping: &HumanoidMapping,
    bones: &[BoneInput],
) -> Option<CharacterFrame> {
    let hips_idx = *mapping.by_role.get(&HumanoidRole::Hips)?;
    let head_idx = *mapping.by_role.get(&HumanoidRole::Head)?;
    let left_upper_arm_idx = *mapping.by_role.get(&HumanoidRole::LeftUpperArm)?;
    let right_upper_arm_idx = *mapping.by_role.get(&HumanoidRole::RightUpperArm)?;

    let hips_pos = bones[hips_idx].rest_position;
    let head_pos = bones[head_idx].rest_position;
    let left_arm_pos = bones[left_upper_arm_idx].rest_position;
    let right_arm_pos = bones[right_upper_arm_idx].rest_position;

    let mut up = [
        head_pos[0] - hips_pos[0],
        head_pos[1] - hips_pos[1],
        head_pos[2] - hips_pos[2],
    ];
    let up_len = magnitude(&up);
    if up_len < 1e-6 {
        return None;
    }
    normalize_in_place(&mut up);

    let mut right = [
        right_arm_pos[0] - left_arm_pos[0],
        right_arm_pos[1] - left_arm_pos[1],
        right_arm_pos[2] - left_arm_pos[2],
    ];
    remove_parallel_component(&mut right, &up);
    let right_len = magnitude(&right);
    if right_len < 1e-6 {
        return None;
    }
    normalize_in_place(&mut right);

    let forward = cross(&up, &right);

    Some(CharacterFrame { right, up, forward })
}

fn dot(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn magnitude(v: &[f32; 3]) -> f32 {
    dot(v, v).sqrt()
}

fn normalize_in_place(v: &mut [f32; 3]) {
    let len = magnitude(v);
    v[0] /= len;
    v[1] /= len;
    v[2] /= len;
}

fn remove_parallel_component(v: &mut [f32; 3], parallel_to: &[f32; 3]) {
    let proj = dot(v, parallel_to);
    v[0] -= proj * parallel_to[0];
    v[1] -= proj * parallel_to[1];
    v[2] -= proj * parallel_to[2];
}

fn cross(a: &[f32; 3], b: &[f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[cfg(test)]
mod tests {
    fn rotate_and_scale(v: [f32; 3], rx: f32, rz: f32, s: f32) -> [f32; 3] {
        let cos_x = rx.cos();
        let sin_x = rx.sin();
        let out = [
            v[0],
            cos_x * v[1] - sin_x * v[2],
            sin_x * v[1] + cos_x * v[2],
        ];
        let cos_z = rz.cos();
        let sin_z = rz.sin();
        [
            s * (cos_z * out[0] - sin_z * out[1]),
            s * (sin_z * out[0] + cos_z * out[1]),
            s * out[2],
        ]
    }

    use std::collections::BTreeMap;

    use super::*;

    fn tpose_bones() -> Vec<BoneInput> {
        vec![
            BoneInput {
                name: "Hips".to_string(),
                parent: None,
                rest_position: [0.0, 1.0, 0.0],
            },
            BoneInput {
                name: "Spine".to_string(),
                parent: Some(0),
                rest_position: [0.0, 1.1, 0.0],
            },
            BoneInput {
                name: "Chest".to_string(),
                parent: Some(1),
                rest_position: [0.0, 1.3, 0.0],
            },
            BoneInput {
                name: "Neck".to_string(),
                parent: Some(2),
                rest_position: [0.0, 1.5, 0.0],
            },
            BoneInput {
                name: "Head".to_string(),
                parent: Some(3),
                rest_position: [0.0, 1.6, 0.0],
            },
            BoneInput {
                name: "LeftUpperArm".to_string(),
                parent: Some(2),
                rest_position: [0.18, 1.45, 0.0],
            },
            BoneInput {
                name: "RightUpperArm".to_string(),
                parent: Some(2),
                rest_position: [-0.18, 1.45, 0.0],
            },
        ]
    }

    fn tpose_mapping() -> HumanoidMapping {
        let mut by_role = BTreeMap::new();
        by_role.insert(HumanoidRole::Hips, 0);
        by_role.insert(HumanoidRole::Head, 4);
        by_role.insert(HumanoidRole::LeftUpperArm, 5);
        by_role.insert(HumanoidRole::RightUpperArm, 6);
        HumanoidMapping { by_role }
    }

    fn almost_eq(a: [f32; 3], b: [f32; 3], eps: f32) -> bool {
        (a[0] - b[0]).abs() < eps && (a[1] - b[1]).abs() < eps && (a[2] - b[2]).abs() < eps
    }

    #[test]
    fn test_tpose_axes() {
        let bones = tpose_bones();
        let mapping = tpose_mapping();
        let frame = derive_character_frame(&mapping, &bones).unwrap();

        assert!(almost_eq(frame.right, [-1.0, 0.0, 0.0], 1e-5));
        assert!(almost_eq(frame.up, [0.0, 1.0, 0.0], 1e-5));
        assert!(almost_eq(frame.forward, [0.0, 0.0, 1.0], 1e-5));
    }

    #[test]
    fn test_rotated_scaled_invariance() {
        let source_bones = tpose_bones();
        let mapping = tpose_mapping();
        let source_frame = derive_character_frame(&mapping, &source_bones).unwrap();

        let rx = std::f32::consts::FRAC_PI_2;
        let rz = std::f32::consts::PI;
        let scale = 2.0;

        let transformed_bones: Vec<BoneInput> = source_bones
            .iter()
            .map(|b| BoneInput {
                name: b.name.clone(),
                parent: b.parent,
                rest_position: rotate_and_scale(b.rest_position, rx, rz, scale),
            })
            .collect();

        let transformed_frame = derive_character_frame(&mapping, &transformed_bones).unwrap();

        let expected_right = rotate_and_scale([-1.0, 0.0, 0.0], rx, rz, 1.0);
        let expected_up = rotate_and_scale([0.0, 1.0, 0.0], rx, rz, 1.0);
        let expected_forward = rotate_and_scale([0.0, 0.0, 1.0], rx, rz, 1.0);

        assert!(almost_eq(transformed_frame.right, expected_right, 1e-5));
        assert!(almost_eq(transformed_frame.up, expected_up, 1e-5));
        assert!(almost_eq(transformed_frame.forward, expected_forward, 1e-5));

        let test_vector = [1.0, 2.0, 3.0];
        let source_char = source_frame.to_character(test_vector);
        let transformed_char =
            transformed_frame.to_character(rotate_and_scale(test_vector, rx, rz, 1.0));

        assert!(almost_eq(source_char, transformed_char, 1e-5));
    }

    #[test]
    fn test_missing_head_returns_none() {
        let bones = tpose_bones();
        let mut by_role = BTreeMap::new();
        by_role.insert(HumanoidRole::Hips, 0);
        by_role.insert(HumanoidRole::LeftUpperArm, 5);
        by_role.insert(HumanoidRole::RightUpperArm, 6);
        let mapping = HumanoidMapping { by_role };

        assert!(derive_character_frame(&mapping, &bones).is_none());
    }
}
