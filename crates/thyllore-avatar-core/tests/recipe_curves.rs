use std::fs;

use thyllore_avatar_core::humanoid::components::role::HumanoidRole;
use thyllore_avatar_core::motion::systems::recipe_curves::{
    build_recipe_curves, sample_recipe_curve, sample_recipe_pose,
};
use thyllore_avatar_core::motion::systems::recipe_io::parse_recipe;

fn load_wave() -> thyllore_avatar_core::motion::components::pose_recipe::PoseRecipe {
    let json = fs::read_to_string("../../assets/recipies/wave.json").unwrap();
    parse_recipe(&json).unwrap()
}

#[test]
fn test_right_lower_arm_z_key_times() {
    let recipe = load_wave();
    let curves = build_recipe_curves(&recipe);
    let keys = &curves.rotations[&HumanoidRole::RightLowerArm][2];
    let times: Vec<f32> = keys.iter().map(|k| k.time).collect();
    assert_eq!(times.len(), 8);
    let expected = [0.0, 0.4, 0.8, 1.2, 1.6, 2.0, 2.4, 2.8];
    for (i, (got, exp)) in times.iter().zip(expected.iter()).enumerate() {
        assert!(
            (got - exp).abs() < 1e-4,
            "time[{}] = {:.4}, expected {:.4}",
            i,
            got,
            exp
        );
    }
}

#[test]
fn test_right_lower_arm_z_values() {
    let recipe = load_wave();
    let curves = build_recipe_curves(&recipe);
    let keys = &curves.rotations[&HumanoidRole::RightLowerArm][2];
    let values: Vec<f32> = keys.iter().map(|k| k.value).collect();
    let expected = [90.0, 110.0, 70.0, 110.0, 70.0, 110.0, 70.0, 0.0];
    assert_eq!(values.len(), expected.len());
    for (i, (got, exp)) in values.iter().zip(expected.iter()).enumerate() {
        assert!(
            (got - exp).abs() < 1e-4,
            "value[{}] = {:.4}, expected {:.4}",
            i,
            got,
            exp
        );
    }
}

#[test]
fn test_interpolation_at_0_2() {
    let recipe = load_wave();
    let curves = build_recipe_curves(&recipe);
    let keys = &curves.rotations[&HumanoidRole::RightLowerArm][2];
    let value = sample_recipe_curve(keys, 0.2).unwrap();
    assert!(
        (value - 100.0).abs() < 1e-4,
        "expected 100.0 at time 0.2, got {:.4}",
        value
    );
}

#[test]
fn test_extrapolation_before() {
    let recipe = load_wave();
    let curves = build_recipe_curves(&recipe);
    let keys = &curves.rotations[&HumanoidRole::RightLowerArm][2];
    let value = sample_recipe_curve(keys, -1.0).unwrap();
    assert!(
        (value - 90.0).abs() < 1e-4,
        "expected 90.0 at time -1.0, got {:.4}",
        value
    );
}

#[test]
fn test_extrapolation_after() {
    let recipe = load_wave();
    let curves = build_recipe_curves(&recipe);
    let keys = &curves.rotations[&HumanoidRole::RightLowerArm][2];
    let value = sample_recipe_curve(keys, 5.0).unwrap();
    assert!(
        (value - 0.0).abs() < 1e-4,
        "expected 0.0 at time 5.0, got {:.4}",
        value
    );
}

#[test]
fn test_head_key_count() {
    let recipe = load_wave();
    let curves = build_recipe_curves(&recipe);
    let head_keys = &curves.rotations[&HumanoidRole::Head][0];
    assert_eq!(head_keys.len(), 2, "Head should have 2 keys");
}

#[test]
fn test_morph_preset_smile_keys() {
    let recipe = load_wave();
    let curves = build_recipe_curves(&recipe);
    let smile_keys = &curves.morph["preset:smile"];
    assert_eq!(smile_keys.len(), 2, "preset:smile should have 2 keys");
    assert!(
        (smile_keys[0].time - 0.0).abs() < 1e-4,
        "first key time should be 0.0"
    );
    assert!(
        (smile_keys[0].value - 0.6).abs() < 1e-4,
        "first key value should be 0.6"
    );
    assert!(
        (smile_keys[1].time - 2.8).abs() < 1e-4,
        "second key time should be 2.8"
    );
    assert!(
        (smile_keys[1].value - 0.0).abs() < 1e-4,
        "second key value should be 0.0"
    );
}

#[test]
fn test_sample_recipe_pose_at_0_4() {
    let recipe = load_wave();
    let curves = build_recipe_curves(&recipe);
    let pose = sample_recipe_pose(&curves, 0.4);
    assert!(
        (pose.rotations[&HumanoidRole::RightHand][2] - 20.0).abs() < 1e-4,
        "RightHand z should be 20 at time 0.4"
    );
    let upper_arm_keys = &curves.rotations[&HumanoidRole::RightUpperArm][2];
    assert!(
        !upper_arm_keys.is_empty(),
        "RightUpperArm curve should have keys (0.0 key retained)"
    );
    assert!(
        (upper_arm_keys[0].time - 0.0).abs() < 1e-4
            && (upper_arm_keys[0].value - 20.0).abs() < 1e-4,
        "RightUpperArm first key should be at time 0.0 with value 20"
    );
    let head_y = pose.rotations[&HumanoidRole::Head][1];
    assert!(
        head_y >= 0.0 && head_y <= 10.0,
        "Head y should be between 0 and 10 at time 0.4, got {:.4}",
        head_y
    );
}
