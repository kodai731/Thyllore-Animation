use anyhow::{bail, Result};
use thyllore_effect_core::WaterSecondaryRays;

use crate::asset::AssetStorage;
use crate::ecs::resource::{BatchRun, WaterRenderSettings};
use crate::ecs::world::World;
use crate::hooks::bootstrap::BootstrapOverrides;

const DEBUG_VIEW_FLAG: &str = "--batch-water-debug-view";
const SECONDARY_FLAG: &str = "--batch-water-secondary";
const CAUSTIC_DEBUG_FLAG: &str = "--batch-water-caustic-debug";
const HISTORY_FLAG: &str = "--batch-water-history";
const TIME_FLAG: &str = "--batch-water-time";
const PROBE_DEBUG_VIEW: i32 = 3;

#[derive(Debug)]
pub struct WaterOverrides {
    pub debug_view: Option<i32>,
    pub secondary: Option<WaterSecondaryRays>,
    pub caustic_debug: Option<i32>,
    pub history_weight: Option<f32>,
    pub fixed_time: Option<f32>,
}

impl BootstrapOverrides for WaterOverrides {
    const NAME: &'static str = "water";

    fn resolve(args: &[String]) -> Result<Self> {
        Ok(Self {
            debug_view: debug_view_resolve_from_args(args)?,
            secondary: secondary_resolve_from_args(args)?,
            caustic_debug: caustic_debug_resolve_from_args(args)?,
            history_weight: history_weight_resolve_from_args(args)?,
            fixed_time: fixed_time_resolve_from_args(args)?,
        })
    }

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

fn debug_view_resolve_from_args(args: &[String]) -> Result<Option<i32>> {
    let Some(position) = args.iter().position(|arg| arg == DEBUG_VIEW_FLAG) else {
        return Ok(None);
    };
    let Some(value) = args.get(position + 1) else {
        bail!("{DEBUG_VIEW_FLAG} requires a value (integer debug view index)");
    };
    let view: i32 = value
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid water debug view '{value}': expected integer"))?;
    Ok(Some(view))
}

fn caustic_debug_resolve_from_args(args: &[String]) -> Result<Option<i32>> {
    let Some(position) = args.iter().position(|arg| arg == CAUSTIC_DEBUG_FLAG) else {
        return Ok(None);
    };
    let Some(value) = args.get(position + 1) else {
        bail!("{CAUSTIC_DEBUG_FLAG} requires a value (integer caustic debug mode)");
    };
    let mode: i32 = value
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid water caustic debug '{value}': expected integer"))?;
    Ok(Some(mode))
}

fn secondary_resolve_from_args(args: &[String]) -> Result<Option<WaterSecondaryRays>> {
    let Some(position) = args.iter().position(|arg| arg == SECONDARY_FLAG) else {
        return Ok(None);
    };
    let Some(value) = args.get(position + 1) else {
        bail!("{SECONDARY_FLAG} requires a value (rayquery|screenspace|raytracing)");
    };
    let secondary = WaterSecondaryRays::parse(value).ok_or_else(|| {
        anyhow::anyhow!("{SECONDARY_FLAG} requires a value (rayquery|screenspace|raytracing)")
    })?;
    Ok(Some(secondary))
}

fn history_weight_resolve_from_args(args: &[String]) -> Result<Option<f32>> {
    let Some(position) = args.iter().position(|arg| arg == HISTORY_FLAG) else {
        return Ok(None);
    };
    let Some(value) = args.get(position + 1) else {
        bail!("{HISTORY_FLAG} requires a value (history blend weight)");
    };
    let weight: f32 = value
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid water history weight '{value}': expected float"))?;
    Ok(Some(weight))
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
        .map_err(|_| anyhow::anyhow!("invalid water time '{value}': expected float seconds"))?;
    Ok(Some(seconds))
}

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
        assert!(
            debug_view_resolve_from_args(&args(&["bin", "--batch-water-debug-view", "x"])).is_err()
        );
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
