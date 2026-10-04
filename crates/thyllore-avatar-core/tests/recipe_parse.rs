use std::path::Path;

use thyllore_avatar_core::motion::systems::recipe_io::parse_recipe;

fn recipe_path(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/recipies")
        .join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("failed to read {}: {}", p.display(), e))
}

#[test]
fn test_wave_json_parses() {
    let json = recipe_path("wave.json");
    let recipe = parse_recipe(&json).unwrap();
    assert_eq!(recipe.name, "wave_right_hand");
    assert_eq!(recipe.version, 1);
    assert!((recipe.duration_seconds - 2.8).abs() < 1e-6);
    assert_eq!(recipe.fps, 30);
    assert!(!recipe.is_loop);
    assert_eq!(recipe.poses.len(), 4);
    assert!(recipe.cycle.is_some());
    let cycle = recipe.cycle.as_ref().unwrap();
    assert!((cycle.start - 0.4).abs() < 1e-6);
    assert!((cycle.end - 1.2).abs() < 1e-6);
    assert_eq!(cycle.count, 2);
    assert!(!recipe.hand_presets.is_empty());
}

#[test]
fn test_unknown_role_error() {
    let json = serde_json::json!({
        "version": 1,
        "name": "bad",
        "duration_seconds": 1.0,
        "fps": 30,
        "poses": [{"time": 0.0, "rotations": {"FakeBone": [0.0, 0.0, 0.0]}}]
    })
    .to_string();
    let err = parse_recipe(&json).unwrap_err();
    assert!(err.to_string().contains("FakeBone"));
}

#[test]
fn test_unknown_field_error() {
    let json = serde_json::json!({
        "version": 1,
        "name": "bad",
        "duration_seconds": 1.0,
        "fps": 30,
        "poses": [{"time": 0.0}],
        "no_such_field": 1
    })
    .to_string();
    let err = parse_recipe(&json).unwrap_err();
    assert!(err.to_string().contains("unknown field"));
}

#[test]
fn test_cycle_interval_pose_error() {
    let json = serde_json::json!({
        "version": 1,
        "name": "bad",
        "duration_seconds": 3.0,
        "fps": 30,
        "poses": [{"time": 0.0}, {"time": 1.5}],
        "cycle": {"start": 0.0, "end": 1.0, "count": 2}
    })
    .to_string();
    let err = parse_recipe(&json).unwrap_err();
    assert!(err.to_string().contains("falls within cycle interval"));
}

#[test]
fn test_morph_missing_prefix_error() {
    let json = serde_json::json!({
        "version": 1,
        "name": "bad",
        "duration_seconds": 1.0,
        "fps": 30,
        "poses": [{"time": 0.0, "morph": {"smile": 0.5}}]
    })
    .to_string();
    let err = parse_recipe(&json).unwrap_err();
    assert!(err.to_string().contains("must start with"));
}

#[test]
fn test_version_2_error() {
    let json = serde_json::json!({
        "version": 2,
        "name": "bad",
        "duration_seconds": 1.0,
        "fps": 30,
        "poses": [{"time": 0.0}]
    })
    .to_string();
    let err = parse_recipe(&json).unwrap_err();
    assert!(err.to_string().contains("version 2"));
}
