use crate::motion::components::pose_recipe::PoseRecipe;

const CYCLE_BOUNDARY_TOLERANCE: f32 = 1e-4;

pub fn parse_recipe(json: &str) -> anyhow::Result<PoseRecipe> {
    let recipe: PoseRecipe = serde_json::from_str(json)?;
    validate_recipe(&recipe)?;
    Ok(recipe)
}

fn validate_recipe(recipe: &PoseRecipe) -> anyhow::Result<()> {
    if recipe.version != 1 {
        anyhow::bail!("unsupported recipe version {}", recipe.version);
    }

    if recipe.fps == 0 {
        anyhow::bail!("fps must not be 0");
    }

    for (i, pose) in recipe.poses.iter().enumerate() {
        if i > 0 && pose.time <= recipe.poses[i - 1].time {
            anyhow::bail!(
                "poses are not in ascending time order: pose {} at {:.3}s <= pose {} at {:.3}s",
                i,
                pose.time,
                i - 1,
                recipe.poses[i - 1].time
            );
        }
    }

    if let Some(last) = recipe.poses.last() {
        if last.time > recipe.duration_seconds {
            anyhow::bail!(
                "last pose time {:.3}s exceeds duration {:.3}s",
                last.time,
                recipe.duration_seconds
            );
        }
    }

    if let Some(cycle) = &recipe.cycle {
        if cycle.end <= cycle.start {
            anyhow::bail!(
                "cycle end ({:.3}s) must be greater than start ({:.3}s)",
                cycle.end,
                cycle.start
            );
        }

        if cycle.count == 0 {
            anyhow::bail!("cycle count must not be 0");
        }

        let interval = cycle.end - cycle.start;
        let cycle_end = cycle.end + (cycle.count as f32) * interval;
        for pose in &recipe.poses {
            if pose.time >= cycle.end && pose.time < cycle_end - CYCLE_BOUNDARY_TOLERANCE {
                anyhow::bail!(
                    "pose at {:.3}s falls within cycle interval [{:.3}s, {:.3}s)",
                    pose.time,
                    cycle.end,
                    cycle_end
                );
            }
        }
    }

    for extend in &recipe.copilot.extend {
        if extend.horizon_frames > 64 {
            anyhow::bail!(
                "copilot extend horizon_frames {} exceeds maximum 64",
                extend.horizon_frames
            );
        }
    }

    for pose in &recipe.poses {
        for (name, weight) in &pose.morph {
            if !name.starts_with("vrc.")
                && !name.starts_with("preset:")
                && !name.starts_with("raw:")
            {
                anyhow::bail!(
                    "morph name {:?} must start with vrc., preset:, or raw:",
                    name
                );
            }

            if *weight < 0.0 || *weight > 1.0 {
                anyhow::bail!(
                    "morph weight {:.3} for {:?} is outside [0, 1]",
                    weight,
                    name
                );
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_json() -> String {
        serde_json::json!({
            "version": 1,
            "name": "test",
            "duration_seconds": 2.0,
            "fps": 30,
            "poses": [
                {"time": 0.0},
                {"time": 1.0}
            ]
        })
        .to_string()
    }

    #[test]
    fn test_unknown_role_error() {
        let json = serde_json::json!({
            "version": 1,
            "name": "test",
            "duration_seconds": 2.0,
            "fps": 30,
            "poses": [
                {
                    "time": 0.0,
                    "rotations": {"UnknownBone": [0.0, 0.0, 0.0]}
                }
            ]
        })
        .to_string();
        let err = parse_recipe(&json).unwrap_err();
        assert!(err.to_string().contains("UnknownBone"));
    }

    #[test]
    fn test_unknown_field_error() {
        let json = serde_json::json!({
            "version": 1,
            "name": "test",
            "duration_seconds": 2.0,
            "fps": 30,
            "poses": [
                {"time": 0.0}
            ],
            "unknown_field": true
        })
        .to_string();
        let err = parse_recipe(&json).unwrap_err();
        assert!(err.to_string().contains("unknown field"));
    }

    #[test]
    fn test_cycle_interval_has_pose_error() {
        let json = serde_json::json!({
            "version": 1,
            "name": "test",
            "duration_seconds": 3.0,
            "fps": 30,
            "poses": [
                {"time": 0.0},
                {"time": 1.5}
            ],
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
            "name": "test",
            "duration_seconds": 2.0,
            "fps": 30,
            "poses": [
                {"time": 0.0, "morph": {"smile": 0.5}}
            ]
        })
        .to_string();
        let err = parse_recipe(&json).unwrap_err();
        assert!(err.to_string().contains("must start with"));
    }

    #[test]
    fn test_version_2_error() {
        let json = serde_json::json!({
            "version": 2,
            "name": "test",
            "duration_seconds": 2.0,
            "fps": 30,
            "poses": [
                {"time": 0.0}
            ]
        })
        .to_string();
        let err = parse_recipe(&json).unwrap_err();
        assert!(err.to_string().contains("version 2"));
    }

    #[test]
    fn test_valid_minimal() {
        let json = base_json();
        let recipe = parse_recipe(&json).unwrap();
        assert_eq!(recipe.version, 1);
        assert_eq!(recipe.poses.len(), 2);
    }

    #[test]
    fn test_cycle_boundary_exactly_at_end_accepted() {
        let json = serde_json::json!({
            "version": 1,
            "name": "test",
            "duration_seconds": 2.8,
            "fps": 30,
            "poses": [
                {"time": 0.0},
                {"time": 2.8}
            ],
            "cycle": {"start": 0.4, "end": 1.2, "count": 2}
        })
        .to_string();
        let recipe = parse_recipe(&json).unwrap();
        assert_eq!(recipe.poses.len(), 2);
    }

    #[test]
    fn test_cycle_boundary_within_interval_rejected() {
        let json = serde_json::json!({
            "version": 1,
            "name": "test",
            "duration_seconds": 2.8,
            "fps": 30,
            "poses": [
                {"time": 0.0},
                {"time": 2.7}
            ],
            "cycle": {"start": 0.4, "end": 1.2, "count": 2}
        })
        .to_string();
        let err = parse_recipe(&json).unwrap_err();
        assert!(err.to_string().contains("falls within cycle interval"));
    }
}
