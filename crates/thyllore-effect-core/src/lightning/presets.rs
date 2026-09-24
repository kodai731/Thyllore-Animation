use crate::lightning::{overwrite_lightning_persisted_fields, LightningEffect, LightningSource};

pub const LIGHTNING_PRESET_NAMES: &[&str] = &["bolt", "arc", "charge", "beam"];

pub fn apply_lightning_preset(effect: &mut LightningEffect, name: &str) -> bool {
    let mut preset = LightningEffect::default();
    match name {
        "bolt" => {
            preset.end_offset = [0.0, -8.5, 0.0];
            preset.source = LightningSource::Point;
            preset.core_radius = 0.02;
            preset.tip_radius_ratio = 0.5;
            preset.core_intensity = 100.0;
            preset.detail_levels = 7;
            preset.tortuosity = 0.45;
            preset.roughness = 0.6;
            preset.branch_depth = 2;
            preset.branch_zone_start = 0.15;
            preset.branch_count = 16.0;
            preset.branch_probability = 0.55;
            preset.branch_angle = 0.9;
            preset.branch_length_ratio = 0.45;
            preset.branch_intensity_ratio = 0.7;
            preset.burst_start = 0.08;
            preset.burst_count = 1;
            preset.attack_time = 0.12;
            preset.sustain_time = 0.35;
            preset.release_time = 0.04;
            preset.stroke_count = 3;
            preset.stroke_interval = 0.08;
            preset.flicker_amplitude = 0.6;
            preset.reseed_period = 0.07;
            preset.reseed_level = 4;
            preset.flash_gain = 0.00014;
        }
        "arc" => {
            preset.end_offset = [6.0, 0.0, 0.0];
            preset.branch_depth = 1;
            preset.branch_count = 3.0;
            preset.branch_probability = 0.1;
            preset.attack_time = 0.17;
            preset.release_time = 0.04;
            preset.flash_gain = 0.0002;
        }
        "charge" => {
            preset.source = LightningSource::Shell { radius: 4.0 };
            preset.strikes_per_burst = 30;
            preset.branch_count = 1.0;
            preset.detail_levels = 2;
            preset.charge_ramp = 1.2;
            preset.flash_gain = 0.00012;
        }
        "beam" => {
            preset.beam_radius = 0.25;
            preset.beam_arc_count = 6;
            preset.attack_time = 0.03;
            preset.release_time = 0.2;
            preset.flash_gain = 0.00003;
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
        assert_eq!(effect.end_offset, [0.0, -8.5, 0.0]);
    }

    #[test]
    fn test_bolt_preset_values() {
        let mut effect = LightningEffect::default();
        assert!(apply_lightning_preset(&mut effect, "bolt"));

        assert_eq!(effect.end_offset, [0.0, -8.5, 0.0]);
        assert_eq!(effect.source, LightningSource::Point);
        assert!((effect.core_radius - 0.02).abs() < 1e-6);
        assert!((effect.tip_radius_ratio - 0.5).abs() < 1e-6);
        assert!((effect.rim_ratio - 2.0).abs() < 1e-6);
        assert!((effect.rim_intensity - 10.0).abs() < 1e-6);
        assert!((effect.core_intensity - 100.0).abs() < 1e-6);
        assert_eq!(effect.detail_levels, 7);
        assert!((effect.tortuosity - 0.45).abs() < 1e-6);
        assert!((effect.roughness - 0.6).abs() < 1e-6);
        assert_eq!(effect.branch_depth, 2);
        assert!((effect.branch_zone_start - 0.15).abs() < 1e-6);
        assert!((effect.branch_count - 16.0).abs() < 1e-6);
        assert!((effect.branch_probability - 0.55).abs() < 1e-6);
        assert!((effect.branch_angle - 0.9).abs() < 1e-6);
        assert!((effect.branch_length_ratio - 0.45).abs() < 1e-6);
        assert!((effect.branch_intensity_ratio - 0.7).abs() < 1e-6);
        assert!((effect.burst_start - 0.08).abs() < 1e-6);
        assert_eq!(effect.burst_count, 1);
        assert!((effect.attack_time - 0.12).abs() < 1e-6);
        assert!((effect.sustain_time - 0.35).abs() < 1e-6);
        assert!((effect.release_time - 0.04).abs() < 1e-6);
        assert_eq!(effect.stroke_count, 3);
        assert!((effect.stroke_interval - 0.08).abs() < 1e-6);
        assert!((effect.flicker_amplitude - 0.6).abs() < 1e-6);
        assert!((effect.reseed_period - 0.07).abs() < 1e-6);
        assert_eq!(effect.reseed_level, 4);
        assert!((effect.flash_gain - 0.00014).abs() < 1e-9);
    }

    #[test]
    fn test_arc_preset_values() {
        let mut effect = LightningEffect::default();
        assert!(apply_lightning_preset(&mut effect, "arc"));

        assert_eq!(effect.end_offset, [6.0, 0.0, 0.0]);
        assert_eq!(effect.branch_depth, 1);
        assert!((effect.branch_count - 3.0).abs() < 1e-6);
        assert!((effect.branch_probability - 0.1).abs() < 1e-6);
        assert!((effect.attack_time - 0.17).abs() < 1e-6);
        assert!((effect.release_time - 0.04).abs() < 1e-6);
        assert!((effect.flash_gain - 0.0002).abs() < 1e-9);
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
        assert!((effect.branch_count - 1.0).abs() < 1e-6);
        assert_eq!(effect.detail_levels, 2);
        assert!((effect.charge_ramp - 1.2).abs() < 1e-6);
        assert!((effect.flash_gain - 0.00012).abs() < 1e-9);
    }

    #[test]
    fn test_beam_preset_values() {
        let mut effect = LightningEffect::default();
        assert!(apply_lightning_preset(&mut effect, "beam"));

        assert!((effect.beam_radius - 0.25).abs() < 1e-6);
        assert_eq!(effect.beam_arc_count, 6);
        assert!((effect.attack_time - 0.03).abs() < 1e-6);
        assert!((effect.release_time - 0.2).abs() < 1e-6);
        assert!((effect.flash_gain - 0.00003).abs() < 1e-9);
    }

    #[test]
    fn test_unknown_preset_leaves_effect_untouched() {
        let mut effect = LightningEffect::default();
        effect.end_offset = [9.0, 9.0, 9.0];
        assert!(!apply_lightning_preset(&mut effect, "unknown"));
        assert_eq!(effect.end_offset, [9.0, 9.0, 9.0]);
    }
}
