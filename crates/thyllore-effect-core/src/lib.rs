extern crate self as thyllore_effect_core;

mod debug;
mod effect_facade;
pub mod flame;
mod gpu_pack;
pub mod lightning;
mod scene_convert;
pub(crate) mod volume;
pub mod water;
pub mod wind;

pub use thyllore_scene_core::{ScalarParam, SceneComponent, UiParam};

pub use debug::{
    probe_flame_wall, WallProbeRay, WallProbeReport, WallProbeView, WALL_PROBE_GRID_COLS,
    WALL_PROBE_GRID_ROWS,
};
pub use effect_facade::{EffectPresets, Placement};
pub use gpu_pack::UboPack;
pub use thyllore_effect_derive::UboPack;

#[cfg(test)]
mod analytic_manifest;
#[cfg(test)]
mod test_support;

#[cfg(any(feature = "python", feature = "python-test"))]
mod pybindings;
