use crate::lightning::{overwrite_lightning_persisted_fields, LightningEffect, LightningSource};

pub const LIGHTNING_PRESET_NAMES: &[&str] = &["bolt", "arc", "charge", "beam"];

pub fn apply_lightning_preset(effect: &mut LightningEffect, name: &str) -> bool {
    let mut preset = LightningEffect::default();
    match name {
        "bolt" => {
            preset.end_offset = [0.0, -6.0, 0.0];
            preset.source = LightningSource::Point;
            preset.branch_depth = 2;
            preset.stroke_count = 3;
            preset.reseed_period = 1.0 / 30.0;
            preset.burst_count = 1;
        }
        "arc" => {
            preset.end_offset = [6.0, 0.0, 0.0];
            preset.branch_depth = 1;
            preset.branch_probability = 0.1;
            preset.attack_time = 0.17;
            preset.release_time = 0.04;
        }
        "charge" => {
            preset.source = LightningSource::Shell { radius: 4.0 };
            preset.strikes_per_burst = 30;
            preset.detail_levels = 2;
            preset.charge_ramp = 1.2;
        }
        "beam" => {
            preset.beam_radius = 0.25;
            preset.beam_arc_count = 6;
            preset.attack_time = 0.03;
            preset.release_time = 0.2;
        }
        _ => return false,
    }

    preset.position = effect.position;
    preset.rotation = effect.rotation;
    overwrite_lightning_persisted_fields(effect, &preset);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgmath::{Quaternion, Vector3};

    #[test]
    fn test_all_presets_apply_and_are_idempotent() {
        for name in LIGHTNING_PRESET_NAMES {
            let mut once = LightningEffect::default();
            assert!(apply_lightning_preset(&mut once, name), "{name}");
            let mut twice = LightningEffect::default();
            apply_lightning_preset(&mut twice, name);
            apply_lightning_preset(&mut twice, name);
            assert_eq!(once, twice, "{name}");
        }
    }

    #[test]
    fn test_preset_preserves_runtime_state() {
        let mut effect = LightningEffect::default();
        effect.position = Vector3::new(1.0, 2.0, 3.0);
        effect.rotation = Quaternion::new(0.99, 0.1, 0.0, 0.0);
        effect.time = 5.0;

        apply_lightning_preset(&mut effect, "bolt");

        assert_eq!(effect.position, Vector3::new(1.0, 2.0, 3.0));
        assert_eq!(effect.rotation, Quaternion::new(0.99, 0.1, 0.0, 0.0));
        assert_eq!(effect.time, 5.0);
        assert_eq!(effect.end_offset, [0.0, -6.0, 0.0]);
    }

    #[test]
    fn test_bolt_preset_values() {
        let mut effect = LightningEffect::default();
        assert!(apply_lightning_preset(&mut effect, "bolt"));

        assert_eq!(effect.end_offset, [0.0, -6.0, 0.0]);
        assert_eq!(effect.source, LightningSource::Point);
        assert_eq!(effect.branch_depth, 2);
        assert_eq!(effect.stroke_count, 3);
        assert!((effect.reseed_period - 1.0 / 30.0).abs() < 1e-6);
        assert_eq!(effect.burst_count, 1);
    }

    #[test]
    fn test_arc_preset_values() {
        let mut effect = LightningEffect::default();
        assert!(apply_lightning_preset(&mut effect, "arc"));

        assert_eq!(effect.end_offset, [6.0, 0.0, 0.0]);
        assert_eq!(effect.branch_depth, 1);
        assert!((effect.branch_probability - 0.1).abs() < 1e-6);
        assert!((effect.attack_time - 0.17).abs() < 1e-6);
        assert!((effect.release_time - 0.04).abs() < 1e-6);
    }

    #[test]
    fn test_charge_preset_values() {
        let mut effect = LightningEffect::default();
        assert!(apply_lightning_preset(&mut effect, "charge"));

        match effect.source {
            LightningSource::Shell { radius } => {
                assert!((radius - 4.0).abs() < 1e-6);
            }
            _ => panic!("expected Shell source"),
        }
        assert_eq!(effect.strikes_per_burst, 30);
        assert_eq!(effect.detail_levels, 2);
        assert!((effect.charge_ramp - 1.2).abs() < 1e-6);
    }

    #[test]
    fn test_beam_preset_values() {
        let mut effect = LightningEffect::default();
        assert!(apply_lightning_preset(&mut effect, "beam"));

        assert!((effect.beam_radius - 0.25).abs() < 1e-6);
        assert_eq!(effect.beam_arc_count, 6);
        assert!((effect.attack_time - 0.03).abs() < 1e-6);
        assert!((effect.release_time - 0.2).abs() < 1e-6);
    }

    #[test]
    fn test_unknown_preset_leaves_effect_untouched() {
        let mut effect = LightningEffect::default();
        effect.end_offset = [9.0, 9.0, 9.0];
        assert!(!apply_lightning_preset(&mut effect, "unknown"));
        assert_eq!(effect.end_offset, [9.0, 9.0, 9.0]);
    }
}
