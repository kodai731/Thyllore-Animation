use cgmath::{InnerSpace, Vector3};

/// When the strand clustering stops joining the two closest centerlines.
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
