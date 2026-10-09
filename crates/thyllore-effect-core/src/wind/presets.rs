use crate::wind::{overwrite_wind_persisted_fields, WindTornadoEffect};

pub const WIND_PRESET_NAMES: &[&str] = &["column", "funnel", "reference", "storm"];
/// The preset the shipped default scene starts from; hosts adding a wind start here too.
pub const WIND_DEFAULT_PRESET: &str = "storm";

pub fn apply_wind_preset(effect: &mut WindTornadoEffect, name: &str) -> bool {
    let mut preset = WindTornadoEffect::default();
    match name {
        "column" => {}
        "reference" => apply_reference_values(&mut preset),
        "storm" => {
            apply_reference_values(&mut preset);
            preset.eddy_amplitude = 1.0;
            preset.eddy_shear = 1.0;
            preset.eddy_erosion = 0.45;
        }
        "funnel" => {
            preset.column_height = 3.0;
            preset.wall_radius_base = 0.25;
            preset.wall_radius_top = 1.2;
            preset.wall_width_q = 0.05;
            preset.top_fade = 0.4;
            preset.rise_initial_height = 0.2;
            preset.rise_duration = 1.5;
            preset.spread_start = 1.5;
            preset.spread_rate = 0.05;
            preset.dissipate_start = 4.0;
            preset.dissipate_time = 2.0;
        }
        _ => return false,
    }

    preset.position = effect.position;
    preset.rotation = effect.rotation;
    overwrite_wind_persisted_fields(effect, &preset);
    true
}

fn apply_reference_values(preset: &mut WindTornadoEffect) {
    preset.column_height = 2.0;
    preset.wall_radius_base = 0.35;
    preset.wall_radius_top = 0.6;
    preset.wall_width_q = 0.08;
    preset.wall_strength = 1.0;
    preset.top_fade = 0.3;
    preset.density = 4.0;
    preset.albedo = [0.9, 0.93, 1.0];
    preset.ambient_brightness = 1.0;
    preset.rise_initial_height = 0.3;
    preset.rise_duration = 1.0;
    preset.spread_start = 0.25;
    preset.spread_rate = 0.15;
    preset.dissipate_start = 2.0;
    preset.dissipate_time = 1.5;
    preset.phase_g = 0.6;
    preset.sun_intensity = 1.0;
    preset.circulation = 2.0;
    preset.streak_order = 3.0;
    preset.streak_twist = 4.0;
    preset.streak_rise_speed = 1.0;
    preset.streak_amplitude = 0.25;
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgmath::{Quaternion, Vector3};

    #[test]
    fn test_all_presets_apply_and_are_idempotent() {
        for name in WIND_PRESET_NAMES {
            let mut once = WindTornadoEffect::default();
            assert!(apply_wind_preset(&mut once, name), "{name}");
            let mut twice = WindTornadoEffect::default();
            apply_wind_preset(&mut twice, name);
            apply_wind_preset(&mut twice, name);
            assert_eq!(once, twice, "{name}");
        }
    }

    #[test]
    fn test_preset_preserves_runtime_state() {
        let mut effect = WindTornadoEffect::default();
        effect.position = Vector3::new(1.0, 2.0, 3.0);
        effect.rotation = Quaternion::new(0.99, 0.1, 0.0, 0.0);
        effect.time = 5.0;

        apply_wind_preset(&mut effect, "funnel");

        assert_eq!(effect.position, Vector3::new(1.0, 2.0, 3.0));
        assert_eq!(effect.rotation, Quaternion::new(0.99, 0.1, 0.0, 0.0));
        assert_eq!(effect.time, 5.0);
        assert_eq!(effect.wall_radius_top, 1.2);
    }

    #[test]
    fn test_reference_preset_applies_measured_values() {
        let mut effect = WindTornadoEffect::default();
        assert!(apply_wind_preset(&mut effect, "reference"));

        assert_eq!(effect.column_height, 2.0);
        assert_eq!(effect.wall_radius_base, 0.35);
        assert_eq!(effect.wall_radius_top, 0.6);
        assert_eq!(effect.rise_initial_height, 0.3);
        assert_eq!(effect.rise_duration, 1.0);
        assert_eq!(effect.spread_start, 0.25);
        assert_eq!(effect.spread_rate, 0.15);
        assert_eq!(effect.dissipate_start, 2.0);
        assert_eq!(effect.dissipate_time, 1.5);
        assert_eq!(effect.circulation, 2.0);
        assert_eq!(effect.streak_amplitude, 0.25);
    }

    #[test]
    fn test_storm_preset_adds_eroded_eddies_to_reference() {
        let mut reference = WindTornadoEffect::default();
        apply_wind_preset(&mut reference, "reference");
        let mut storm = WindTornadoEffect::default();
        assert!(apply_wind_preset(&mut storm, "storm"));

        assert_eq!(storm.eddy_amplitude, 1.0);
        assert_eq!(storm.eddy_shear, 1.0);
        assert_eq!(storm.eddy_erosion, 0.45);

        storm.eddy_amplitude = reference.eddy_amplitude;
        storm.eddy_shear = reference.eddy_shear;
        storm.eddy_erosion = reference.eddy_erosion;
        assert_eq!(storm, reference);
    }

    #[test]
    fn test_unknown_preset_leaves_effect_untouched() {
        let mut effect = WindTornadoEffect::default();
        effect.column_height = 7.0;
        assert!(!apply_wind_preset(&mut effect, "unknown"));
        assert_eq!(effect.column_height, 7.0);
    }
}
