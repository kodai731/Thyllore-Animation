use std::f32::consts::TAU;

use cgmath::{InnerSpace, Vector3};
use thyllore_math_core::{fit_principal_axis, world_y_axis};

use super::chain_gen::{build_azimuth_basis, progress_of, resample_polyline};
use crate::garment::components::chain_gen::{
    joint_name, ChainGenInput, ChainGenParams, GeneratedChain, GeneratedJoint,
    StrandCenterlineRecord, StrandMode,
};

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
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StrandMergeStop {
    Count(usize),
    Distance(f32),
    Never,
}

impl StrandMergeStop {
    pub fn from_settings(column_count: u32, strand_merge_distance: f32) -> Self {
        if column_count > 0 {
            Self::Count(column_count as usize)
        } else if strand_merge_distance > 0.0 {
            Self::Distance(strand_merge_distance)
        } else {
            Self::Never
        }
    }
}

fn centerline_distance(a: &[Vector3<f32>], b: &[Vector3<f32>]) -> f32 {
    let len = a.len().min(b.len());
    a.iter()
        .zip(b)
        .map(|(p, q)| (p - q).magnitude())
        .sum::<f32>()
        / len as f32
}

pub fn merge_strands(centerlines: &[Vec<Vector3<f32>>], stop: StrandMergeStop) -> Vec<Vec<usize>> {
    let n = centerlines.len();
    if n == 0 {
        return vec![];
    }

    let mut clusters: Vec<Vec<usize>> = (0..n).map(|i| vec![i]).collect();

    loop {
        let Some((i, j, distance)) = closest_cluster_pair(&clusters, centerlines) else {
            break;
        };

        let reached_stop = match stop {
            StrandMergeStop::Never => true,
            StrandMergeStop::Count(target) => clusters.len() <= target,
            StrandMergeStop::Distance(threshold) => distance > threshold,
        };
        if reached_stop {
            break;
        }

        let mut merged = clusters[i].clone();
        merged.append(&mut clusters[j]);
        clusters.remove(j);
        clusters[i] = merged;
    }

    for cluster in &mut clusters {
        cluster.sort_unstable();
    }
    clusters.sort_by_key(|c| c[0]);

    clusters
}

fn closest_cluster_pair(
    clusters: &[Vec<usize>],
    centerlines: &[Vec<Vector3<f32>>],
) -> Option<(usize, usize, f32)> {
    if clusters.len() < 2 {
        return None;
    }

    let (mut best_i, mut best_j) = (0, 1);
    let mut best_dist = cluster_distance(&clusters[0], &clusters[1], centerlines);

    for i in 0..clusters.len() {
        for j in (i + 1)..clusters.len() {
            let dist = cluster_distance(&clusters[i], &clusters[j], centerlines);
            if dist < best_dist {
                best_dist = dist;
                best_i = i;
                best_j = j;
            }
        }
    }

    Some((best_i, best_j, best_dist))
}

fn cluster_distance(a: &[usize], b: &[usize], centerlines: &[Vec<Vector3<f32>>]) -> f32 {
    let mut total = 0.0;
    let mut count = 0usize;
    for &ai in a {
        for &bi in b {
            total += centerline_distance(&centerlines[ai], &centerlines[bi]);
            count += 1;
        }
    }
    total / count as f32
}

pub fn tuft_centerline(
    centerlines: &[Vec<Vector3<f32>>],
    weights: &[usize],
    members: &[usize],
) -> Vec<Vector3<f32>> {
    let total_weight: usize = members.iter().map(|&c| weights[c]).sum();
    if total_weight == 0 || centerlines.is_empty() {
        return vec![];
    }

    let sample_count = centerlines[members[0]].len();
    let mut result = Vec::with_capacity(sample_count);

    for k in 0..sample_count {
        let mut sum = Vector3::new(0.0, 0.0, 0.0);
        for &c in members {
            sum += centerlines[c][k] * weights[c] as f32;
        }
        result.push(sum / total_weight as f32);
    }

    result
}

pub fn decide_strand_mode(
    input: &ChainGenInput,
    components: &[Vec<u32>],
    band_progress: f32,
) -> StrandMode {
    if input.tex_coords.is_empty() {
        return StrandMode::Azimuth;
    }

    let up = world_y_axis();
    let mut total_below = 0;
    let mut correlated_below = 0;
    for members in components {
        let below_count = members
            .iter()
            .filter(|&&v| {
                progress_of(input.positions[v as usize], input.root_position, up) > band_progress
            })
            .count();
        total_below += below_count;

        let is_correlated = uv_strand_correlation(input.positions, input.tex_coords, members)
            .is_some_and(|r| r.abs() >= MIN_STRAND_UV_CORRELATION);
        if is_correlated {
            correlated_below += below_count;
        }
    }

    if correlated_below * 2 > total_below {
        StrandMode::Card
    } else {
        StrandMode::Azimuth
    }
}

struct CardStrand {
    component: u32,
    centerline: Vec<Vector3<f32>>,
    below_band: Vec<u32>,
}

impl CardStrand {
    fn centerline_record(&self, chain: usize) -> StrandCenterlineRecord {
        StrandCenterlineRecord {
            component: self.component,
            chain,
            points: self.centerline.iter().map(|p| [p.x, p.y, p.z]).collect(),
        }
    }
}

fn build_card_strand(
    input: &ChainGenInput,
    members: &[u32],
    band_progress: f32,
) -> Option<CardStrand> {
    let component = members.iter().copied().min()?;
    let correlation = uv_strand_correlation(input.positions, input.tex_coords, members)?;
    if correlation.abs() < MIN_STRAND_UV_CORRELATION {
        return None;
    }
    let centerline = strand_centerline(
        input.positions,
        input.tex_coords,
        members,
        input.root_position,
        band_progress,
    )?;

    let up = world_y_axis();
    let below_band = members
        .iter()
        .copied()
        .filter(|&v| {
            progress_of(input.positions[v as usize], input.root_position, up) > band_progress
        })
        .collect();

    Some(CardStrand {
        component,
        centerline,
        below_band,
    })
}

fn build_tuft_chain(
    column: usize,
    strands: &[CardStrand],
    tuft: &[usize],
    centerline: Vec<Vector3<f32>>,
    params: &ChainGenParams,
) -> GeneratedChain {
    let joints: Vec<GeneratedJoint> =
        resample_polyline(&centerline, params.settings.joint_count as usize)
            .into_iter()
            .enumerate()
            .map(|(joint, head)| GeneratedJoint {
                name: joint_name(params.role, column, joint),
                head,
            })
            .collect();
    let mut vertices: Vec<u32> = tuft
        .iter()
        .flat_map(|&strand| strands[strand].below_band.iter().copied())
        .collect();
    vertices.sort_unstable();

    let up = world_y_axis();
    let tail = match tuft
        .iter()
        .filter_map(|&strand| strands[strand].centerline.last().copied())
        .min_by(|a, b| a.dot(up).total_cmp(&b.dot(up)))
    {
        Some(t) => t,
        None => centerline[centerline.len() - 1],
    };

    GeneratedChain {
        column,
        joints,
        tail,
        vertices,
    }
}

pub(crate) fn place_card_chains(
    input: &ChainGenInput,
    components: &[Vec<u32>],
    band_progress: f32,
    azimuth_center: Vector3<f32>,
    params: &ChainGenParams,
) -> (Vec<GeneratedChain>, Vec<StrandCenterlineRecord>) {
    let strands: Vec<CardStrand> = components
        .iter()
        .filter_map(|members| build_card_strand(input, members, band_progress))
        .collect();
    let centerlines: Vec<Vec<Vector3<f32>>> = strands
        .iter()
        .map(|strand| strand.centerline.clone())
        .collect();
    let weights: Vec<usize> = strands
        .iter()
        .map(|strand| strand.below_band.len())
        .collect();
    let stop = StrandMergeStop::from_settings(
        params.settings.column_count,
        params.settings.strand_merge_distance,
    );
    let tufts = merge_strands(&centerlines, stop);

    let (reference, side) = build_azimuth_basis(world_y_axis());
    let mut tuft_centerlines: Vec<(f32, Vec<usize>, Vec<Vector3<f32>>)> = tufts
        .into_iter()
        .map(|tuft| {
            let centerline = tuft_centerline(&centerlines, &weights, &tuft);
            let offset = centerline[0] - azimuth_center;
            let azimuth = offset
                .dot(side)
                .atan2(offset.dot(reference))
                .rem_euclid(TAU);
            (azimuth, tuft, centerline)
        })
        .collect();
    tuft_centerlines.sort_by(|a, b| a.0.total_cmp(&b.0));

    let mut records: Vec<StrandCenterlineRecord> = Vec::new();
    let chains = tuft_centerlines
        .into_iter()
        .enumerate()
        .map(|(column, (_, tuft, centerline))| {
            records.extend(tuft.iter().map(|&s| strands[s].centerline_record(column)));
            build_tuft_chain(column, &strands, &tuft, centerline, params)
        })
        .collect();
    records.sort_by_key(|record| record.component);
    (chains, records)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::garment::components::chain_gen::{ChainGenSettings, ChainRole};
    use crate::garment::systems::chain_gen::{
        hair_root_band, place_chains, place_chains_with_strands,
    };

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

    fn make_vertical_centerline(offset_x: f32) -> Vec<Vector3<f32>> {
        (0..CENTERLINE_SAMPLE_COUNT)
            .map(|i| {
                let t = i as f32 / (CENTERLINE_SAMPLE_COUNT - 1) as f32;
                Vector3::new(offset_x, 9.0 * (1.0 - t), 0.0)
            })
            .collect()
    }

    #[test]
    fn merge_strands_distance_two_clusters() {
        let centerlines = vec![
            make_vertical_centerline(0.0),
            make_vertical_centerline(2.0),
            make_vertical_centerline(4.0),
            make_vertical_centerline(24.0),
        ];
        let clusters = merge_strands(&centerlines, StrandMergeStop::Distance(5.0));

        assert_eq!(clusters.len(), 2);
        assert_eq!(clusters[0], [0, 1, 2]);
        assert_eq!(clusters[1], [3]);
    }

    #[test]
    fn merge_strands_count_three_clusters() {
        let centerlines = vec![
            make_vertical_centerline(0.0),
            make_vertical_centerline(2.0),
            make_vertical_centerline(4.0),
            make_vertical_centerline(24.0),
        ];
        let clusters = merge_strands(&centerlines, StrandMergeStop::Count(3));

        assert_eq!(clusters.len(), 3);
        assert_eq!(clusters[0], [0, 1]);
        assert_eq!(clusters[1], [2]);
        assert_eq!(clusters[2], [3]);
    }

    #[test]
    fn merge_strands_never() {
        let centerlines = vec![
            make_vertical_centerline(0.0),
            make_vertical_centerline(2.0),
            make_vertical_centerline(4.0),
            make_vertical_centerline(24.0),
        ];
        let clusters = merge_strands(&centerlines, StrandMergeStop::Never);

        assert_eq!(clusters.len(), 4);
        assert_eq!(clusters[0], [0]);
        assert_eq!(clusters[1], [1]);
        assert_eq!(clusters[2], [2]);
        assert_eq!(clusters[3], [3]);
    }

    #[test]
    fn merge_strands_count_ten_no_merge() {
        let centerlines = vec![
            make_vertical_centerline(0.0),
            make_vertical_centerline(2.0),
            make_vertical_centerline(4.0),
            make_vertical_centerline(24.0),
        ];
        let clusters = merge_strands(&centerlines, StrandMergeStop::Count(10));

        assert_eq!(clusters.len(), 4);
        assert_eq!(clusters[0], [0]);
        assert_eq!(clusters[1], [1]);
        assert_eq!(clusters[2], [2]);
        assert_eq!(clusters[3], [3]);
    }

    #[test]
    fn tuft_centerline_weighted_average() {
        let centerlines = vec![make_vertical_centerline(0.0), make_vertical_centerline(4.0)];
        let weights = [1, 3];
        let members = [0, 1];
        let tuft = tuft_centerline(&centerlines, &weights, &members);

        assert_eq!(tuft.len(), CENTERLINE_SAMPLE_COUNT);
        for sample in &tuft {
            assert!(
                (sample.x - 3.0).abs() < 1e-4,
                "expected x=3.0 (1:3 weighted average of 0 and 4), got {}",
                sample.x
            );
        }
    }

    #[test]
    fn merge_strands_all_close_becomes_one_cluster() {
        let centerlines = vec![
            make_vertical_centerline(0.0),
            make_vertical_centerline(1.0),
            make_vertical_centerline(2.0),
        ];
        let clusters = merge_strands(&centerlines, StrandMergeStop::Distance(5.0));

        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0], [0, 1, 2]);
    }

    #[test]
    fn merge_strands_single_centerline() {
        let centerlines = vec![make_vertical_centerline(0.0)];
        let clusters = merge_strands(&centerlines, StrandMergeStop::Distance(5.0));

        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0], [0]);
    }

    struct StripSet {
        positions: Vec<Vector3<f32>>,
        tex_coords: Vec<[f32; 2]>,
        components: Vec<Vec<u32>>,
    }

    fn build_three_vertical_strips() -> StripSet {
        let mut strip_set = StripSet {
            positions: Vec::new(),
            tex_coords: Vec::new(),
            components: Vec::new(),
        };
        for index in 0..3 {
            let strip = build_vertical_strip();
            let first_vertex = strip_set.positions.len() as u32;
            let offset = Vector3::new(5.0 * index as f32, 0.0, 0.0);

            strip_set
                .components
                .push(strip.members.iter().map(|&m| m + first_vertex).collect());
            strip_set
                .positions
                .extend(strip.positions.iter().map(|&p| p + offset));
            strip_set.tex_coords.extend(strip.tex_coords);
        }
        strip_set
    }

    fn decide_mode_for(strip_set: &StripSet, tex_coords: &[[f32; 2]]) -> StrandMode {
        let input = ChainGenInput {
            positions: &strip_set.positions,
            triangles: &[],
            root_position: Vector3::new(0.0, 9.0, 0.0),
            attachment_positions: &[],
            tex_coords,
        };
        decide_strand_mode(&input, &strip_set.components, KEEP_ALL_BAND)
    }

    #[test]
    fn vertical_strips_with_v_along_strand_are_card() {
        let strip_set = build_three_vertical_strips();

        assert_eq!(
            decide_mode_for(&strip_set, &strip_set.tex_coords),
            StrandMode::Card
        );
    }

    #[test]
    fn missing_tex_coords_fall_back_to_azimuth() {
        let strip_set = build_three_vertical_strips();

        assert_eq!(decide_mode_for(&strip_set, &[]), StrandMode::Azimuth);
    }

    #[test]
    fn random_v_strips_are_azimuth() {
        let mut strip_set = build_three_vertical_strips();
        let mut state = 42;
        for tex_coord in &mut strip_set.tex_coords {
            tex_coord[1] = next_lcg(&mut state);
        }

        assert_eq!(
            decide_mode_for(&strip_set, &strip_set.tex_coords),
            StrandMode::Azimuth
        );
    }

    const CARD_ROWS: usize = 10;
    const CARD_COLUMNS: usize = 3;
    const CARD_LENGTH: f32 = 6.0;
    const CARD_COLUMN_SPACING: f32 = 0.1;
    const CARD_TILT_DEGREES: f32 = 40.0;

    struct CardPlacement {
        azimuth_degrees: f32,
        side_shift: f32,
    }

    struct CardMesh {
        positions: Vec<Vector3<f32>>,
        triangles: Vec<[u32; 3]>,
        tex_coords: Vec<[f32; 2]>,
        attachment_positions: Vec<Vector3<f32>>,
        strip_axes: Vec<(Vector3<f32>, Vector3<f32>)>,
        strip_vertices: Vec<Vec<u32>>,
    }

    fn card_root() -> Vector3<f32> {
        Vector3::new(0.0, 10.0, 0.0)
    }

    fn build_head_ring() -> Vec<Vector3<f32>> {
        (0..16)
            .map(|index| {
                let angle = index as f32 / 16.0 * TAU;
                Vector3::new(angle.cos(), 9.0, angle.sin())
            })
            .collect()
    }

    fn add_card_strip(mesh: &mut CardMesh, placement: &CardPlacement, length: f32) {
        let azimuth = placement.azimuth_degrees.to_radians();
        let outward = Vector3::new(azimuth.cos(), 0.0, azimuth.sin());
        let across = world_y_axis().cross(outward);
        let tilt = CARD_TILT_DEGREES.to_radians();
        let down = outward * tilt.sin() - world_y_axis() * tilt.cos();
        let top = Vector3::new(0.0, 8.5, 0.0) + outward + across * placement.side_shift;
        let first_vertex = mesh.positions.len() as u32;

        for row in 0..CARD_ROWS {
            let v = 1.0 - row as f32 / (CARD_ROWS - 1) as f32;
            let row_center = top + down * (length * (1.0 - v));
            for column in 0..CARD_COLUMNS {
                let width_offset = (column as f32 - 1.0) * CARD_COLUMN_SPACING;
                mesh.positions.push(row_center + across * width_offset);
                mesh.tex_coords
                    .push([column as f32 / (CARD_COLUMNS - 1) as f32, v]);
            }
        }

        for row in 0..CARD_ROWS - 1 {
            for column in 0..CARD_COLUMNS - 1 {
                let upper = first_vertex + (row * CARD_COLUMNS + column) as u32;
                let lower = upper + CARD_COLUMNS as u32;
                mesh.triangles.push([upper, upper + 1, lower]);
                mesh.triangles.push([upper + 1, lower + 1, lower]);
            }
        }

        mesh.strip_axes.push((top, top + down * length));
        mesh.strip_vertices
            .push((first_vertex..mesh.positions.len() as u32).collect());
    }

    fn build_card_mesh(placements: &[CardPlacement]) -> CardMesh {
        let mut mesh = CardMesh {
            positions: Vec::new(),
            triangles: Vec::new(),
            tex_coords: Vec::new(),
            attachment_positions: build_head_ring(),
            strip_axes: Vec::new(),
            strip_vertices: Vec::new(),
        };
        for placement in placements {
            add_card_strip(&mut mesh, placement, CARD_LENGTH);
        }
        mesh
    }

    fn build_three_tilted_cards() -> CardMesh {
        build_card_mesh(&[0.0, 120.0, 240.0].map(|azimuth_degrees| CardPlacement {
            azimuth_degrees,
            side_shift: 0.0,
        }))
    }

    fn card_input<'a>(mesh: &'a CardMesh, tex_coords: &'a [[f32; 2]]) -> ChainGenInput<'a> {
        ChainGenInput {
            positions: &mesh.positions,
            triangles: &mesh.triangles,
            root_position: card_root(),
            attachment_positions: &mesh.attachment_positions,
            tex_coords,
        }
    }

    fn hair_params(column_count: u32, strand_merge_distance: f32) -> ChainGenParams {
        ChainGenParams {
            role: ChainRole::Hair,
            settings: ChainGenSettings {
                column_count,
                strand_merge_distance,
                ..ChainGenSettings::default()
            },
        }
    }

    fn place_card_mesh_chains(mesh: &CardMesh, params: &ChainGenParams) -> Vec<GeneratedChain> {
        place_chains(&card_input(mesh, &mesh.tex_coords), params).expect("chains are placed")
    }

    fn decide_card_mesh_mode(mesh: &CardMesh, tex_coords: &[[f32; 2]]) -> StrandMode {
        let input = card_input(mesh, tex_coords);
        let band = hair_root_band(&input, &hair_params(0, 0.0)).expect("hair has a root band");
        decide_strand_mode(&input, &mesh.strip_vertices, band.progress)
    }

    fn distance_to_segment(point: Vector3<f32>, (start, end): (Vector3<f32>, Vector3<f32>)) -> f32 {
        let along = end - start;
        let t = ((point - start).dot(along) / along.magnitude2()).clamp(0.0, 1.0);
        (point - (start + along * t)).magnitude()
    }

    #[test]
    fn three_cards_become_three_chains_along_their_centerlines() {
        let mesh = build_three_tilted_cards();
        let placement =
            place_chains_with_strands(&card_input(&mesh, &mesh.tex_coords), &hair_params(0, 0.0))
                .expect("chains are placed");
        let chains = placement.chains;
        let strands = placement.strands.expect("hair has a strand placement");
        let records = strands.centerlines;

        assert_eq!(
            decide_card_mesh_mode(&mesh, &mesh.tex_coords),
            StrandMode::Card
        );
        assert_eq!(chains.len(), 3);
        assert_eq!(strands.mode, StrandMode::Card);
        assert_eq!(records.len(), 3);

        for record in &records {
            let chain = &chains[record.chain];
            for (i, strip_vertices) in mesh.strip_vertices.iter().enumerate() {
                if strip_vertices[0] == record.component {
                    assert!(
                        strip_vertices.iter().all(|v| chain.vertices.contains(v)),
                        "chain {} should contain all vertices of strip {}",
                        chain.column,
                        i
                    );
                }
            }
        }

        for (strip_vertices, &axis) in mesh.strip_vertices.iter().zip(&mesh.strip_axes) {
            let chain = chains
                .iter()
                .find(|chain| chain.vertices.contains(&strip_vertices[0]))
                .expect("strip belongs to a chain");
            assert!(strip_vertices.iter().all(|v| chain.vertices.contains(v)));
            for joint in &chain.joints {
                let distance = distance_to_segment(joint.head, axis);
                assert!(distance <= 0.5, "{} is {distance} off", joint.name);
            }
        }
    }

    #[test]
    fn adjacent_cards_merge_within_strand_merge_distance() {
        let mesh = build_card_mesh(&[
            CardPlacement {
                azimuth_degrees: 0.0,
                side_shift: 0.0,
            },
            CardPlacement {
                azimuth_degrees: 0.0,
                side_shift: 0.5,
            },
            CardPlacement {
                azimuth_degrees: 120.0,
                side_shift: 0.0,
            },
        ]);
        let chains = place_card_mesh_chains(&mesh, &hair_params(0, 2.0));

        assert_eq!(chains.len(), 2);
    }

    #[test]
    fn column_count_one_merges_every_card() {
        let mesh = build_three_tilted_cards();
        let chains = place_card_mesh_chains(&mesh, &hair_params(1, 0.0));

        assert_eq!(chains.len(), 1);
    }

    #[test]
    fn cards_without_tex_coords_use_the_azimuth_path() {
        let mesh = build_three_tilted_cards();
        let placement = place_chains_with_strands(&card_input(&mesh, &[]), &hair_params(0, 0.0))
            .expect("chains are placed");
        let strands = placement.strands.expect("hair has a strand placement");

        assert_eq!(decide_card_mesh_mode(&mesh, &[]), StrandMode::Azimuth);
        assert_eq!(strands.mode, StrandMode::Azimuth);
        assert!(strands.centerlines.is_empty());
        assert_eq!(placement.chains.len(), 3);
    }

    #[test]
    fn skirt_role_has_no_strand_placement() {
        let mesh = build_three_tilted_cards();
        let params = ChainGenParams {
            role: ChainRole::Skirt,
            settings: ChainGenSettings::default(),
        };
        let placement = place_chains_with_strands(&card_input(&mesh, &mesh.tex_coords), &params)
            .expect("chains are placed");

        assert!(placement.strands.is_none());
    }

    #[test]
    fn merged_tuft_tail_reaches_longest_strand_tip() {
        let mut mesh = CardMesh {
            positions: Vec::new(),
            triangles: Vec::new(),
            tex_coords: Vec::new(),
            attachment_positions: build_head_ring(),
            strip_axes: Vec::new(),
            strip_vertices: Vec::new(),
        };
        add_card_strip(
            &mut mesh,
            &CardPlacement {
                azimuth_degrees: 0.0,
                side_shift: 0.0,
            },
            3.0,
        );
        add_card_strip(
            &mut mesh,
            &CardPlacement {
                azimuth_degrees: 0.0,
                side_shift: 0.5,
            },
            6.0,
        );

        let chains = place_card_mesh_chains(&mesh, &hair_params(0, 2.0));
        assert_eq!(
            chains.len(),
            1,
            "two adjacent strips should merge into one chain"
        );

        let chain = &chains[0];
        let shorter_tip_y = 8.5 - 3.0 * CARD_TILT_DEGREES.to_radians().cos();
        let longer_tip_y = 8.5 - 6.0 * CARD_TILT_DEGREES.to_radians().cos();

        let distance_to_longer = (chain.tail.y - longer_tip_y).abs();
        let distance_to_shorter = (chain.tail.y - shorter_tip_y).abs();
        assert!(
            distance_to_longer < distance_to_shorter,
            "tail y={} should be closer to longer tip y={:.1} (dist {:.2}) than shorter tip y={:.1} (dist {:.2})",
            chain.tail.y,
            longer_tip_y,
            distance_to_longer,
            shorter_tip_y,
            distance_to_shorter,
        );
    }
}
