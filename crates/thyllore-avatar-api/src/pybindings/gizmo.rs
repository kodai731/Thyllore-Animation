use pyo3::prelude::*;
use pythonize::depythonize;
use thyllore_math_core::WireSegment;
use thyllore_ml_api::python::depythonize_error_to_pyerr;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct WireSphereRequest {
    center: [f32; 3],
    radius: f32,
    segments: u32,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct WireCapsuleRequest {
    head: [f32; 3],
    tail: [f32; 3],
    radius: f32,
    segments: u32,
}

#[pyfunction]
pub fn wire_sphere_segments(request: &Bound<PyAny>) -> PyResult<Vec<WireSegment>> {
    let request: WireSphereRequest = depythonize(request).map_err(depythonize_error_to_pyerr)?;
    Ok(thyllore_math_core::wire_sphere_segments(
        request.center,
        request.radius,
        request.segments,
    ))
}

#[pyfunction]
pub fn wire_capsule_segments(request: &Bound<PyAny>) -> PyResult<Vec<WireSegment>> {
    let request: WireCapsuleRequest = depythonize(request).map_err(depythonize_error_to_pyerr)?;
    Ok(thyllore_math_core::wire_capsule_segments(
        request.head,
        request.tail,
        request.radius,
        request.segments,
    ))
}
