use std::path::PathBuf;

use anyhow::Result;
use clap::builder::{PossibleValuesParser, RangedU64ValueParser};
use clap::Args;
use thyllore_effect_core::{
    apply_flame_preset, find_scalar_param, refresh_flame_coefficients, FlameBaked, FlameColor,
    FlameDebugView, FlameTrailState, StyleGroups, TextureFitGroups, FLAME_PRESET_NAMES,
    FLAME_SCALAR_PARAMS,
};

use crate::asset::AssetStorage;
use crate::ecs::component::{FlameBoneAttachment, FlameEffect, FlameTrail, HeatPlume, MotionPath};
use crate::ecs::resource::{
    BatchFlameOrbit, BatchRun, FlameBatchCapture, FlameDumpSink, FlameRenderSettings,
    FlameSdfSource, FlameShadingMode,
};
use crate::ecs::systems::cli_args::{
    finite_float_parse, float_pair_parse, scalar_assignment_parse,
};
use crate::ecs::world::{Entity, World};
use crate::hooks::bootstrap::BootstrapOverrides;

const ROT_Z_DEG_KEY: &str = "rot_z_deg";

#[derive(Args, Debug)]
pub struct FlameOverrides {
    #[arg(long = "batch-flame-mode")]
    pub mode: Option<FlameShadingMode>,
    #[arg(long = "batch-flame-debug-view")]
    pub debug_view: Option<FlameDebugView>,
    #[arg(long = "batch-flame-steps", value_parser = clap::value_parser!(u32).range(1..))]
    pub steps: Option<u32>,
    #[arg(long = "flame-dump")]
    pub dump_path: Option<String>,
    #[arg(long = "batch-flame-count", value_parser = RangedU64ValueParser::<usize>::new().range(1..=4))]
    pub count: Option<usize>,
    #[arg(long = "batch-flame-preset", value_parser = PossibleValuesParser::new(FLAME_PRESET_NAMES))]
    pub preset: Option<String>,
    #[arg(long = "batch-flame-set", value_parser = flame_set_entry_parse)]
    pub set: Vec<(String, f32)>,
    #[arg(long = "batch-flame-trail", value_parser = trail_parse)]
    pub trail: Option<f32>,
    #[arg(long = "batch-flame-orbit", value_parser = orbit_parse)]
    pub orbit: Option<(f32, f32)>,
    #[arg(long = "batch-flame-motion", value_parser = motion_parse)]
    pub motion: Option<(f32, f32)>,
    #[arg(long = "batch-flame-sdf")]
    pub sdf: Option<String>,
    #[arg(long = "batch-flame-bone")]
    pub bone: Option<String>,
    #[arg(long = "batch-flame-texture", value_parser = texture_fit_parse)]
    pub texture_fit: Option<(String, f32, bool)>,
    #[arg(long = "batch-flame-style", value_parser = style_parse)]
    pub style: Option<(String, StyleGroups)>,
    #[arg(long = "batch-flame-style-dump")]
    pub style_dump: Option<String>,
    #[arg(long = "batch-flame-trace")]
    pub trace_path: Option<PathBuf>,
    #[arg(long = "batch-wall-probe")]
    pub wall_probe_path: Option<PathBuf>,
    #[arg(
        long = "batch-heat-plume",
        num_args = 0..=1,
        default_missing_value = "10.0,0.5",
        value_parser = heat_plume_parse
    )]
    pub heat_plume: Option<(f32, f32)>,
}

impl BootstrapOverrides for FlameOverrides {
    const NAME: &'static str = "flame";

    fn apply(&self, world: &mut World, assets: &mut AssetStorage) -> Result<()> {
        self.apply_render_settings(world);
        if let Some(path) = &self.dump_path {
            world.insert_resource(FlameDumpSink::new(std::path::PathBuf::from(path)));
        }
        if let Some(path) = &self.sdf {
            world.insert_resource(FlameSdfSource { path: path.clone() });
        }
        if world.contains_resource::<BatchRun>() {
            let mut request = super::flame_batch_capture_mut(world);
            request.trace_path = self.trace_path.clone();
            request.wall_probe_path = self.wall_probe_path.clone();
        }

        self.spawn_extra_flames(world, assets);
        for entity in world.query_flames() {
            self.apply_to_flame(world, entity);
        }
        self.attach_flame_companions(world);
        Ok(())
    }
}

crate::bootstrap_hook!(FlameOverrides);

impl FlameOverrides {
    fn apply_render_settings(&self, world: &mut World) {
        if self.mode.is_none() && self.steps.is_none() && self.debug_view.is_none() {
            return;
        }
        let mut settings = world.resource_mut::<FlameRenderSettings>();
        if let Some(mode) = self.mode {
            settings.shading_mode = mode;
        }
        if let Some(step_count) = self.steps {
            settings.reference_step_count = step_count;
            settings.noise_step_count = step_count;
        }
        if let Some(debug_view) = self.debug_view {
            settings.debug_view = debug_view;
        }
    }

    fn spawn_extra_flames(&self, world: &mut World, assets: &mut AssetStorage) {
        let Some(count) = self.count else {
            return;
        };
        for i in 1..count {
            let effect = FlameEffect {
                position: cgmath::Vector3::new(1.5 * i as f32, 0.0, 0.0),
                radius: 0.7,
                height: 0.8,
                color: FlameColor {
                    temperature_base_k: 1900.0 - 250.0 * i as f32,
                    temperature_tip_k: 1100.0 - 150.0 * i as f32,
                    ..FlameColor::default()
                },
                ..FlameEffect::default()
            };
            let entity =
                super::spawn_flame_with_clip(world, assets, &format!("Flame {}", i + 1), effect);
            world.insert_component(entity, FlameBaked::default());
        }
    }

    fn apply_to_flame(&self, world: &mut World, entity: Entity) {
        let Some(mut effect) = world.get_component::<FlameEffect>(entity).cloned() else {
            return;
        };
        let mut baked = world
            .get_component::<FlameBaked>(entity)
            .cloned()
            .unwrap_or_default();

        if let Some(name) = self.preset.as_deref() {
            apply_flame_preset(&mut effect, name);
        }
        if let Some((path, blend, profile)) = &self.texture_fit {
            super::apply_texture_fit_from_path(
                &mut effect,
                &mut baked,
                path,
                *blend,
                TextureFitGroups::default(),
                *profile,
                "cli",
            );
        }
        if let Some((path, groups)) = &self.style {
            super::apply_flame_style_from_path(&mut effect, path, *groups);
        }
        apply_flame_overrides(&mut effect, &self.set);
        refresh_flame_coefficients(&mut effect, &baked);
        if let Some(path) = &self.style_dump {
            super::dump_flame_style_to_path(&effect, path);
        }

        world.insert_component(entity, effect);
        world.insert_component(entity, baked);
    }

    fn attach_flame_companions(&self, world: &mut World) {
        let entities = world.query_flames();

        if let Some(fade) = self.trail {
            for &entity in &entities {
                world.insert_component(
                    entity,
                    FlameTrail {
                        state: FlameTrailState {
                            enabled: true,
                            fade_seconds: fade,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                );
            }
        }
        if let Some((gain, amp)) = self.heat_plume {
            for &entity in &entities {
                world.insert_component(
                    entity,
                    HeatPlume {
                        distortion_gain: gain,
                        turbulence_amp: amp,
                        ..Default::default()
                    },
                );
            }
        }
        if let Some(bone) = &self.bone {
            for &entity in &entities {
                world.insert_component(entity, FlameBoneAttachment { bone: bone.clone() });
            }
        }

        if let Some((radius, period)) = self.orbit {
            world.insert_resource(BatchFlameOrbit {
                radius,
                period_seconds: period,
                initial: None,
            });
        }
        if let Some((radius, angular_speed)) = self.motion {
            if let Some(&first) = entities.first() {
                let center = world
                    .get_component::<FlameEffect>(first)
                    .map(|e| e.position)
                    .unwrap_or(cgmath::Vector3::new(0.0, 0.0, 0.0));
                world.insert_component(
                    first,
                    MotionPath {
                        center,
                        radius,
                        angular_speed,
                        phase_offset: 0.0,
                        enabled: true,
                    },
                );
            }
        }
    }
}

pub(crate) fn flame_set_valid_keys() -> Vec<&'static str> {
    FLAME_SCALAR_PARAMS
        .iter()
        .map(|param| param.name)
        .chain([ROT_Z_DEG_KEY])
        .collect()
}

pub fn apply_flame_overrides(effect: &mut FlameEffect, overrides: &[(String, f32)]) {
    for (key, value) in overrides {
        if key == ROT_Z_DEG_KEY {
            effect.rotation = cgmath::Quaternion::from(cgmath::Euler::new(
                cgmath::Deg(0.0),
                cgmath::Deg(0.0),
                cgmath::Deg(*value),
            ));
            continue;
        }

        let param = find_scalar_param(FLAME_SCALAR_PARAMS, key)
            .unwrap_or_else(|| unreachable!("unknown key (parser should have rejected)"));
        (param.set)(effect, *value);
    }
}

fn flame_set_entry_parse(text: &str) -> Result<(String, f32), String> {
    scalar_assignment_parse(text, &flame_set_valid_keys())
}

fn trail_parse(text: &str) -> Result<f32, String> {
    let fade = finite_float_parse(text)?;
    if fade <= 0.0 {
        return Err(format!("fade_seconds must be > 0, got '{text}'"));
    }
    Ok(fade)
}

fn orbit_parse(text: &str) -> Result<(f32, f32), String> {
    let (radius, period) = float_pair_parse(text)?;
    if radius < 0.0 || period <= 0.0 {
        return Err(format!(
            "expected <radius>,<period_seconds> with radius >= 0 and period > 0, got '{text}'"
        ));
    }
    Ok((radius, period))
}

fn motion_parse(text: &str) -> Result<(f32, f32), String> {
    let (radius, angular_speed) = float_pair_parse(text)?;
    if radius < 0.0 {
        return Err(format!(
            "expected <radius>,<angular_speed> with radius >= 0, got '{text}'"
        ));
    }
    Ok((radius, angular_speed))
}

fn heat_plume_parse(text: &str) -> Result<(f32, f32), String> {
    let mut parts = text.split(',');
    let gain = finite_float_parse(parts.next().unwrap_or_default())?;
    let amp = match parts.next() {
        None => 0.5,
        Some(amp) => finite_float_parse(amp)?,
    };
    if parts.next().is_some() {
        return Err(format!("expected <gain>[,<amp>], got '{text}'"));
    }
    if gain < 0.0 || amp < 0.0 {
        return Err(format!("gain and amp must be >= 0, got '{text}'"));
    }
    Ok((gain, amp))
}

fn texture_fit_parse(text: &str) -> Result<(String, f32, bool), String> {
    let parts: Vec<&str> = text.split(',').map(str::trim).collect();
    let path = parts[0].to_string();
    if path.is_empty() {
        return Err("expected <path>[,<blend>[,<profile>]] with a non-empty path".to_string());
    }

    let blend = match parts.get(1) {
        None => 1.0,
        Some(blend) => finite_float_parse(blend)?,
    };
    if !(0.0..=1.0).contains(&blend) {
        return Err(format!("blend must be in [0, 1], got '{text}'"));
    }

    let profile = match parts.get(2) {
        None | Some(&"statistics") => false,
        Some(&"profile") => true,
        Some(other) => {
            return Err(format!(
                "profile must be 'profile' or 'statistics', got '{other}'"
            ))
        }
    };
    Ok((path, blend, profile))
}

fn style_parse(text: &str) -> Result<(String, StyleGroups), String> {
    let mut parts = text.split(',').map(str::trim);
    let path = parts.next().unwrap_or_default().to_string();
    if path.is_empty() {
        return Err(
            "expected <path>[,motion][,texture][,optics] with a non-empty path".to_string(),
        );
    }

    let group_names: Vec<&str> = parts.collect();
    if group_names.is_empty() {
        return Ok((path, StyleGroups::default()));
    }
    let mut groups = StyleGroups {
        motion: false,
        texture: false,
        optics: false,
    };
    for name in group_names {
        match name {
            "motion" => groups.motion = true,
            "texture" => groups.texture = true,
            "optics" => groups.optics = true,
            other => {
                return Err(format!(
                    "unknown style group '{other}'. Valid groups: motion, texture, optics"
                ))
            }
        }
    }
    Ok((path, groups))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn resolve_mode_and_steps() {
        let overrides = FlameOverrides::resolve(&args(&[
            "bin",
            "--batch-flame-mode",
            "raymarch",
            "--batch-flame-steps",
            "512",
        ]))
        .unwrap();
        assert_eq!(overrides.mode, Some(FlameShadingMode::ReferenceRaymarch));
        assert_eq!(overrides.steps, Some(512));
    }

    #[test]
    fn resolve_rejects_invalid_mode_and_steps() {
        assert!(FlameOverrides::resolve(&args(&["bin", "--batch-flame-mode", "x"])).is_err());
        assert!(FlameOverrides::resolve(&args(&["bin", "--batch-flame-steps", "0"])).is_err());
        assert!(FlameOverrides::resolve(&args(&["bin", "--batch-flame-steps", "abc"])).is_err());
    }

    #[test]
    fn resolve_rejects_count_outside_range_and_unknown_preset() {
        assert!(FlameOverrides::resolve(&args(&["bin", "--batch-flame-count", "5"])).is_err());
        assert!(FlameOverrides::resolve(&args(&["bin", "--batch-flame-count", "0"])).is_err());
        assert!(FlameOverrides::resolve(&args(&["bin", "--batch-flame-preset", "lava"])).is_err());
    }

    #[test]
    fn style_path_only_defaults_to_all_groups() {
        let (path, groups) = style_parse("assets/flames/styles/pillar.style.ron").unwrap();
        assert_eq!(path, "assets/flames/styles/pillar.style.ron");
        assert_eq!(groups, StyleGroups::default());
    }

    #[test]
    fn style_group_subset() {
        let (_, groups) = style_parse("s.ron,motion,optics").unwrap();
        assert!(groups.motion && groups.optics && !groups.texture);
    }

    #[test]
    fn style_unknown_group_error() {
        assert!(style_parse("s.ron,shape").is_err());
    }

    #[test]
    fn set_combined_form() {
        let pairs = FlameOverrides::resolve(&args(&["--batch-flame-set=noise_amplitude=0.35"]))
            .unwrap()
            .set;
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "noise_amplitude");
        assert!((pairs[0].1 - 0.35).abs() < 1e-6);
    }

    #[test]
    fn set_separate_form() {
        let pairs = FlameOverrides::resolve(&args(&["--batch-flame-set", "noise_amplitude=0.35"]))
            .unwrap()
            .set;
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "noise_amplitude");
        assert!((pairs[0].1 - 0.35).abs() < 1e-6);
    }

    #[test]
    fn set_unknown_key_error() {
        let err =
            FlameOverrides::resolve(&args(&["--batch-flame-set", "invalid_key=1.0"])).unwrap_err();
        assert!(err.to_string().contains("invalid_key"));
    }

    #[test]
    fn apply_overrides_no_panic_for_all_keys() {
        for key in flame_set_valid_keys() {
            let mut effect = FlameEffect::default();
            apply_flame_overrides(&mut effect, &[(key.to_string(), 1.0)]);
        }
    }

    /// Every key the pre-registry FLAME_SET_KEYS table accepted must keep working.
    #[test]
    fn set_legacy_keys_stay_accepted() {
        let legacy_keys = [
            "warp_amp",
            "warp_freq",
            "rise_speed",
            "taper_power",
            "radius_tip_ratio",
            "edge_low",
            "edge_high",
            "white_boost",
            "bend_amount",
            "bend_power",
            "wind_x",
            "wind_z",
            "noise_amplitude",
            "noise_contrast",
            "noise_frequency",
            "noise_scroll_speed",
            "sigma_t",
            "intensity",
            "height",
            "radius",
            "time",
            "time_scale",
            "time_offset",
            "rot_z_deg",
            "temperature_base_k",
            "temperature_tip_k",
            "envelope_peak",
            "envelope_base",
            "envelope_tail",
            "radial_sharpness",
            "emitter_kind",
            "ring_major_radius",
            "ring_angular_speed",
            "noise_aniso_y",
            "warp_y_scale",
            "occlusion_lum_ref",
            "contour_wiggle_amp",
            "aniso_axis_advect",
            "rte_bands",
            "sigma_dispersion",
            "boundary_amp",
            "near_fade_radius",
            "carve_residual",
            "tip_carve_depth",
            "tip_carve_reach",
            "warp_reach",
            "swirl_gain",
            "swirl_speed",
            "spread_gain",
            "support_margin",
            "meander_amp",
            "meander_frequency",
            "mix_lo",
            "mix_hi",
            "mix_height_gain",
            "mix_scale",
            "mix_radial_gain",
            "density_exp",
            "temp_exp",
            "wien_c_k",
            "wave_segments",
            "boundary_freq",
            "boundary_speed",
            "boundary_radius_ratio",
            "edge_outer_sharpen",
            "noise_scale_mode",
            "erosion_noise_gain",
            "twist_gain",
            "twist_speed",
            "burnout_gain",
            "noise_shaping_scale",
            "optical_depth",
            "branch_period",
            "branch_life",
            "branch_gain",
            "branch_core_radius",
            "branch_core_offset",
            "branch_reach",
            "branch_spread",
            "branch_spawn_height",
            "branch_spawn_range",
            "branch_seed",
        ];
        let valid = flame_set_valid_keys();
        for key in legacy_keys {
            assert!(valid.contains(&key), "legacy key {key} no longer accepted");
        }
    }

    #[test]
    fn preset_resolve_valid() {
        let result = FlameOverrides::resolve(&args(&["--batch-flame-preset", "candle"])).unwrap();
        assert_eq!(result.preset, Some(String::from("candle")));
    }

    #[test]
    fn preset_then_override_order() {
        let mut effect = FlameEffect::default();
        apply_flame_preset(&mut effect, "candle");

        apply_flame_overrides(&mut effect, &[(String::from("height"), 1.5)]);

        assert!(
            (effect.height - 1.5).abs() < 1e-5,
            "height should be overridden to 1.5, got {}",
            effect.height
        );
        assert!(
            (effect.radius - 0.07).abs() < 1e-5,
            "radius should still be candle's 0.07, got {}",
            effect.radius
        );
    }

    #[test]
    fn texture_fit_path_only_defaults_blend_to_one() {
        let resolved = texture_fit_parse("image.png").unwrap();
        assert_eq!(resolved.0, "image.png");
        assert!((resolved.1 - 1.0).abs() < 1e-6);
        assert!(!resolved.2);
    }

    #[test]
    fn texture_fit_path_with_blend() {
        let resolved = texture_fit_parse("image.png,0.4").unwrap();
        assert_eq!(resolved.0, "image.png");
        assert!((resolved.1 - 0.4).abs() < 1e-6);
        assert!(!resolved.2);
    }

    #[test]
    fn texture_fit_invalid_blend_is_err() {
        assert!(texture_fit_parse("image.png,abc").is_err());
    }

    #[test]
    fn texture_fit_profile() {
        let resolved = texture_fit_parse("image.png,0.5,profile").unwrap();
        assert_eq!(resolved.0, "image.png");
        assert!((resolved.1 - 0.5).abs() < 1e-6);
        assert!(resolved.2);
    }

    fn heat_plume_of(list: &[&str]) -> Result<Option<(f32, f32)>> {
        Ok(FlameOverrides::resolve(&args(list))?.heat_plume)
    }

    #[test]
    fn heat_plume_defaults_and_pairs() {
        assert_eq!(
            heat_plume_of(&["bin", "--batch-heat-plume"]).unwrap(),
            Some((10.0, 0.5))
        );
        assert_eq!(
            heat_plume_of(&["bin", "--batch-heat-plume", "--batch-flame-steps", "8"]).unwrap(),
            Some((10.0, 0.5))
        );
        assert_eq!(
            heat_plume_of(&["bin", "--batch-heat-plume", "4"]).unwrap(),
            Some((4.0, 0.5))
        );
        assert_eq!(
            heat_plume_of(&["bin", "--batch-heat-plume", "4,0.2"]).unwrap(),
            Some((4.0, 0.2))
        );
        assert!(heat_plume_of(&["bin", "--batch-heat-plume", "1,2,3"]).is_err());
        assert!(heat_plume_of(&["bin", "--batch-heat-plume", "-1"]).is_err());
    }

    #[test]
    fn orbit_and_motion_pairs_are_validated() {
        let resolved = FlameOverrides::resolve(&args(&[
            "bin",
            "--batch-flame-orbit",
            "2,4",
            "--batch-flame-motion",
            "1,-0.5",
        ]))
        .unwrap();
        assert_eq!(resolved.orbit, Some((2.0, 4.0)));
        assert_eq!(resolved.motion, Some((1.0, -0.5)));
        assert!(FlameOverrides::resolve(&args(&["bin", "--batch-flame-orbit", "2,0"])).is_err());
        assert!(FlameOverrides::resolve(&args(&["bin", "--batch-flame-orbit", "2"])).is_err());
        assert!(FlameOverrides::resolve(&args(&["bin", "--batch-flame-motion", "-1,2"])).is_err());
        assert!(FlameOverrides::resolve(&args(&["bin", "--batch-flame-trail", "0"])).is_err());
    }

    #[test]
    fn batch_dump_flags_fill_the_capture_request() {
        let overrides = FlameOverrides::resolve(&args(&[
            "bin",
            "--batch-flame-trace",
            "/tmp/trace.json",
            "--batch-wall-probe",
            "/tmp/wall.json",
        ]))
        .unwrap();
        let mut world = World::new();
        world.insert_resource(BatchRun::new(
            crate::ecs::resource::CaptureSchedule::single(PathBuf::from("/tmp/out.png"), 1),
        ));
        overrides
            .apply(&mut world, &mut AssetStorage::new())
            .unwrap();

        let request = world.resource::<FlameBatchCapture>();
        assert_eq!(request.trace_path, Some(PathBuf::from("/tmp/trace.json")));
        assert_eq!(
            request.wall_probe_path,
            Some(PathBuf::from("/tmp/wall.json"))
        );
        assert!(!request.wall_probe);
    }

    #[test]
    fn apply_without_flags_leaves_an_empty_world_untouched() {
        let overrides = FlameOverrides::resolve(&args(&["bin"])).unwrap();
        let mut world = World::new();
        let mut assets = AssetStorage::new();
        overrides.apply(&mut world, &mut assets).unwrap();
        assert!(world.get_resource::<FlameSdfSource>().is_none());
        assert!(world.get_resource::<BatchFlameOrbit>().is_none());
    }
}
