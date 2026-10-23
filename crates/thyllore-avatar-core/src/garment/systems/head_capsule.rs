use cgmath::{InnerSpace, Vector3};
use thyllore_math_core::world_y_axis;

use super::chain_gen::{
    attachment_or_target, build_azimuth_basis, collect_target_vertices, widest_slice,
};
use crate::garment::components::chain_gen::{ChainGenInput, ChainGenParams, ChainRole};

/// A vertical capsule standing on the widest cross-section of the skin the root bone drives.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeadCapsule {
    pub bottom: Vector3<f32>,
    pub top: Vector3<f32>,
    pub radius: f32,
}

pub fn estimate_head_capsule(
    attachment: &[Vector3<f32>],
    root_position: Vector3<f32>,
) -> Option<HeadCapsule> {
    let up = world_y_axis();
    let slice = widest_slice(attachment, root_position)?;
    let (reference, side) = build_azimuth_basis(up);

    let extent = |axis: Vector3<f32>| -> (f32, f32) {
        let projected = slice.members.iter().map(|p| p.dot(axis));
        let lo = projected.clone().reduce(f32::min).unwrap_or(0.0);
        let hi = projected.reduce(f32::max).unwrap_or(0.0);
        (lo, hi)
    };
    let (reference_lo, reference_hi) = extent(reference);
    let (side_lo, side_hi) = extent(side);
    let radius = ((reference_hi - reference_lo) * 0.5).max((side_hi - side_lo) * 0.5);

    let bottom_height = root_position.dot(up) - slice.progress;
    let bottom = reference * (reference_lo + reference_hi) * 0.5
        + side * (side_lo + side_hi) * 0.5
        + up * bottom_height;
    let top_height = attachment
        .iter()
        .map(|p| p.dot(up))
        .reduce(f32::max)
        .unwrap_or(bottom_height);
    let top = bottom + up * (top_height - radius - bottom_height).max(0.0);

    Some(HeadCapsule {
        bottom,
        top,
        radius,
    })
}

pub fn hair_head_capsule(input: &ChainGenInput, params: &ChainGenParams) -> Option<HeadCapsule> {
    match params.role {
        ChainRole::Hair => {
            let vertices = collect_target_vertices(input);
            estimate_head_capsule(&attachment_or_target(input, &vertices), input.root_position)
        }
        ChainRole::Skirt => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    fn ring(height: f32, radius: f32, count: usize) -> Vec<Vector3<f32>> {
        (0..count)
            .map(|i| {
                let angle = i as f32 / count as f32 * TAU;
                Vector3::new(radius * angle.cos(), height, radius * angle.sin())
            })
            .collect()
    }

    fn egg_head() -> Vec<Vector3<f32>> {
        let mut points = Vec::new();
        for (height, radius) in [
            (1.10, 0.05),
            (1.15, 0.08),
            (1.20, 0.09),
            (1.25, 0.08),
            (1.30, 0.05),
            (1.32, 0.02),
        ] {
            points.extend(ring(height, radius, 32));
        }
        points
    }

    #[test]
    fn capsule_stands_on_the_widest_ring_and_reaches_the_top() {
        let capsule = estimate_head_capsule(&egg_head(), Vector3::new(0.0, 1.13, 0.0))
            .expect("head has points");

        assert!((capsule.radius - 0.09).abs() < 1e-3);
        assert!((capsule.bottom.y - 1.20).abs() < 0.01);
        assert!((capsule.top.y - (1.32 - 0.09)).abs() < 1e-3);
        assert!(capsule.bottom.x.abs() < 1e-4 && capsule.bottom.z.abs() < 1e-4);
    }

    #[test]
    fn a_flat_head_becomes_a_sphere() {
        let points = ring(1.2, 0.1, 32);
        let capsule =
            estimate_head_capsule(&points, Vector3::new(0.0, 1.13, 0.0)).expect("ring has points");

        assert_eq!(capsule.top, capsule.bottom);
        assert!((capsule.radius - 0.1).abs() < 1e-4);
    }

    #[test]
    fn radius_is_the_larger_half_extent_and_the_axis_is_centred() {
        let points: Vec<Vector3<f32>> = ring(1.2, 1.0, 64)
            .into_iter()
            .map(|p| Vector3::new(p.x * 0.08 + 0.01, p.y, p.z * 0.06 + 0.02))
            .collect();
        let capsule =
            estimate_head_capsule(&points, Vector3::new(0.0, 1.13, 0.0)).expect("ring has points");

        assert!((capsule.radius - 0.08).abs() < 1e-3);
        assert!((capsule.bottom.x - 0.01).abs() < 1e-3);
        assert!((capsule.bottom.z - 0.02).abs() < 1e-3);
    }

    #[test]
    fn no_points_gives_no_capsule() {
        assert!(estimate_head_capsule(&[], Vector3::new(0.0, 1.13, 0.0)).is_none());
    }
}
