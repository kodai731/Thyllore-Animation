#![cfg(test)]

use std::collections::BTreeMap;

use thyllore_avatar_core::humanoid::components::role::HumanoidRole;
use thyllore_avatar_core::motion::components::sampled_pose::SampledPose;
use thyllore_avatar_core::muscle::components::unity_muscle_table::default_unity_muscle_table;
use thyllore_avatar_core::muscle::systems::unity_muscle_conversion::{
    from_unity_muscles, to_unity_muscles,
};

fn empty_pose() -> SampledPose {
    SampledPose {
        rotations: BTreeMap::new(),
        hips_translation: [0.0, 0.0, 0.0],
        morph: BTreeMap::new(),
    }
}

#[test]
fn table_has_95_muscles_with_roles() {
    let table = default_unity_muscle_table();
    assert_eq!(table.muscles.len(), 95);
    for (i, muscle) in table.muscles.iter().enumerate() {
        assert_eq!(muscle.index, i);
    }
}

#[test]
fn rest_pose_gives_rest_muscles() {
    let table = default_unity_muscle_table();
    let pose = empty_pose();
    let values = to_unity_muscles(&pose, table);
    for (i, muscle) in table.muscles.iter().enumerate() {
        assert!(
            (values[i] - muscle.rest).abs() < 1e-6,
            "muscle {} ({}) value {:.6} != rest {:.6}",
            i,
            muscle.name,
            values[i],
            muscle.rest
        );
    }
}

#[test]
fn left_upper_arm_z_follows_muscle() {
    let table = default_unity_muscle_table();
    let mut rotations = BTreeMap::new();
    rotations.insert(HumanoidRole::LeftUpperArm, [0.0, 0.0, 20.0]);
    let pose = SampledPose {
        rotations,
        hips_translation: [0.0, 0.0, 0.0],
        morph: BTreeMap::new(),
    };
    let values = to_unity_muscles(&pose, table);

    let muscle = table
        .muscles
        .iter()
        .find(|m| m.name == "Left Arm Down-Up")
        .expect("muscle 'Left Arm Down-Up' not found");

    let expected: f32 = muscle.rest - 20.0 / -muscle.min;
    assert_eq!(muscle.sign, -1.0);
    assert!(
        (values[muscle.index] - expected).abs() < 1e-5,
        "value {:.6} != expected {:.6}",
        values[muscle.index],
        expected
    );
}

#[test]
fn from_unity_muscles_inverts_primary_axes() {
    let table = default_unity_muscle_table();
    let mut rotations = BTreeMap::new();
    rotations.insert(HumanoidRole::LeftUpperArm, [0.0, 0.0, -15.0]);
    rotations.insert(HumanoidRole::RightLowerLeg, [10.0, 0.0, 0.0]);
    rotations.insert(HumanoidRole::Spine, [0.0, 8.0, 0.0]);
    rotations.insert(HumanoidRole::LeftIndexProximal, [0.0, 0.0, 12.0]);
    let pose = SampledPose {
        rotations: rotations.clone(),
        hips_translation: [0.0, 0.0, 0.0],
        morph: BTreeMap::new(),
    };

    let values = to_unity_muscles(&pose, table);
    let recovered = from_unity_muscles(&values, table);

    for (role, expected_angles) in &rotations {
        let recovered_angles = recovered
            .rotations
            .get(role)
            .copied()
            .unwrap_or([0.0, 0.0, 0.0]);
        for axis in 0..3 {
            let diff = (recovered_angles[axis] - expected_angles[axis]).abs();
            assert!(
                diff < 1e-6,
                "{:?} axis {}: recovered {:.6} != expected {:.6} (diff {:.6})",
                role,
                axis,
                recovered_angles[axis],
                expected_angles[axis],
                diff
            );
        }
    }
}

#[test]
fn finger_muscles_use_hand_attribute() {
    let table = default_unity_muscle_table();
    let muscle = &table.muscles[55];
    assert_eq!(muscle.name, "Left Thumb 1 Stretched");
    assert_eq!(muscle.attribute, "LeftHand.Thumb.1 Stretched");

    let index_muscle = table
        .muscles
        .iter()
        .find(|m| m.name == "Left Index 1 Stretched")
        .expect("Left Index 1 Stretched not found");
    assert_eq!(index_muscle.attribute, "LeftHand.Index.1 Stretched");

    let spread = table
        .muscles
        .iter()
        .find(|m| m.name == "Left Index Spread")
        .expect("Left Index Spread not found");
    assert_eq!(spread.attribute, "LeftHand.Index.Spread");

    let right_little = table
        .muscles
        .iter()
        .find(|m| m.name == "Right Little 3 Stretched")
        .expect("Right Little 3 Stretched not found");
    assert_eq!(right_little.attribute, "RightHand.Little.3 Stretched");
}
