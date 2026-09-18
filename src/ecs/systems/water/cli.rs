use anyhow::Result;
use clap::Args;
use thyllore_effect_core::WaterSecondaryRays;

use crate::asset::AssetStorage;
use crate::ecs::resource::{BatchRun, WaterRenderSettings};
use crate::ecs::world::World;
use crate::hooks::bootstrap::BootstrapOverrides;

const PROBE_DEBUG_VIEW: i32 = 3;

#[derive(Args, Debug)]
pub struct WaterOverrides {
    #[arg(long = "batch-water-debug-view")]
    pub debug_view: Option<i32>,
    #[arg(long = "batch-water-secondary")]
    pub secondary: Option<WaterSecondaryRays>,
    #[arg(long = "batch-water-caustic-debug")]
    pub caustic_debug: Option<i32>,
    #[arg(long = "batch-water-history")]
    pub history_weight: Option<f32>,
    #[arg(long = "batch-water-time")]
    pub fixed_time: Option<f32>,
}

impl BootstrapOverrides for WaterOverrides {
    const NAME: &'static str = "water";

    fn apply(&self, world: &mut World, _assets: &mut AssetStorage) -> Result<()> {
        let probe_requested = world
            .get_resource::<BatchRun>()
            .is_some_and(|batch| batch.water_probe_path.is_some());
        let debug_view = match (self.debug_view, probe_requested) {
            (Some(view), _) => Some(view),
            (None, true) => Some(PROBE_DEBUG_VIEW),
            (None, false) => None,
        };
        if debug_view.is_none()
            && self.secondary.is_none()
            && self.caustic_debug.is_none()
            && self.history_weight.is_none()
            && self.fixed_time.is_none()
        {
            return Ok(());
        }

        let mut settings = world.resource_mut::<WaterRenderSettings>();
        if let Some(view) = debug_view {
            settings.debug_view = view;
        }
        if let Some(secondary) = self.secondary {
            settings.secondary_rays = secondary;
        }
        if let Some(caustic_debug) = self.caustic_debug {
            settings.caustic_debug = caustic_debug;
        }
        if let Some(weight) = self.history_weight {
            settings.batch_history_weight = Some(weight);
        }
        if let Some(seconds) = self.fixed_time {
            settings.batch_fixed_time = Some(seconds);
        }
        Ok(())
    }
}

crate::bootstrap_hook!(WaterOverrides);

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn resolve_debug_view_and_time() {
        let overrides = WaterOverrides::resolve(&args(&[
            "bin",
            "--batch-water-debug-view",
            "2",
            "--batch-water-time",
            "1.5",
        ]))
        .unwrap();
        assert_eq!(overrides.debug_view, Some(2));
        assert_eq!(overrides.fixed_time, Some(1.5));
        assert!(WaterOverrides::resolve(&args(&["bin", "--batch-water-debug-view", "x"])).is_err());
    }

    #[test]
    fn resolve_secondary_rays() {
        let overrides =
            WaterOverrides::resolve(&args(&["bin", "--batch-water-secondary", "screenspace"]))
                .unwrap();
        assert_eq!(overrides.secondary, Some(WaterSecondaryRays::ScreenSpace));
        assert!(WaterOverrides::resolve(&args(&["bin", "--batch-water-secondary", "x"])).is_err());
    }

    #[test]
    fn probe_run_defaults_the_debug_view() {
        let mut world = World::new();
        world.insert_resource(WaterRenderSettings::default());
        let mut batch = BatchRun::new(PathBuf::from("/tmp/out.png"), 1);
        batch.water_probe_path = Some(PathBuf::from("/tmp/probe.json"));
        world.insert_resource(batch);

        let overrides = WaterOverrides::resolve(&args(&["bin"])).unwrap();
        overrides
            .apply(&mut world, &mut AssetStorage::new())
            .unwrap();
        assert_eq!(
            world.resource::<WaterRenderSettings>().debug_view,
            PROBE_DEBUG_VIEW
        );
    }

    #[test]
    fn apply_without_flags_leaves_an_empty_world_untouched() {
        let overrides = WaterOverrides::resolve(&args(&["bin"])).unwrap();
        let mut world = World::new();
        overrides
            .apply(&mut world, &mut AssetStorage::new())
            .unwrap();
        assert!(world.get_resource::<WaterRenderSettings>().is_none());
    }
}
