use crate::expression::components::preset::{ExpressionPreset, PresetApplication};

const CAPTURE_EPSILON: f32 = 1e-6;

pub fn apply_preset(
    preset: &ExpressionPreset,
    channel_names: &[String],
    default_weights: &[f32],
) -> PresetApplication {
    let mut weights = Vec::with_capacity(channel_names.len());
    let mut unknown_channels = Vec::new();

    for (index, name) in channel_names.iter().enumerate() {
        let default_weight = default_weights.get(index).copied().unwrap_or(0.0);
        let weight = preset.weights.get(name).copied().unwrap_or(default_weight);
        weights.push(weight);
    }

    for channel_name in preset.weights.keys() {
        if !channel_names.contains(channel_name) {
            unknown_channels.push(channel_name.clone());
        }
    }

    PresetApplication {
        weights,
        unknown_channels,
    }
}

pub fn capture_preset(name: &str, channel_names: &[String], weights: &[f32]) -> ExpressionPreset {
    let mut preset = ExpressionPreset {
        name: name.to_string(),
        weights: std::collections::BTreeMap::new(),
    };

    for (i, weight) in weights.iter().enumerate() {
        if weight.abs() > CAPTURE_EPSILON {
            preset.weights.insert(channel_names[i].clone(), *weight);
        }
    }

    preset
}

#[cfg(test)]
mod tests {
    use super::*;

    fn channel_names() -> Vec<String> {
        vec![
            "eye_angry".to_string(),
            "mouth_smile".to_string(),
            "eyebrow_raise".to_string(),
        ]
    }

    #[test]
    fn test_apply_preset_full_match() {
        let names = channel_names();
        let preset = ExpressionPreset {
            name: "test".to_string(),
            weights: [
                ("eye_angry".to_string(), 1.0),
                ("mouth_smile".to_string(), 0.5),
                ("eyebrow_raise".to_string(), 0.0),
            ]
            .into_iter()
            .collect(),
        };

        let result = apply_preset(&preset, &names, &[]);

        assert_eq!(result.weights, vec![1.0, 0.5, 0.0]);
        assert!(result.unknown_channels.is_empty());
    }

    #[test]
    fn test_apply_preset_missing_channels_are_zero() {
        let names = channel_names();
        let preset = ExpressionPreset {
            name: "test".to_string(),
            weights: [("eye_angry".to_string(), 1.0)].into_iter().collect(),
        };

        let result = apply_preset(&preset, &names, &[]);

        assert_eq!(result.weights, vec![1.0, 0.0, 0.0]);
        assert!(result.unknown_channels.is_empty());
    }

    #[test]
    fn test_apply_preset_unknown_channels() {
        let names = channel_names();
        let preset = ExpressionPreset {
            name: "test".to_string(),
            weights: [
                ("eye_angry".to_string(), 1.0),
                ("unknown_channel".to_string(), 0.5),
            ]
            .into_iter()
            .collect(),
        };

        let result = apply_preset(&preset, &names, &[]);

        assert_eq!(result.weights, vec![1.0, 0.0, 0.0]);
        assert_eq!(result.unknown_channels, vec!["unknown_channel"]);
    }

    #[test]
    fn test_capture_preset() {
        let names = channel_names();
        let weights = [1.0, 0.5, 0.0];

        let preset = capture_preset("test", &names, &weights);

        assert_eq!(preset.name, "test");
        assert_eq!(preset.weights.len(), 2);
        assert_eq!(*preset.weights.get("eye_angry").unwrap(), 1.0);
        assert_eq!(*preset.weights.get("mouth_smile").unwrap(), 0.5);
        assert!(!preset.weights.contains_key("eyebrow_raise"));
    }

    #[test]
    fn test_capture_preset_epsilon_filtering() {
        let names = channel_names();
        let weights = [1e-7, 1e-5, 0.5];

        let preset = capture_preset("test", &names, &weights);

        assert_eq!(preset.weights.len(), 2);
        assert!(!preset.weights.contains_key("eye_angry"));
        assert!(preset.weights.contains_key("mouth_smile"));
    }

    #[test]
    fn test_capture_preset_keeps_negative_weights() {
        let names = channel_names();
        let weights = [-0.5, -1e-7, 0.0];

        let preset = capture_preset("test", &names, &weights);

        assert_eq!(preset.weights.len(), 1);
        assert_eq!(*preset.weights.get("eye_angry").unwrap(), -0.5);
    }

    #[test]
    fn test_round_trip() {
        let names = channel_names();
        let original_weights = [1.0, 0.5, 0.0];

        let preset = capture_preset("roundtrip", &names, &original_weights);
        let applied = apply_preset(&preset, &names, &[]);

        assert_eq!(applied.weights, vec![1.0, 0.5, 0.0]);
        assert!(applied.unknown_channels.is_empty());
    }

    #[test]
    fn test_round_trip_with_unknown() {
        let names = channel_names();
        let original_weights = [1.0, 0.5, 0.3];

        let preset = capture_preset("roundtrip", &names, &original_weights);

        let old_names = vec!["eye_angry".to_string(), "mouth_smile".to_string()];
        let applied = apply_preset(&preset, &old_names, &[]);

        assert_eq!(applied.weights, vec![1.0, 0.5]);
        assert_eq!(applied.unknown_channels, vec!["eyebrow_raise"]);
    }

    #[test]
    fn test_apply_preset_falls_back_to_default_weight() {
        let names = vec!["Toe_heels".to_string(), "eye_angry".to_string()];
        let preset = ExpressionPreset {
            name: "test".to_string(),
            weights: [("eye_angry".to_string(), 0.8)].into_iter().collect(),
        };

        let result = apply_preset(&preset, &names, &[1.0, 0.0]);

        assert_eq!(result.weights, vec![1.0, 0.8]);
    }
}
