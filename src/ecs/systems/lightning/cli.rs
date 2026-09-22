use anyhow::Result;
use clap::Args;
use thyllore_effect_core::{
    apply_lightning_preset, find_scalar_param, LightningDebugView, LightningShadingMode,
    LIGHTNING_PRESET_NAMES, LIGHTNING_SCALAR_PARAMS,
};

use crate::asset::AssetStorage;
use crate::ecs::component::LightningEffect;
use crate::ecs::resource::LightningRenderSettings;
use crate::ecs::systems::cli_args::scalar_assignment_parse;
use crate::ecs::world::World;
use crate::hooks::bootstrap::BootstrapOverrides;

#[derive(Args, Debug)]
pub struct LightningOverrides {
    #[arg(long = "batch-lightning-time")]
    pub fixed_time: Option<f32>,
    #[arg(long = "batch-lightning-mode")]
    pub mode: Option<LightningShadingMode>,
    #[arg(long = "batch-lightning-debug-view")]
    pub debug_view: Option<LightningDebugView>,
    #[arg(long = "batch-lightning-steps", value_parser = clap::value_parser!(u32).range(1..))]
    pub steps: Option<u32>,
    #[arg(long = "batch-lightning-preset", value_parser = lightning_preset_parse)]
    pub preset: Option<String>,
    #[arg(long = "batch-lightning-set", value_parser = lightning_set_entry_parse)]
    pub set: Vec<(String, f32)>,
}

impl BootstrapOverrides for LightningOverrides {
    const NAME: &'static str = "lightning";

    fn apply(&self, world: &mut World, _assets: &mut AssetStorage) -> Result<()> {
        self.apply_render_settings(world);

        if self.preset.is_none() && self.set.is_empty() {
            return Ok(());
        }
        for entity in world.entities_with::<LightningEffect>() {
            let Some(mut effect) = world.get_component::<LightningEffect>(entity).cloned() else {
                continue;
            };
            if let Some(name) = self.preset.as_deref() {
                apply_lightning_preset(&mut effect, name);
            }
            apply_lightning_overrides(&mut effect, &self.set);
            world.insert_component(entity, effect);
        }
        Ok(())
    }
}

crate::bootstrap_hook!(LightningOverrides);

impl LightningOverrides {
    fn apply_render_settings(&self, world: &mut World) {
        if self.fixed_time.is_none()
            && self.mode.is_none()
            && self.debug_view.is_none()
            && self.steps.is_none()
        {
            return;
        }
        let mut settings = world.resource_mut::<LightningRenderSettings>();
        if let Some(seconds) = self.fixed_time {
            settings.batch_fixed_time = Some(seconds);
        }
        if let Some(mode) = self.mode {
            settings.shading_mode = mode;
        }
        if let Some(debug_view) = self.debug_view {
            settings.debug_view = debug_view;
        }
        if let Some(step_count) = self.steps {
            settings.reference_step_count = step_count;
        }
    }
}

pub(crate) fn lightning_set_valid_keys() -> Vec<&'static str> {
    LIGHTNING_SCALAR_PARAMS
        .iter()
        .map(|param| param.name)
        .collect()
}

fn lightning_set_entry_parse(text: &str) -> Result<(String, f32), String> {
    scalar_assignment_parse(text, &lightning_set_valid_keys())
}

fn lightning_preset_parse(text: &str) -> Result<String, String> {
    if !LIGHTNING_PRESET_NAMES.contains(&text) {
        return Err(format!(
            "unknown lightning preset '{text}'. Valid presets: {}",
            LIGHTNING_PRESET_NAMES.join(", ")
        ));
    }
    Ok(text.to_string())
}

pub fn apply_lightning_overrides(effect: &mut LightningEffect, overrides: &[(String, f32)]) {
    for (key, value) in overrides {
        let param = find_scalar_param(LIGHTNING_SCALAR_PARAMS, key)
            .unwrap_or_else(|| unreachable!("unknown key (parser should have rejected)"));
        (param.set)(effect, *value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn resolve_mode_and_debug_view() {
        let overrides = LightningOverrides::resolve(&args(&[
            "bin",
            "--batch-lightning-mode",
            "reference",
            "--batch-lightning-debug-view",
            "hits",
        ]))
        .unwrap();
        assert_eq!(
            overrides.mode,
            Some(LightningShadingMode::ReferenceQuadrature)
        );
        assert_eq!(overrides.debug_view, Some(LightningDebugView::SegmentHits));
        assert!(
            LightningOverrides::resolve(&args(&["bin", "--batch-lightning-mode", "x"])).is_err()
        );

        let coverage = LightningOverrides::resolve(&args(&[
            "bin",
            "--batch-lightning-debug-view",
            "coverage",
        ]))
        .unwrap();
        assert_eq!(coverage.debug_view, Some(LightningDebugView::Coverage));

        let core =
            LightningOverrides::resolve(&args(&["bin", "--batch-lightning-debug-view", "core"]))
                .unwrap();
        assert_eq!(core.debug_view, Some(LightningDebugView::CoreCoverage));
    }

    #[test]
    fn resolve_fixed_time() {
        let overrides =
            LightningOverrides::resolve(&args(&["bin", "--batch-lightning-time", "0.25"])).unwrap();
        assert_eq!(overrides.fixed_time, Some(0.25));
        assert_eq!(
            LightningOverrides::resolve(&args(&["bin"]))
                .unwrap()
                .fixed_time,
            None
        );

        assert!(LightningOverrides::resolve(&args(&["bin", "--batch-lightning-time"])).is_err());
        assert!(
            LightningOverrides::resolve(&args(&["bin", "--batch-lightning-time", "abc"])).is_err()
        );
    }

    #[test]
    fn resolve_steps() {
        let overrides =
            LightningOverrides::resolve(&args(&["bin", "--batch-lightning-steps", "4"])).unwrap();
        assert_eq!(overrides.steps, Some(4));
        assert_eq!(
            LightningOverrides::resolve(&args(&["bin"])).unwrap().steps,
            None
        );

        assert!(LightningOverrides::resolve(&args(&["bin", "--batch-lightning-steps"])).is_err());
        assert!(
            LightningOverrides::resolve(&args(&["bin", "--batch-lightning-steps", "abc"])).is_err()
        );
        assert!(
            LightningOverrides::resolve(&args(&["bin", "--batch-lightning-steps", "0"])).is_err()
        );
    }

    #[test]
    fn set_parses_both_forms_and_rejects_unknown_key() {
        let pairs =
            LightningOverrides::resolve(&args(&["--batch-lightning-set=core_intensity=12.0"]))
                .unwrap()
                .set;
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "core_intensity");
        assert!((pairs[0].1 - 12.0).abs() < 1e-6);

        let separate =
            LightningOverrides::resolve(&args(&["--batch-lightning-set", "core_intensity=12.0"]))
                .unwrap()
                .set;
        assert_eq!(separate, pairs);

        let err = LightningOverrides::resolve(&args(&["--batch-lightning-set", "invalid_key=1.0"]))
            .unwrap_err();
        assert!(err.to_string().contains("invalid_key"));
    }

    #[test]
    fn preset_parses_a_known_name_and_rejects_an_unknown_one() {
        assert_eq!(
            LightningOverrides::resolve(&args(&["bin"])).unwrap().preset,
            None
        );
        assert_eq!(
            LightningOverrides::resolve(&args(&["bin", "--batch-lightning-preset", "charge"]))
                .unwrap()
                .preset,
            Some("charge".to_string())
        );

        let err = LightningOverrides::resolve(&args(&["bin", "--batch-lightning-preset", "fog"]))
            .unwrap_err();
        assert!(err.to_string().contains("fog"));
    }

    #[test]
    fn apply_overrides_no_panic_for_all_keys() {
        for key in lightning_set_valid_keys() {
            let mut effect = LightningEffect::default();
            apply_lightning_overrides(&mut effect, &[(key.to_string(), 1.0)]);

            let param =
                find_scalar_param(LIGHTNING_SCALAR_PARAMS, key).expect("valid key is registered");
            assert_eq!((param.get)(&effect), 1.0, "{key}");
        }
    }
}
