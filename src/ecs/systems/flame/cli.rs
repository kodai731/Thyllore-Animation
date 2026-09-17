use anyhow::{bail, Result};
use thyllore_effect_core::{
    apply_flame_preset, find_scalar_param, refresh_flame_coefficients, FlameBaked, FlameColor,
    FlameDebugView, FlameTrailState, StyleGroups, TextureFitGroups, FLAME_PRESET_NAMES,
    FLAME_SCALAR_PARAMS,
};

use crate::asset::AssetStorage;
use crate::ecs::component::{FlameBoneAttachment, FlameEffect, FlameTrail, HeatPlume, MotionPath};
use crate::ecs::resource::{
    BatchFlameOrbit, BatchRun, FlameDumpSink, FlameRenderSettings, FlameSdfSource, FlameShadingMode,
};
use crate::ecs::systems::cli_args::{flag_value_resolve_from_args, scalar_set_resolve_from_args};
use crate::ecs::world::{Entity, World};
use crate::hooks::bootstrap::BootstrapOverrides;

const MODE_FLAG: &str = "--batch-flame-mode";
const DEBUG_VIEW_FLAG: &str = "--batch-flame-debug-view";
const STEPS_FLAG: &str = "--batch-flame-steps";
const DUMP_FLAG: &str = "--flame-dump";
const COUNT_FLAG: &str = "--batch-flame-count";
const TRAIL_FLAG: &str = "--batch-flame-trail";
const ORBIT_FLAG: &str = "--batch-flame-orbit";
const BONE_FLAG: &str = "--batch-flame-bone";
const PRESET_FLAG: &str = "--batch-flame-preset";
const MOTION_FLAG: &str = "--batch-flame-motion";
const SDF_FLAG: &str = "--batch-flame-sdf";
const SET_FLAG: &str = "--batch-flame-set";
const STYLE_FLAG: &str = "--batch-flame-style";
const STYLE_DUMP_FLAG: &str = "--batch-flame-style-dump";
const TEXTURE_FLAG: &str = "--batch-flame-texture";
const HEAT_PLUME_FLAG: &str = "--batch-heat-plume";
const ROT_Z_DEG_KEY: &str = "rot_z_deg";

#[derive(Debug)]
pub struct FlameOverrides {
    pub mode: Option<FlameShadingMode>,
    pub debug_view: Option<FlameDebugView>,
    pub steps: Option<u32>,
    pub dump_path: Option<String>,
    pub count: Option<usize>,
    pub preset: Option<String>,
    pub set: Vec<(String, f32)>,
    pub trail: Option<f32>,
    pub orbit: Option<(f32, f32)>,
    pub motion: Option<(f32, f32)>,
    pub sdf: Option<String>,
    pub bone: Option<String>,
    pub texture_fit: Option<(String, f32, bool)>,
    pub style: Option<(String, StyleGroups)>,
    pub style_dump: Option<String>,
    pub heat_plume: Option<(f32, f32)>,
}

impl BootstrapOverrides for FlameOverrides {
    const NAME: &'static str = "flame";

    fn resolve(args: &[String]) -> Result<Self> {
        Ok(Self {
            mode: mode_resolve_from_args(args)?,
            debug_view: debug_view_resolve_from_args(args)?,
            steps: steps_resolve_from_args(args)?,
            dump_path: flag_value_resolve_from_args(args, DUMP_FLAG)?,
            count: count_resolve_from_args(args)?,
            preset: preset_resolve_from_args(args)?,
            set: set_resolve_from_args(args)?,
            trail: trail_resolve_from_args(args)?,
            orbit: orbit_resolve_from_args(args)?,
            motion: motion_resolve_from_args(args)?,
            sdf: flag_value_resolve_from_args(args, SDF_FLAG)?,
            bone: flag_value_resolve_from_args(args, BONE_FLAG)?,
            texture_fit: texture_fit_resolve_from_args(args)?,
            style: style_resolve_from_args(args)?,
            style_dump: flag_value_resolve_from_args(args, STYLE_DUMP_FLAG)?,
            heat_plume: heat_plume_resolve_from_args(args)?,
        })
    }

    fn apply(&self, world: &mut World, assets: &mut AssetStorage) -> Result<()> {
        self.apply_render_settings(world);
        if let Some(path) = &self.dump_path {
            world.insert_resource(FlameDumpSink::new(std::path::PathBuf::from(path)));
        }
        if let Some(path) = &self.sdf {
            world.insert_resource(FlameSdfSource { path: path.clone() });
        }
        if let Some(mut batch_run) = world.get_resource_mut::<BatchRun>() {
            batch_run.flame_set = self.set.clone();
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
            crate::ecs::systems::apply_texture_fit_from_path(
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

fn mode_resolve_from_args(args: &[String]) -> Result<Option<FlameShadingMode>> {
    let Some(position) = args.iter().position(|arg| arg == MODE_FLAG) else {
        return Ok(None);
    };
    let Some(value) = args.get(position + 1) else {
        bail!("{MODE_FLAG} requires a value: analytic|raymarch|thickness|noise|depthclamp");
    };
    let mode = FlameShadingMode::parse(value).ok_or_else(|| {
        anyhow::anyhow!(
            "invalid flame mode '{value}': expected analytic|raymarch|thickness|noise|depthclamp"
        )
    })?;
    Ok(Some(mode))
}

fn debug_view_resolve_from_args(args: &[String]) -> Result<Option<FlameDebugView>> {
    let Some(position) = args.iter().position(|arg| arg == DEBUG_VIEW_FLAG) else {
        return Ok(None);
    };
    let Some(value) = args.get(position + 1) else {
        bail!("{DEBUG_VIEW_FLAG} requires a value: off|shaped|erosion|argument|density|sigma|emission|jitter|wcoord");
    };
    let view = FlameDebugView::parse(value).ok_or_else(|| {
        anyhow::anyhow!(
            "invalid flame debug view '{value}': expected off|shaped|erosion|argument|density|sigma|emission|jitter|wcoord|grid|strain|stretch"
        )
    })?;
    Ok(Some(view))
}

fn steps_resolve_from_args(args: &[String]) -> Result<Option<u32>> {
    let Some(position) = args.iter().position(|arg| arg == STEPS_FLAG) else {
        return Ok(None);
    };
    let Some(value) = args.get(position + 1) else {
        bail!("{STEPS_FLAG} requires a step count");
    };
    let steps: u32 = value
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid step count '{value}': expected integer"))?;
    if steps == 0 {
        bail!("{STEPS_FLAG} must be >= 1");
    }
    Ok(Some(steps))
}

fn count_resolve_from_args(args: &[String]) -> Result<Option<usize>> {
    let Some(position) = args.iter().position(|arg| arg == COUNT_FLAG) else {
        return Ok(None);
    };
    let Some(value) = args.get(position + 1) else {
        bail!("{COUNT_FLAG} requires a count");
    };
    let count: usize = value
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid flame count '{value}': expected integer"))?;
    if !(1..=4).contains(&count) {
        bail!("{COUNT_FLAG} must be in range 1..=4, got {}", count);
    }
    Ok(Some(count))
}

fn set_resolve_from_args(args: &[String]) -> Result<Vec<(String, f32)>> {
    scalar_set_resolve_from_args(args, SET_FLAG, &flame_set_valid_keys())
}

fn preset_resolve_from_args(args: &[String]) -> Result<Option<String>> {
    let Some(position) = args.iter().position(|arg| arg == PRESET_FLAG) else {
        return Ok(None);
    };
    let Some(value) = args.get(position + 1) else {
        bail!("{PRESET_FLAG} requires <name>");
    };
    if !FLAME_PRESET_NAMES.contains(&value.as_str()) {
        bail!(
            "unknown flame preset '{}'. Valid presets: {}",
            value,
            FLAME_PRESET_NAMES.join(", ")
        );
    }
    Ok(Some(value.clone()))
}

fn style_resolve_from_args(args: &[String]) -> Result<Option<(String, StyleGroups)>> {
    let Some(position) = args.iter().position(|arg| arg == STYLE_FLAG) else {
        return Ok(None);
    };
    let Some(value) = args.get(position + 1).filter(|v| !v.starts_with("--")) else {
        bail!("{STYLE_FLAG} requires <path>[,motion][,texture][,optics]");
    };

    let mut parts = value.split(',');
    let path = parts.next().unwrap_or_default().trim().to_string();
    if path.is_empty() {
        bail!("{STYLE_FLAG} requires a non-empty path");
    }

    let group_names: Vec<&str> = parts.map(str::trim).collect();
    if group_names.is_empty() {
        return Ok(Some((path, StyleGroups::default())));
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
            other => bail!(
                "unknown {STYLE_FLAG} group '{}'. Valid groups: motion, texture, optics",
                other
            ),
        }
    }
    Ok(Some((path, groups)))
}

fn texture_fit_resolve_from_args(args: &[String]) -> Result<Option<(String, f32, bool)>> {
    let Some(position) = args.iter().position(|arg| arg == TEXTURE_FLAG) else {
        return Ok(None);
    };
    let Some(value) = args.get(position + 1) else {
        bail!("{TEXTURE_FLAG} requires <path>[,<blend>[,<profile>]]");
    };
    let parts: Vec<&str> = value.split(',').collect();
    let path = parts[0].trim().to_string();
    let blend = if parts.len() > 1 {
        parts[1]
            .trim()
            .parse::<f32>()
            .map_err(|_| anyhow::anyhow!("invalid {TEXTURE_FLAG} blend value '{}'", parts[1]))?
    } else {
        1.0
    };
    if !blend.is_finite() || blend < 0.0 || blend > 1.0 {
        bail!("{TEXTURE_FLAG} blend must be in [0, 1] and finite: '{value}'");
    }
    let profile = match parts.get(2).map(|s| s.trim()) {
        None | Some("statistics") => false,
        Some("profile") => true,
        Some(other) => {
            bail!(
                "invalid {TEXTURE_FLAG} profile value '{}'; must be 'profile' or 'statistics'",
                other
            );
        }
    };
    Ok(Some((path, blend, profile)))
}

fn heat_plume_resolve_from_args(args: &[String]) -> Result<Option<(f32, f32)>> {
    let Some(position) = args.iter().position(|arg| arg == HEAT_PLUME_FLAG) else {
        return Ok(None);
    };
    let value = match args.get(position + 1) {
        None => "10.0,0.5",
        Some(value) if value.starts_with("--") => "10.0,0.5",
        Some(value) => value.as_str(),
    };
    let parts: Vec<&str> = value.split(',').collect();
    if parts.len() > 2 {
        bail!("{HEAT_PLUME_FLAG} expects <gain>[,<amp>] but got '{value}'");
    }

    let gain: f32 = parts[0]
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid {HEAT_PLUME_FLAG} value '{value}'"))?;
    if !gain.is_finite() || gain < 0.0 {
        bail!("{HEAT_PLUME_FLAG} gain must be >= 0 and finite: '{value}'");
    }
    let amp: f32 = match parts.get(1) {
        None => 0.5,
        Some(text) => text
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid {HEAT_PLUME_FLAG} value '{value}'"))?,
    };
    if !amp.is_finite() || amp < 0.0 {
        bail!("{HEAT_PLUME_FLAG} amp must be >= 0 and finite: '{value}'");
    }
    Ok(Some((gain, amp)))
}

fn trail_resolve_from_args(args: &[String]) -> Result<Option<f32>> {
    let Some(position) = args.iter().position(|arg| arg == TRAIL_FLAG) else {
        return Ok(None);
    };
    let Some(value) = args.get(position + 1) else {
        bail!("{TRAIL_FLAG} requires <fade_seconds>");
    };
    let fade: f32 = value
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid {TRAIL_FLAG} value '{value}'"))?;
    if !fade.is_finite() || fade <= 0.0 {
        bail!("{TRAIL_FLAG} fade_seconds must be > 0 and finite: '{value}'");
    }
    Ok(Some(fade))
}

fn pair_resolve_from_args(args: &[String], flag: &str, usage: &str) -> Result<Option<(f32, f32)>> {
    let Some(position) = args.iter().position(|arg| arg == flag) else {
        return Ok(None);
    };
    let Some(value) = args.get(position + 1) else {
        bail!("{flag} requires {usage}");
    };
    let parts: Vec<&str> = value.split(',').collect();
    if parts.len() != 2 {
        bail!("{flag} expects 2 comma-separated values, got '{value}'");
    }
    let first: f32 = parts[0]
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid {flag} value in '{value}'"))?;
    let second: f32 = parts[1]
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid {flag} value in '{value}'"))?;
    Ok(Some((first, second)))
}

fn orbit_resolve_from_args(args: &[String]) -> Result<Option<(f32, f32)>> {
    let Some((radius, period)) =
        pair_resolve_from_args(args, ORBIT_FLAG, "<radius>,<period_seconds>")?
    else {
        return Ok(None);
    };
    if !radius.is_finite() || radius < 0.0 || !period.is_finite() || period <= 0.0 {
        bail!("{ORBIT_FLAG} radius must be >= 0 and period > 0, all finite");
    }
    Ok(Some((radius, period)))
}

fn motion_resolve_from_args(args: &[String]) -> Result<Option<(f32, f32)>> {
    let Some((radius, angular_speed)) =
        pair_resolve_from_args(args, MOTION_FLAG, "<radius>,<angular_speed>")?
    else {
        return Ok(None);
    };
    if !radius.is_finite() || radius < 0.0 || !angular_speed.is_finite() {
        bail!("{MOTION_FLAG} radius must be >= 0 and angular_speed finite");
    }
    Ok(Some((radius, angular_speed)))
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
        assert!(mode_resolve_from_args(&args(&["bin", "--batch-flame-mode", "x"])).is_err());
        assert!(steps_resolve_from_args(&args(&["bin", "--batch-flame-steps", "0"])).is_err());
        assert!(steps_resolve_from_args(&args(&["bin", "--batch-flame-steps", "abc"])).is_err());
    }

    #[test]
    fn style_path_only_defaults_to_all_groups() {
        let (path, groups) = style_resolve_from_args(&args(&[
            "--batch-flame-style",
            "assets/flames/styles/pillar.style.ron",
        ]))
        .unwrap()
        .unwrap();
        assert_eq!(path, "assets/flames/styles/pillar.style.ron");
        assert_eq!(groups, StyleGroups::default());
    }

    #[test]
    fn style_group_subset() {
        let (_, groups) =
            style_resolve_from_args(&args(&["--batch-flame-style", "s.ron,motion,optics"]))
                .unwrap()
                .unwrap();
        assert!(groups.motion && groups.optics && !groups.texture);
    }

    #[test]
    fn style_unknown_group_error() {
        assert!(style_resolve_from_args(&args(&["--batch-flame-style", "s.ron,shape"])).is_err());
    }

    #[test]
    fn set_combined_form() {
        let pairs =
            set_resolve_from_args(&args(&["--batch-flame-set=noise_amplitude=0.35"])).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "noise_amplitude");
        assert!((pairs[0].1 - 0.35).abs() < 1e-6);
    }

    #[test]
    fn set_separate_form() {
        let pairs =
            set_resolve_from_args(&args(&["--batch-flame-set", "noise_amplitude=0.35"])).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "noise_amplitude");
        assert!((pairs[0].1 - 0.35).abs() < 1e-6);
    }

    #[test]
    fn set_unknown_key_error() {
        let err =
            set_resolve_from_args(&args(&["--batch-flame-set", "invalid_key=1.0"])).unwrap_err();
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
        let result = preset_resolve_from_args(&args(&["--batch-flame-preset", "candle"])).unwrap();
        assert_eq!(result, Some(String::from("candle")));
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
        let resolved =
            texture_fit_resolve_from_args(&args(&["bin", "--batch-flame-texture", "image.png"]))
                .unwrap()
                .unwrap();
        assert_eq!(resolved.0, "image.png");
        assert!((resolved.1 - 1.0).abs() < 1e-6);
        assert!(!resolved.2);
    }

    #[test]
    fn texture_fit_path_with_blend() {
        let resolved = texture_fit_resolve_from_args(&args(&[
            "bin",
            "--batch-flame-texture",
            "image.png,0.4",
        ]))
        .unwrap()
        .unwrap();
        assert_eq!(resolved.0, "image.png");
        assert!((resolved.1 - 0.4).abs() < 1e-6);
        assert!(!resolved.2);
    }

    #[test]
    fn texture_fit_invalid_blend_is_err() {
        assert!(texture_fit_resolve_from_args(&args(&[
            "bin",
            "--batch-flame-texture",
            "image.png,abc"
        ]))
        .is_err());
    }

    #[test]
    fn texture_fit_profile() {
        let resolved = texture_fit_resolve_from_args(&args(&[
            "bin",
            "--batch-flame-texture",
            "image.png,0.5,profile",
        ]))
        .unwrap()
        .unwrap();
        assert_eq!(resolved.0, "image.png");
        assert!((resolved.1 - 0.5).abs() < 1e-6);
        assert!(resolved.2);
    }

    #[test]
    fn heat_plume_defaults_and_pairs() {
        assert_eq!(
            heat_plume_resolve_from_args(&args(&["bin", "--batch-heat-plume"])).unwrap(),
            Some((10.0, 0.5))
        );
        assert_eq!(
            heat_plume_resolve_from_args(&args(&["bin", "--batch-heat-plume", "4"])).unwrap(),
            Some((4.0, 0.5))
        );
        assert_eq!(
            heat_plume_resolve_from_args(&args(&["bin", "--batch-heat-plume", "4,0.2"])).unwrap(),
            Some((4.0, 0.2))
        );
        assert!(
            heat_plume_resolve_from_args(&args(&["bin", "--batch-heat-plume", "1,2,3"])).is_err()
        );
        assert!(heat_plume_resolve_from_args(&args(&["bin", "--batch-heat-plume", "-1"])).is_err());
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
