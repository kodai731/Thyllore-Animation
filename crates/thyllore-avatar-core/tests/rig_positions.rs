mod support;

use cgmath::Matrix;

use support::canonical_bones;
use support::rig_convention::rig_convention;
use support::rig_names::CONVENTIONS;
use support::rig_positions::character_to_file_rotation;
use support::rig_positions::file_positions;

fn inverse_transform(
    file_pos: [f64; 3],
    convention: &support::rig_convention::RigConvention,
) -> [f64; 3] {
    let mut p = cgmath::Vector3::new(file_pos[0], file_pos[1], file_pos[2]);
    let inv_rot = character_to_file_rotation(convention.character_to_file).transpose();
    p = inv_rot * p;
    let meter_scale = convention.unit_scale_factor / 100.0 / convention.body_scale;
    p *= meter_scale;
    [-p.x, p.y, p.z]
}

#[test]
fn inverse_roundtrip_all_conventions() {
    for id in CONVENTIONS {
        let convention = rig_convention(id);
        let positions = file_positions(&convention);
        let bones = canonical_bones();
        assert_eq!(positions.len(), bones.len());
        let arm_bone_indices: [usize; 4] = if id == "blender_apose" {
            [7, 8, 11, 12]
        } else {
            [usize::MAX; 4]
        };
        for (i, bone) in bones.iter().enumerate() {
            let is_arm = arm_bone_indices.contains(&i);
            if is_arm {
                continue;
            }
            let recovered = inverse_transform(positions[i], &convention);
            let expected: [f64; 3] = bone.position.map(|v| v as f64);
            for d in 0..3 {
                assert!(
                    (recovered[d] - expected[d]).abs() < 1e-7,
                    "{}: bone {} dim {} recovered={:.10} expected={:.10}",
                    id,
                    bone.role,
                    d,
                    recovered[d],
                    expected[d]
                );
            }
        }
    }
}

#[test]
fn blender_apose_right_hand_below_upper_arm() {
    let convention = rig_convention("blender_apose");
    let positions = file_positions(&convention);
    let bones = canonical_bones();
    let right_upper_arm_idx = bones
        .iter()
        .position(|b| b.role == "RightUpperArm")
        .unwrap();
    let right_hand_idx = bones.iter().position(|b| b.role == "RightHand").unwrap();
    let right_upper_arm_y = positions[right_upper_arm_idx][1];
    let right_hand_y = positions[right_hand_idx][1];
    assert!(
        right_hand_y < right_upper_arm_y,
        "RightHand y={:.6} should be below RightUpperArm y={:.6}",
        right_hand_y,
        right_upper_arm_y
    );
}

#[test]
fn max_biped_head_z_greater_than_hips() {
    let convention = rig_convention("max_biped");
    let positions = file_positions(&convention);
    let bones = canonical_bones();
    let hips_idx = bones.iter().position(|b| b.role == "Hips").unwrap();
    let head_idx = bones.iter().position(|b| b.role == "Head").unwrap();
    let hips_z = positions[hips_idx][2];
    let head_z = positions[head_idx][2];
    assert!(
        head_z > hips_z,
        "Head z={:.6} should be greater than Hips z={:.6}",
        head_z,
        hips_z
    );
}

#[test]
fn unreal_head_z_greater_than_hips() {
    let convention = rig_convention("unreal");
    let positions = file_positions(&convention);
    let bones = canonical_bones();
    let hips_idx = bones.iter().position(|b| b.role == "Hips").unwrap();
    let head_idx = bones.iter().position(|b| b.role == "Head").unwrap();
    assert!(
        positions[head_idx][2] > positions[hips_idx][2],
        "unreal: Head z={:.6} should be greater than Hips z={:.6}",
        positions[head_idx][2],
        positions[hips_idx][2]
    );
}

#[test]
fn unreal_right_hand_x_greater_than_hips() {
    let convention = rig_convention("unreal");
    let positions = file_positions(&convention);
    let bones = canonical_bones();
    let hips_idx = bones.iter().position(|b| b.role == "Hips").unwrap();
    let right_hand_idx = bones.iter().position(|b| b.role == "RightHand").unwrap();
    assert!(
        positions[right_hand_idx][0] > positions[hips_idx][0],
        "unreal: RightHand x={:.6} should be greater than Hips x={:.6}",
        positions[right_hand_idx][0],
        positions[hips_idx][0]
    );
}

#[test]
fn max_biped_right_hand_x_less_than_hips() {
    let convention = rig_convention("max_biped");
    let positions = file_positions(&convention);
    let bones = canonical_bones();
    let hips_idx = bones.iter().position(|b| b.role == "Hips").unwrap();
    let right_hand_idx = bones.iter().position(|b| b.role == "RightHand").unwrap();
    assert!(
        positions[right_hand_idx][0] < positions[hips_idx][0],
        "max_biped: RightHand x={:.6} should be less than Hips x={:.6}",
        positions[right_hand_idx][0],
        positions[hips_idx][0]
    );
}

#[test]
fn adversarial_head_x_less_than_hips() {
    let convention = rig_convention("adversarial");
    let positions = file_positions(&convention);
    let bones = canonical_bones();
    let hips_idx = bones.iter().position(|b| b.role == "Hips").unwrap();
    let head_idx = bones.iter().position(|b| b.role == "Head").unwrap();
    assert!(
        positions[head_idx][0] < positions[hips_idx][0],
        "adversarial: Head x={:.6} should be less than Hips x={:.6}",
        positions[head_idx][0],
        positions[hips_idx][0]
    );
}

#[test]
fn maya_head_y_greater_than_hips() {
    let convention = rig_convention("maya");
    let positions = file_positions(&convention);
    let bones = canonical_bones();
    let hips_idx = bones.iter().position(|b| b.role == "Hips").unwrap();
    let head_idx = bones.iter().position(|b| b.role == "Head").unwrap();
    assert!(
        positions[head_idx][1] > positions[hips_idx][1],
        "maya: Head y={:.6} should be greater than Hips y={:.6}",
        positions[head_idx][1],
        positions[hips_idx][1]
    );
}

#[test]
fn maya_right_hand_x_less_than_hips() {
    let convention = rig_convention("maya");
    let positions = file_positions(&convention);
    let bones = canonical_bones();
    let hips_idx = bones.iter().position(|b| b.role == "Hips").unwrap();
    let right_hand_idx = bones.iter().position(|b| b.role == "RightHand").unwrap();
    assert!(
        positions[right_hand_idx][0] < positions[hips_idx][0],
        "maya: RightHand x={:.6} should be less than Hips x={:.6}",
        positions[right_hand_idx][0],
        positions[hips_idx][0]
    );
}
