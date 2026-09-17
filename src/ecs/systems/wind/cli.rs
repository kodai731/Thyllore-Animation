use anyhow::{bail, Result};
use thyllore_effect_core::{
    find_scalar_param, WindDebugView, WindResolveScale, WindShadingMode, WIND_SCALAR_PARAMS,
};

use crate::asset::AssetStorage;
use crate::ecs::component::WindTornadoEffect;
use crate::ecs::resource::WindRenderSettings;
use crate::ecs::systems::cli_args::scalar_set_resolve_from_args;
use crate::ecs::world::World;
use crate::hooks::bootstrap::BootstrapOverrides;

const TIME_FLAG: &str = "--batch-wind-time";
const MODE_FLAG: &str = "--batch-wind-mode";
const DEBUG_VIEW_FLAG: &str = "--batch-wind-debug-view";
const RESOLVE_SCALE_FLAG: &str = "--batch-wind-resolve-scale";
const SET_FLAG: &str = "--batch-wind-set";

#[derive(Debug)]
pub struct WindOverrides {
    pub fixed_time: Option<f32>,
    pub mode: Option<WindShadingMode>,
    pub resolve_scale: Option<WindResolveScale>,
    pub debug_view: Option<WindDebugView>,
    pub set: Vec<(String, f32)>,
}

impl BootstrapOverrides for WindOverrides {
    const NAME: &'static str = "wind";

    fn resolve(args: &[String]) -> Result<Self> {
        Ok(Self {
            fixed_time: fixed_time_resolve_from_args(args)?,
            mode: mode_resolve_from_args(args)?,
            resolve_scale: resolve_scale_resolve_from_args(args)?,
            debug_view: debug_view_resolve_from_args(args)?,
            set: scalar_set_resolve_from_args(args, SET_FLAG, &wind_set_valid_keys())?,
        })
    }

    fn apply(&self, world: &mut World, _assets: &mut AssetStorage) -> Result<()> {
        self.apply_render_settings(world);

        if self.set.is_empty() {
            return Ok(());
        }
        for entity in world.query_winds() {
            let Some(mut effect) = world.get_component::<WindTornadoEffect>(entity).cloned() else {
                continue;
            };
            apply_wind_overrides(&mut effect, &self.set);
            world.insert_component(entity, effect);
        }
        Ok(())
    }
}

crate::bootstrap_hook!(WindOverrides);

impl WindOverrides {
    fn apply_render_settings(&self, world: &mut World) {
        if self.fixed_time.is_none()
            && self.mode.is_none()
            && self.resolve_scale.is_none()
            && self.debug_view.is_none()
        {
            return;
        }
        let mut settings = world.resource_mut::<WindRenderSettings>();
        if let Some(seconds) = self.fixed_time {
            settings.batch_fixed_time = Some(seconds);
        }
        if let Some(mode) = self.mode {
            settings.shading_mode = mode;
        }
        if let Some(resolve_scale) = self.resolve_scale {
            settings.resolve_scale = resolve_scale;
        }
        if let Some(debug_view) = self.debug_view {
            settings.debug_view = debug_view;
        }
    }
}

pub(crate) fn wind_set_valid_keys() -> Vec<&'static str> {
    WIND_SCALAR_PARAMS.iter().map(|param| param.name).collect()
}

pub fn apply_wind_overrides(effect: &mut WindTornadoEffect, overrides: &[(String, f32)]) {
    for (key, value) in overrides {
        let param = find_scalar_param(WIND_SCALAR_PARAMS, key)
            .unwrap_or_else(|| unreachable!("unknown key (parser should have rejected)"));
        (param.set)(effect, *value);
    }
}

fn mode_resolve_from_args(args: &[String]) -> Result<Option<WindShadingMode>> {
    let Some(position) = args.iter().position(|arg| arg == MODE_FLAG) else {
        return Ok(None);
    };
    let Some(value) = args.get(position + 1) else {
        bail!("{MODE_FLAG} requires a value: closed|reference");
    };
    let mode = WindShadingMode::parse(value)
        .ok_or_else(|| anyhow::anyhow!("invalid wind mode '{value}': expected closed|reference"))?;
    Ok(Some(mode))
}

fn debug_view_resolve_from_args(args: &[String]) -> Result<Option<WindDebugView>> {
    let Some(position) = args.iter().position(|arg| arg == DEBUG_VIEW_FLAG) else {
        return Ok(None);
    };
    let Some(value) = args.get(position + 1) else {
        bail!("{DEBUG_VIEW_FLAG} requires a value: off|depth|knots|coverage");
    };
    let view = WindDebugView::parse(value).ok_or_else(|| {
        anyhow::anyhow!("invalid wind debug view '{value}': expected off|depth|knots|coverage")
    })?;
    Ok(Some(view))
}

fn resolve_scale_resolve_from_args(args: &[String]) -> Result<Option<WindResolveScale>> {
    let Some(position) = args.iter().position(|arg| arg == RESOLVE_SCALE_FLAG) else {
        return Ok(None);
    };
    let Some(value) = args.get(position + 1) else {
        bail!("{RESOLVE_SCALE_FLAG} requires a value: full|half");
    };
    let scale = WindResolveScale::parse(value).ok_or_else(|| {
        anyhow::anyhow!("invalid wind resolve scale '{value}': expected full|half")
    })?;
    Ok(Some(scale))
}

fn fixed_time_resolve_from_args(args: &[String]) -> Result<Option<f32>> {
    let Some(position) = args.iter().position(|arg| arg == TIME_FLAG) else {
        return Ok(None);
    };
    let Some(value) = args.get(position + 1) else {
        bail!("{TIME_FLAG} requires a value (seconds)");
    };
    let seconds: f32 = value
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid wind time '{value}': expected float seconds"))?;
    Ok(Some(seconds))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn resolve_mode_and_debug_view() {
        let overrides = WindOverrides::resolve(&args(&[
            "bin",
            "--batch-wind-mode",
            "reference",
            "--batch-wind-debug-view",
            "depth",
        ]))
        .unwrap();
        assert_eq!(overrides.mode, Some(WindShadingMode::ReferenceQuadrature));
        assert_eq!(overrides.debug_view, Some(WindDebugView::OpticalDepth));
        assert!(mode_resolve_from_args(&args(&["bin", "--batch-wind-mode", "x"])).is_err());

        let coverage =
            WindOverrides::resolve(&args(&["bin", "--batch-wind-debug-view", "coverage"])).unwrap();
        assert_eq!(coverage.debug_view, Some(WindDebugView::Coverage));
    }

    #[test]
    fn resolve_resolve_scale() {
        assert_eq!(
            WindOverrides::resolve(&args(&["bin"]))
                .unwrap()
                .resolve_scale,
            None
        );

        let half =
            WindOverrides::resolve(&args(&["bin", "--batch-wind-resolve-scale", "half"])).unwrap();
        assert_eq!(half.resolve_scale, Some(WindResolveScale::Half));

        let full =
            WindOverrides::resolve(&args(&["bin", "--batch-wind-resolve-scale", "full"])).unwrap();
        assert_eq!(full.resolve_scale, Some(WindResolveScale::Full));

        assert!(resolve_scale_resolve_from_args(&args(&[
            "bin",
            "--batch-wind-resolve-scale",
            "x"
        ]))
        .is_err());
    }

    #[test]
    fn resolve_fixed_time() {
        let overrides =
            WindOverrides::resolve(&args(&["bin", "--batch-wind-time", "10.5"])).unwrap();
        assert_eq!(overrides.fixed_time, Some(10.5));
        assert_eq!(
            WindOverrides::resolve(&args(&["bin"])).unwrap().fixed_time,
            None
        );

        assert!(fixed_time_resolve_from_args(&args(&["bin", "--batch-wind-time"])).is_err());
        assert!(fixed_time_resolve_from_args(&args(&["bin", "--batch-wind-time", "abc"])).is_err());
    }

    #[test]
    fn set_parses_both_forms_and_rejects_unknown_key() {
        let pairs = WindOverrides::resolve(&args(&["--batch-wind-set=wall_strength=0.5"]))
            .unwrap()
            .set;
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "wall_strength");
        assert!((pairs[0].1 - 0.5).abs() < 1e-6);

        let separate = WindOverrides::resolve(&args(&["--batch-wind-set", "wall_strength=0.5"]))
            .unwrap()
            .set;
        assert_eq!(separate, pairs);

        let err =
            WindOverrides::resolve(&args(&["--batch-wind-set", "invalid_key=1.0"])).unwrap_err();
        assert!(err.to_string().contains("invalid_key"));
    }

    #[test]
    fn apply_overrides_no_panic_for_all_keys() {
        for key in wind_set_valid_keys() {
            let mut effect = WindTornadoEffect::default();
            apply_wind_overrides(&mut effect, &[(key.to_string(), 1.0)]);

            let param =
                find_scalar_param(WIND_SCALAR_PARAMS, key).expect("valid key is registered");
            assert_eq!((param.get)(&effect), 1.0, "{key}");
        }
    }
}
