use pyo3::prelude::*;
use thyllore_math_core::WireSegment;

#[pyfunction]
pub fn wire_sphere_segments(center: [f32; 3], radius: f32, segments: u32) -> Vec<WireSegment> {
    thyllore_math_core::wire_sphere_segments(center, radius, segments)
}

#[pyfunction]
pub fn wire_capsule_segments(
    head: [f32; 3],
    tail: [f32; 3],
    radius: f32,
    segments: u32,
) -> Vec<WireSegment> {
    thyllore_math_core::wire_capsule_segments(head, tail, radius, segments)
}
