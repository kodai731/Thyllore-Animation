use crate::lightning::{overwrite_lightning_persisted_fields, LightningEffect, LightningSource};

pub const LIGHTNING_PRESET_NAMES: &[&str] = &["bolt", "arc", "charge", "beam"];

pub fn apply_lightning_preset(effect: &mut LightningEffect, name: &str) -> bool {
    let mut preset = LightningEffect::default();
    match name {
        "bolt" => {
            preset.shape.end_offset = [0.0, -8.5, 0.0];
            preset.shape.source = LightningSource::Point;
            preset.shape.core_radius = 0.02;
            preset.shape.tip_radius_ratio = 0.5;
            preset.look.core_intensity = 100.0;
            preset.shape.detail_levels = 7;
            preset.shape.tortuosity = 0.45;
            preset.shape.roughness = 0.6;
            preset.branch.depth = 2;
            preset.branch.zone_start = 0.15;
            preset.branch.count = 8.0;
            preset.branch.probability = 0.55;
            preset.branch.angle = 0.9;
            preset.branch.length_ratio = 0.25;
            preset.branch.intensity_ratio = 0.7;
            preset.timing.burst_start = 0.08;
            preset.timing.burst_count = 1;
            preset.timing.attack_time = 0.12;
            preset.timing.sustain_time = 0.35;
            preset.timing.release_time = 0.04;
            preset.timing.stroke_count = 3;
            preset.timing.stroke_interval = 0.08;
            preset.timing.flicker_amplitude = 0.6;
            preset.timing.reseed_period = 0.07;
            preset.timing.reseed_level = 4;
            preset.look.flash_gain = 0.00014;
        }
        "arc" => {
            preset.shape.end_offset = [6.0, 0.0, 0.0];
            preset.branch.depth = 1;
            preset.branch.count = 3.0;
            preset.branch.probability = 0.1;
            preset.timing.attack_time = 0.17;
            preset.timing.release_time = 0.04;
            preset.look.flash_gain = 0.0002;
        }
        "charge" => {
            preset.shape.source = LightningSource::Shell { radius: 4.0 };
            preset.shape.strikes_per_burst = 30;
            preset.branch.count = 1.0;
            preset.shape.detail_levels = 2;
            preset.timing.charge_ramp = 1.2;
            preset.look.flash_gain = 0.00012;
        }
        "beam" => {
            preset.look.beam_radius = 0.25;
            preset.look.beam_arc_count = 6;
            preset.timing.attack_time = 0.03;
            preset.timing.release_time = 0.2;
            preset.look.flash_gain = 0.00003;
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
        assert_eq!(effect.shape.end_offset, [0.0, -8.5, 0.0]);
    }

    #[test]
    fn test_bolt_preset_values() {
        let mut effect = LightningEffect::default();
        assert!(apply_lightning_preset(&mut effect, "bolt"));

        assert_eq!(effect.shape.end_offset, [0.0, -8.5, 0.0]);
        assert_eq!(effect.shape.source, LightningSource::Point);
        assert!((effect.shape.core_radius - 0.02).abs() < 1e-6);
        assert!((effect.shape.tip_radius_ratio - 0.5).abs() < 1e-6);
        assert!((effect.look.rim_ratio - 2.0).abs() < 1e-6);
        assert!((effect.look.rim_intensity - 10.0).abs() < 1e-6);
        assert!((effect.look.core_intensity - 100.0).abs() < 1e-6);
        assert_eq!(effect.shape.detail_levels, 7);
        assert!((effect.shape.tortuosity - 0.45).abs() < 1e-6);
        assert!((effect.shape.roughness - 0.6).abs() < 1e-6);
        assert_eq!(effect.branch.depth, 2);
        assert!((effect.branch.zone_start - 0.15).abs() < 1e-6);
        assert!((effect.branch.count - 8.0).abs() < 1e-6);
        assert!((effect.branch.probability - 0.55).abs() < 1e-6);
        assert!((effect.branch.angle - 0.9).abs() < 1e-6);
        assert!((effect.branch.length_ratio - 0.25).abs() < 1e-6);
        assert!((effect.branch.intensity_ratio - 0.7).abs() < 1e-6);
        assert!((effect.timing.burst_start - 0.08).abs() < 1e-6);
        assert_eq!(effect.timing.burst_count, 1);
        assert!((effect.timing.attack_time - 0.12).abs() < 1e-6);
        assert!((effect.timing.sustain_time - 0.35).abs() < 1e-6);
        assert!((effect.timing.release_time - 0.04).abs() < 1e-6);
        assert_eq!(effect.timing.stroke_count, 3);
        assert!((effect.timing.stroke_interval - 0.08).abs() < 1e-6);
        assert!((effect.timing.flicker_amplitude - 0.6).abs() < 1e-6);
        assert!((effect.timing.reseed_period - 0.07).abs() < 1e-6);
        assert_eq!(effect.timing.reseed_level, 4);
        assert!((effect.look.flash_gain - 0.00014).abs() < 1e-9);
    }

    #[test]
    fn test_arc_preset_values() {
        let mut effect = LightningEffect::default();
        assert!(apply_lightning_preset(&mut effect, "arc"));

        assert_eq!(effect.shape.end_offset, [6.0, 0.0, 0.0]);
        assert_eq!(effect.branch.depth, 1);
        assert!((effect.branch.count - 3.0).abs() < 1e-6);
        assert!((effect.branch.probability - 0.1).abs() < 1e-6);
        assert!((effect.timing.attack_time - 0.17).abs() < 1e-6);
        assert!((effect.timing.release_time - 0.04).abs() < 1e-6);
        assert!((effect.look.flash_gain - 0.0002).abs() < 1e-9);
    }

    #[test]
    fn test_charge_preset_values() {
        let mut effect = LightningEffect::default();
        assert!(apply_lightning_preset(&mut effect, "charge"));

        match effect.shape.source {
            LightningSource::Shell { radius } => {
                assert!((radius - 4.0).abs() < 1e-6);
            }
            _ => panic!("expected Shell source"),
        }
        assert_eq!(effect.shape.strikes_per_burst, 30);
        assert!((effect.branch.count - 1.0).abs() < 1e-6);
        assert_eq!(effect.shape.detail_levels, 2);
        assert!((effect.timing.charge_ramp - 1.2).abs() < 1e-6);
        assert!((effect.look.flash_gain - 0.00012).abs() < 1e-9);
    }

    #[test]
    fn test_beam_preset_values() {
        let mut effect = LightningEffect::default();
        assert!(apply_lightning_preset(&mut effect, "beam"));

        assert!((effect.look.beam_radius - 0.25).abs() < 1e-6);
        assert_eq!(effect.look.beam_arc_count, 6);
        assert!((effect.timing.attack_time - 0.03).abs() < 1e-6);
        assert!((effect.timing.release_time - 0.2).abs() < 1e-6);
        assert!((effect.look.flash_gain - 0.00003).abs() < 1e-9);
    }

    #[test]
    fn test_unknown_preset_leaves_effect_untouched() {
        let mut effect = LightningEffect::default();
        effect.shape.end_offset = [9.0, 9.0, 9.0];
        assert!(!apply_lightning_preset(&mut effect, "unknown"));
        assert_eq!(effect.shape.end_offset, [9.0, 9.0, 9.0]);
    }
}
