use pyo3::prelude::*;
use pythonize::depythonize;

use super::avatar_setup::depythonize_error_to_pyerr;
use super::chain_gen::pythonize_value;
use super::error::anyhow_to_pyerr;
use crate::avatar_setup_bridge::BoneSpec;
use crate::clip_bake_bridge::bake_clip_file as bridge_bake_clip_file;

/// Bakes a `.anim.ron` humanoid clip onto `bones` (with `rest_frame`) at `fps`, as the engine plays a template.
#[pyfunction]
pub fn bake_clip_file<'py>(
    py: Python<'py>,
    bones: &Bound<PyAny>,
    clip_ron: &str,
    fps: u32,
) -> PyResult<Bound<'py, PyAny>> {
    let specs: Vec<BoneSpec> = depythonize(bones).map_err(depythonize_error_to_pyerr)?;
    let baked = bridge_bake_clip_file(&specs, clip_ron, fps).map_err(anyhow_to_pyerr)?;
    pythonize_value(py, &baked)
}
