mod support;

use support::fixture_bones::load_fixture_bones;

fn test_rest_pose(
    convention_id: &str,
    expected_pose: thyllore_avatar_core::humanoid::components::rest_pose::RestPose,
) {
    let bones = load_fixture_bones(convention_id);

    let (mapping, _) = thyllore_avatar_core::humanoid::systems::name_match::infer_mapping(&bones);

    let frame = thyllore_avatar_core::humanoid::systems::character_frame::derive_character_frame(
        &mapping, &bones,
    )
    .unwrap();

    let pose = thyllore_avatar_core::humanoid::systems::pose::detect_rest_pose(&mapping, &bones);
    assert_eq!(
        pose, expected_pose,
        "convention {}: expected {:?}, got {:?}",
        convention_id, expected_pose, pose
    );

    let hips_idx =
        mapping.by_role[&thyllore_avatar_core::humanoid::components::role::HumanoidRole::Hips];
    let head_idx =
        mapping.by_role[&thyllore_avatar_core::humanoid::components::role::HumanoidRole::Head];
    let right_hand_idx =
        mapping.by_role[&thyllore_avatar_core::humanoid::components::role::HumanoidRole::RightHand];
    let right_upper_arm_idx = mapping.by_role
        [&thyllore_avatar_core::humanoid::components::role::HumanoidRole::RightUpperArm];

    let hips_pos = bones[hips_idx].rest_position;
    let head_pos = bones[head_idx].rest_position;

    let head_hips_char = frame.to_character([
        head_pos[0] - hips_pos[0],
        head_pos[1] - hips_pos[1],
        head_pos[2] - hips_pos[2],
    ]);

    assert!(
        head_hips_char[1] > 0.0,
        "convention {}: Head - Hips character y = {} (expected positive)",
        convention_id,
        head_hips_char[1]
    );
    assert!(
        head_hips_char[0].abs() < 1e-3,
        "convention {}: Head - Hips character x = {} (expected ~0)",
        convention_id,
        head_hips_char[0]
    );
    assert!(
        head_hips_char[2].abs() < 1e-3,
        "convention {}: Head - Hips character z = {} (expected ~0)",
        convention_id,
        head_hips_char[2]
    );

    let right_hand_char = frame.to_character(bones[right_hand_idx].rest_position);
    let right_upper_arm_char = frame.to_character(bones[right_upper_arm_idx].rest_position);

    assert!(
        right_hand_char[0] > right_upper_arm_char[0],
        "convention {}: RightHand character x = {} not greater than RightUpperArm character x = {}",
        convention_id,
        right_hand_char[0],
        right_upper_arm_char[0]
    );
}

#[test]
fn test_blender() {
    test_rest_pose(
        "blender",
        thyllore_avatar_core::humanoid::components::rest_pose::RestPose::TPose,
    );
}

#[test]
fn test_maya() {
    test_rest_pose(
        "maya",
        thyllore_avatar_core::humanoid::components::rest_pose::RestPose::TPose,
    );
}

#[test]
fn test_max_biped() {
    test_rest_pose(
        "max_biped",
        thyllore_avatar_core::humanoid::components::rest_pose::RestPose::TPose,
    );
}

#[test]
fn test_mixamo() {
    test_rest_pose(
        "mixamo",
        thyllore_avatar_core::humanoid::components::rest_pose::RestPose::TPose,
    );
}

#[test]
fn test_unreal() {
    test_rest_pose(
        "unreal",
        thyllore_avatar_core::humanoid::components::rest_pose::RestPose::TPose,
    );
}

#[test]
fn test_vrm_normalized() {
    test_rest_pose(
        "vrm_normalized",
        thyllore_avatar_core::humanoid::components::rest_pose::RestPose::TPose,
    );
}

#[test]
fn test_blender_apose() {
    test_rest_pose(
        "blender_apose",
        thyllore_avatar_core::humanoid::components::rest_pose::RestPose::APose,
    );
}

#[test]
fn test_adversarial() {
    test_rest_pose(
        "adversarial",
        thyllore_avatar_core::humanoid::components::rest_pose::RestPose::TPose,
    );
}
