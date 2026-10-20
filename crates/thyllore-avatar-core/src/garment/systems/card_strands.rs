use cgmath::{InnerSpace, Vector3};
use thyllore_math_core::{fit_principal_axis, world_y_axis};

use super::chain_gen::{progress_of, resample_polyline};

pub const CENTERLINE_SAMPLE_COUNT: usize = 16;
pub const MIN_STRAND_UV_CORRELATION: f32 = 0.8;

pub fn uv_strand_correlation(
    positions: &[Vector3<f32>],
    tex_coords: &[[f32; 2]],
    members: &[u32],
) -> Option<f32> {
    let strand_positions: Vec<Vector3<f32>> =
        members.iter().map(|&v| positions[v as usize]).collect();
    let principal_axis = fit_principal_axis(&strand_positions)?;

    let n = members.len() as f32;
    let projections: Vec<f32> = strand_positions
        .iter()
        .map(|p| (p - principal_axis.mean).dot(principal_axis.axis))
        .collect();
    let mean_projection = projections.iter().sum::<f32>() / n;
    let mean_v = members
        .iter()
        .map(|&v| tex_coords[v as usize][1])
        .sum::<f32>()
        / n;

    let mut sum_xy = 0.0;
    let mut sum_x2 = 0.0;
    let mut sum_y2 = 0.0;
    for (&projection, &member) in projections.iter().zip(members) {
        let x = projection - mean_projection;
        let y = tex_coords[member as usize][1] - mean_v;
        sum_xy += x * y;
        sum_x2 += x * x;
        sum_y2 += y * y;
    }

    if sum_x2 <= 0.0 || sum_y2 <= 0.0 {
        return None;
    }

    Some(sum_xy / (sum_x2 * sum_y2).sqrt())
}

fn build_v_bin_centroids(
    positions: &[Vector3<f32>],
    tex_coords: &[[f32; 2]],
    members: &[u32],
) -> Vec<Vector3<f32>> {
    let v_of = |member: u32| tex_coords[member as usize][1];
    let min_v = members
        .iter()
        .map(|&m| v_of(m))
        .reduce(f32::min)
        .unwrap_or_default();
    let max_v = members
        .iter()
        .map(|&m| v_of(m))
        .reduce(f32::max)
        .unwrap_or_default();
    let bin_width = (max_v - min_v) / CENTERLINE_SAMPLE_COUNT as f32;

    let mut bin_sums = vec![(Vector3::new(0.0, 0.0, 0.0), 0usize); CENTERLINE_SAMPLE_COUNT];
    for &member in members {
        let bin = if bin_width > 0.0 {
            (((v_of(member) - min_v) / bin_width) as usize).min(CENTERLINE_SAMPLE_COUNT - 1)
        } else {
            0
        };
        bin_sums[bin].0 += positions[member as usize];
        bin_sums[bin].1 += 1;
    }

    bin_sums
        .into_iter()
        .filter(|(_, count)| *count > 0)
        .map(|(sum, count)| sum / count as f32)
        .collect()
}

pub fn strand_centerline(
    positions: &[Vector3<f32>],
    tex_coords: &[[f32; 2]],
    members: &[u32],
    root_position: Vector3<f32>,
    band_progress: f32,
) -> Option<Vec<Vector3<f32>>> {
    let up = world_y_axis();
    let mut centroids = build_v_bin_centroids(positions, tex_coords, members);

    let (&first, &last) = (centroids.first()?, centroids.last()?);
    if progress_of(last, root_position, up) < progress_of(first, root_position, up) {
        centroids.reverse();
    }

    centroids.retain(|&centroid| progress_of(centroid, root_position, up) > band_progress);
    let &tip = centroids.last()?;
    if centroids.len() < 2 {
        return None;
    }

    let mut points = resample_polyline(&centroids, CENTERLINE_SAMPLE_COUNT - 1);
    points.push(tip);

    Some(points)
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEEP_ALL_BAND: f32 = -1.0;

    struct Strip {
        positions: Vec<Vector3<f32>>,
        tex_coords: Vec<[f32; 2]>,
        members: Vec<u32>,
    }

    impl Strip {
        fn top(&self) -> Vector3<f32> {
            self.positions
                .iter()
                .copied()
                .reduce(|a, b| if b.y > a.y { b } else { a })
                .expect("strip has vertices")
        }

        fn centerline(&self, root_position: Vector3<f32>, band_progress: f32) -> Vec<Vector3<f32>> {
            strand_centerline(
                &self.positions,
                &self.tex_coords,
                &self.members,
                root_position,
                band_progress,
            )
            .expect("centerline exists")
        }
    }

    fn build_strip(rows: usize, place_row: impl Fn(f32) -> Vector3<f32>) -> Strip {
        let columns = 3;
        let mut positions = Vec::new();
        let mut tex_coords = Vec::new();

        for row in 0..rows {
            let v = 1.0 - row as f32 / (rows - 1) as f32;
            for column in 0..columns {
                let width_offset = column as f32 - 1.0;
                positions.push(place_row(v) + Vector3::new(width_offset, 0.0, 0.0));
                tex_coords.push([column as f32 / (columns - 1) as f32, v]);
            }
        }

        let members = (0..positions.len() as u32).collect();
        Strip {
            positions,
            tex_coords,
            members,
        }
    }

    fn build_vertical_strip() -> Strip {
        build_strip(10, |v| Vector3::new(0.0, 9.0 * v, 0.0))
    }

    fn tilt_about_z(strip: &mut Strip, angle: f32) {
        let (sin, cos) = angle.sin_cos();
        for p in &mut strip.positions {
            *p = Vector3::new(p.x * cos - p.y * sin, p.x * sin + p.y * cos, p.z);
        }
    }

    fn angle_between_degrees(a: Vector3<f32>, b: Vector3<f32>) -> f32 {
        (a.dot(b) / (a.magnitude() * b.magnitude()))
            .clamp(-1.0, 1.0)
            .acos()
            .to_degrees()
    }

    fn next_lcg(state: &mut u64) -> f32 {
        *state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (*state >> 40) as f32 / (1u64 << 24) as f32
    }

    #[test]
    fn vertical_strip_centerline_runs_top_to_bottom_on_axis() {
        let strip = build_vertical_strip();
        let centerline = strip.centerline(strip.top(), KEEP_ALL_BAND);

        assert_eq!(centerline.len(), CENTERLINE_SAMPLE_COUNT);
        assert!(centerline.windows(2).all(|pair| pair[1].y < pair[0].y));
        assert!(centerline
            .iter()
            .all(|p| p.x.abs() < 1e-4 && p.z.abs() < 1e-4));
    }

    #[test]
    fn tilted_strip_centerline_follows_tilt() {
        let mut strip = build_vertical_strip();
        tilt_about_z(&mut strip, 60.0_f32.to_radians());
        let centerline = strip.centerline(strip.top(), KEEP_ALL_BAND);

        let direction = centerline[CENTERLINE_SAMPLE_COUNT - 1] - centerline[0];
        let tilt = angle_between_degrees(direction, -world_y_axis());

        assert!((tilt - 60.0).abs() <= 2.0, "tilt {tilt}");
    }

    #[test]
    fn quarter_circle_strip_centerline_turns_ninety_degrees() {
        let radius = 5.0;
        let strip = build_strip(30, |v| {
            let angle = v * std::f32::consts::FRAC_PI_2;
            Vector3::new(radius * angle.cos(), radius * angle.sin(), 0.0)
        });
        let centerline = strip.centerline(strip.top(), KEEP_ALL_BAND);

        let total_turn: f32 = centerline
            .windows(3)
            .map(|triple| angle_between_degrees(triple[1] - triple[0], triple[2] - triple[1]))
            .sum();

        assert!((total_turn - 90.0).abs() <= 10.0, "total turn {total_turn}");
    }

    #[test]
    fn flipped_v_still_roots_at_higher_end() {
        let mut strip = build_vertical_strip();
        for tex_coord in &mut strip.tex_coords {
            tex_coord[1] = 1.0 - tex_coord[1];
        }
        let centerline = strip.centerline(strip.top(), KEEP_ALL_BAND);

        assert!((centerline[0].y - 9.0).abs() < 1e-4);
        assert!(centerline[CENTERLINE_SAMPLE_COUNT - 1].y.abs() < 1e-4);
    }

    #[test]
    fn rows_above_band_are_dropped() {
        let strip = build_vertical_strip();
        let root_position = strip.top();
        let band_progress = 2.5;
        let centerline = strip.centerline(root_position, band_progress);

        assert!((centerline[0].y - 6.0).abs() < 1e-4);
        assert!(centerline
            .iter()
            .all(|&p| progress_of(p, root_position, world_y_axis()) > band_progress));
    }

    #[test]
    fn vertical_strip_v_correlates_with_strand_axis() {
        let strip = build_vertical_strip();
        let r = uv_strand_correlation(&strip.positions, &strip.tex_coords, &strip.members)
            .expect("correlation exists");

        assert!(r.abs() > 0.99, "r {r}");
    }

    #[test]
    fn random_v_does_not_correlate_with_strand_axis() {
        let mut strip = build_vertical_strip();
        let mut state = 42;
        for tex_coord in &mut strip.tex_coords {
            tex_coord[1] = next_lcg(&mut state);
        }
        let r = uv_strand_correlation(&strip.positions, &strip.tex_coords, &strip.members)
            .expect("correlation exists");

        assert!(r.abs() < 0.5, "r {r}");
    }
}
