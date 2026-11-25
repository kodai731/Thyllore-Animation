use std::f32::consts::TAU;

/// A direction on the circle in radians with the weight it carries.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeightedAngle {
    pub angle: f32,
    pub weight: f32,
}

pub fn angular_distance(a: f32, b: f32) -> f32 {
    let difference = (a - b).rem_euclid(TAU);
    difference.min(TAU - difference)
}

/// Weighted circular mean and the mean resultant length (1 = one direction, 0 = spread over the circle).
pub fn circular_mean_and_concentration(
    angles: impl Iterator<Item = WeightedAngle>,
) -> Option<(f32, f32)> {
    let (sin_sum, cos_sum, weight_sum) =
        angles.fold((0.0f32, 0.0f32, 0.0f32), |(s, c, n), sample| {
            (
                s + sample.angle.sin() * sample.weight,
                c + sample.angle.cos() * sample.weight,
                n + sample.weight,
            )
        });
    (weight_sum > 0.0).then(|| (sin_sum.atan2(cos_sum), sin_sum.hypot(cos_sum) / weight_sum))
}

pub fn circular_mean(angles: impl Iterator<Item = WeightedAngle>) -> Option<f32> {
    circular_mean_and_concentration(angles).map(|(mean, _)| mean)
}

/// The bin of `angle` when the circle is cut into `bin_count` equal bins starting at angle 0.
pub fn angle_bin(angle: f32, bin_count: usize) -> usize {
    let normalized = angle.rem_euclid(TAU) / TAU;
    ((normalized * bin_count as f32) as usize).min(bin_count - 1)
}

/// Weighted histogram over the circle, each bin averaged with its two neighbours.
pub fn smoothed_circular_histogram(samples: &[WeightedAngle], bin_count: usize) -> Vec<f32> {
    let mut histogram = vec![0.0f32; bin_count];
    for sample in samples {
        histogram[angle_bin(sample.angle, bin_count)] += sample.weight;
    }
    (0..bin_count)
        .map(|bin| {
            let left = histogram[(bin + bin_count - 1) % bin_count];
            let right = histogram[(bin + 1) % bin_count];
            (left + histogram[bin] + right) / 3.0
        })
        .collect()
}

/// Local maxima of a circular histogram, at least one.
pub fn count_circular_modes(histogram: &[f32]) -> usize {
    let bin_count = histogram.len();
    let mode_count = (0..bin_count)
        .filter(|&bin| {
            let left = histogram[(bin + bin_count - 1) % bin_count];
            let right = histogram[(bin + 1) % bin_count];
            histogram[bin] > 0.0 && histogram[bin] > left && histogram[bin] >= right
        })
        .count();
    mode_count.max(1)
}

/// Start and angular extent of the occupied arc: the full circle when no bin is empty, otherwise the
/// complement of the widest empty gap.
pub fn occupied_arc(histogram: &[f32]) -> (f32, f32) {
    let bin_count = histogram.len();
    let bin_width = TAU / bin_count as f32;
    let empty = |bin: usize| histogram[bin % bin_count] <= 0.0;
    let Some(first_filled) = (0..bin_count).find(|&bin| !empty(bin)) else {
        return (0.0, TAU);
    };

    let mut widest_gap: Option<(usize, usize)> = None;
    let mut bin = first_filled;
    while bin < first_filled + bin_count {
        if !empty(bin) {
            bin += 1;
            continue;
        }
        let gap_start = bin;
        while bin < first_filled + bin_count && empty(bin) {
            bin += 1;
        }
        let gap_length = bin - gap_start;
        if widest_gap.is_none_or(|(_, length)| gap_length > length) {
            widest_gap = Some((gap_start, gap_length));
        }
    }

    match widest_gap {
        None => (0.0, TAU),
        Some((gap_start, gap_length)) => (
            (gap_start + gap_length) as f32 * bin_width,
            (bin_count - gap_length) as f32 * bin_width,
        ),
    }
}

fn nearest_center(angle: f32, centers: &[f32]) -> usize {
    centers
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            angular_distance(angle, **a).total_cmp(&angular_distance(angle, **b))
        })
        .map(|(index, _)| index)
        .unwrap_or(0)
}

/// Circular k-means: the cluster index of every sample, with `cluster_count` centers seeded evenly
/// over the occupied arc of the histogram.
pub fn cluster_angles(
    samples: &[WeightedAngle],
    cluster_count: usize,
    histogram_bin_count: usize,
    iteration_count: usize,
) -> Vec<usize> {
    let histogram = smoothed_circular_histogram(samples, histogram_bin_count);
    let (arc_start, arc_extent) = occupied_arc(&histogram);
    let mut centers: Vec<f32> = (0..cluster_count)
        .map(|index| arc_start + arc_extent * (index as f32 + 0.5) / cluster_count as f32)
        .collect();
    let mut clusters = vec![0usize; samples.len()];

    for _ in 0..iteration_count {
        for (cluster, sample) in clusters.iter_mut().zip(samples) {
            *cluster = nearest_center(sample.angle, &centers);
        }

        for (center_index, center) in centers.iter_mut().enumerate() {
            let members = samples
                .iter()
                .zip(&clusters)
                .filter(|(_, &cluster)| cluster == center_index)
                .map(|(sample, _)| *sample);
            if let Some(mean) = circular_mean(members) {
                *center = mean;
            }
        }
    }

    clusters
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    fn unit(angle: f32) -> WeightedAngle {
        WeightedAngle { angle, weight: 1.0 }
    }

    #[test]
    fn angular_distance_wraps_around() {
        assert!((angular_distance(0.1, TAU - 0.1) - 0.2).abs() < 1e-6);
        assert!((angular_distance(PI, 0.0) - PI).abs() < 1e-6);
    }

    #[test]
    fn mean_of_opposite_directions_has_no_concentration() {
        let (_, concentration) =
            circular_mean_and_concentration([unit(0.0), unit(PI)].into_iter()).unwrap();
        assert!(concentration < 1e-6);
        assert!(circular_mean(std::iter::empty()).is_none());
    }

    #[test]
    fn mean_across_the_seam_is_the_seam() {
        let mean = circular_mean([unit(0.1), unit(TAU - 0.1)].into_iter()).unwrap();
        assert!(angular_distance(mean, 0.0) < 1e-5);
    }

    #[test]
    fn occupied_arc_excludes_the_widest_gap() {
        let samples: Vec<WeightedAngle> = (0..9).map(|i| unit(i as f32 * TAU / 36.0)).collect();
        let histogram = smoothed_circular_histogram(&samples, 36);
        let (start, extent) = occupied_arc(&histogram);
        let bin_width = TAU / 36.0;
        assert!(angular_distance(start, -bin_width) < 1e-5);
        assert!((extent - 11.0 * bin_width).abs() < 1e-5);
        assert_eq!(count_circular_modes(&histogram), 1);
    }

    #[test]
    fn two_groups_cluster_apart() {
        let samples: Vec<WeightedAngle> = [0.0, 0.1, -0.1, PI, PI + 0.1, PI - 0.1]
            .into_iter()
            .map(unit)
            .collect();
        let clusters = cluster_angles(&samples, 2, 36, 8);
        assert_eq!(clusters[0], clusters[1]);
        assert_eq!(clusters[0], clusters[2]);
        assert_eq!(clusters[3], clusters[4]);
        assert_ne!(clusters[0], clusters[3]);
    }
}
