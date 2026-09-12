mod effect;
mod flame;
mod water;
mod wind;

use pyo3::prelude::*;

#[cfg(test)]
use flame::{
    flame_bounds_corners, flame_effective_optical_depth, flame_preset_params,
    flame_shader_specialization, flame_ui_params, pack_flame_ubo,
};
#[cfg(test)]
use water::water_ui_params;
#[cfg(test)]
use wind::{pack_wind_ubo, wind_preset_params};

#[pymodule]
fn thyllore_effect_core(_py: Python<'_>, m: &Bound<PyModule>) -> PyResult<()> {
    flame::register(m)?;
    water::register(m)?;
    wind::register(m)?;
    Ok(())
}

#[cfg(test)]
mod tests;
