use pyo3::prelude::*;

use super::{effect, flame, lightning, water, wind};

#[pymodule]
fn thyllore_effect_core(_py: Python<'_>, m: &Bound<PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(effect::blender_to_engine_matrix, m)?)?;
    m.add_function(wrap_pyfunction!(effect::engine_projection, m)?)?;
    m.add_function(wrap_pyfunction!(effect::engine_projection_from_window, m)?)?;
    m.add_function(wrap_pyfunction!(effect::object_local_points, m)?)?;
    flame::register(m)?;
    lightning::register(m)?;
    water::register(m)?;
    wind::register(m)?;
    Ok(())
}
