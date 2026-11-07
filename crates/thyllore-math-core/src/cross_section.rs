use glam::Vec3;

use crate::centroid;

pub const CROSS_SECTION_SLICE_COUNT: usize = 64;

/// The points of one slab along an axis and the slab centre's distance along that axis from the origin.
pub struct AxisSlice {
    pub members: Vec<Vec3>,
    pub distance: f32,
}

/// Cuts the cloud into `slice_count` equal slabs along `axis` from `origin` and returns the slab whose
/// points reach farthest from the cloud centroid perpendicular to the axis.
pub fn widest_cross_section(
    points: &[Vec3],
    origin: Vec3,
    axis: Vec3,
    slice_count: usize,
) -> Option<AxisSlice> {
    let center = centroid(points)?;
    let distances: Vec<f32> = points.iter().map(|&p| (p - origin).dot(axis)).collect();
    let min_distance = distances.iter().copied().reduce(f32::min)?;
    let max_distance = distances.iter().copied().reduce(f32::max)?;
    let slice_width = (max_distance - min_distance) / slice_count as f32;
    let slice_of = |distance: f32| -> usize {
        if slice_width > 0.0 {
            (((distance - min_distance) / slice_width) as usize).min(slice_count - 1)
        } else {
            0
        }
    };

    let mut widest_radius = vec![0.0f32; slice_count];
    for (&p, &distance) in points.iter().zip(&distances) {
        let offset = p - center;
        let radius = (offset - axis * offset.dot(axis)).length();
        let slice = slice_of(distance);
        widest_radius[slice] = widest_radius[slice].max(radius);
    }
    let widest = (0..slice_count)
        .max_by(|a, b| widest_radius[*a].total_cmp(&widest_radius[*b]))
        .unwrap_or(0);

    let members = points
        .iter()
        .zip(&distances)
        .filter(|(_, &distance)| slice_of(distance) == widest)
        .map(|(&p, _)| p)
        .collect();
    Some(AxisSlice {
        members,
        distance: min_distance + (widest as f32 + 0.5) * slice_width,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ring(height: f32, radius: f32, count: usize) -> Vec<Vec3> {
        (0..count)
            .map(|i| {
                let angle = i as f32 / count as f32 * crate::TAU;
                Vec3::new(radius * angle.cos(), height, radius * angle.sin())
            })
            .collect()
    }

    #[test]
    fn the_widest_ring_wins_and_its_distance_runs_along_the_axis() {
        let mut points = ring(1.0, 0.1, 16);
        points.extend(ring(2.0, 0.5, 16));
        points.extend(ring(3.0, 0.2, 16));
        let down = Vec3::new(0.0, -1.0, 0.0);

        let slice =
            widest_cross_section(&points, Vec3::new(0.0, 3.0, 0.0), down, 4).expect("points exist");

        assert_eq!(slice.members.len(), 16);
        assert!(slice.members.iter().all(|p| (p.y - 2.0).abs() < 1e-6));
        assert!((slice.distance - 1.0).abs() < 0.3);
    }

    #[test]
    fn no_points_gives_no_slice() {
        assert!(widest_cross_section(&[], Vec3::new(0.0, 0.0, 0.0), Vec3::Y, 4).is_none());
    }
}
