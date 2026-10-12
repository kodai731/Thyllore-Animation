use serde::Serialize;

pub const REC709_LUMA_WEIGHTS: [f32; 3] = [0.2126, 0.7152, 0.0722];
const PERCENTILES: [u8; 7] = [1, 10, 40, 50, 90, 99, 100];

/// Distribution of one luminance channel over an image, for comparing views or exposure settings.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct LuminanceStatistics {
    pub pixel_count: usize,
    pub mean: f32,
    /// exp2 of the mean log2 luminance, the quantity an auto exposure histogram averages.
    pub log_mean: f32,
    pub percentiles: Vec<Percentile>,
    /// Share of pixels whose luminance is at or below the threshold given at construction.
    pub dark_fraction: f32,
    pub dark_threshold: f32,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Percentile {
    pub percent: u8,
    pub value: f32,
}

pub fn rec709_luminance(rgb: [f32; 3]) -> f32 {
    rgb[0] * REC709_LUMA_WEIGHTS[0]
        + rgb[1] * REC709_LUMA_WEIGHTS[1]
        + rgb[2] * REC709_LUMA_WEIGHTS[2]
}

pub fn luminance_statistics(
    luminances: &[f32],
    dark_threshold: f32,
) -> Option<LuminanceStatistics> {
    if luminances.is_empty() {
        return None;
    }

    let mut sorted: Vec<f32> = luminances
        .iter()
        .copied()
        .filter(|v| v.is_finite())
        .collect();
    if sorted.is_empty() {
        return None;
    }
    sorted.sort_by(|a, b| a.total_cmp(b));

    let count = sorted.len();
    let mean = sorted.iter().sum::<f32>() / count as f32;
    let log_floor = f32::MIN_POSITIVE;
    let log_mean =
        (sorted.iter().map(|v| v.max(log_floor).log2()).sum::<f32>() / count as f32).exp2();
    let dark_count = sorted.iter().take_while(|v| **v <= dark_threshold).count();

    let percentiles = PERCENTILES
        .iter()
        .map(|&percent| Percentile {
            percent,
            value: sorted[((count - 1) * percent as usize) / 100],
        })
        .collect();

    Some(LuminanceStatistics {
        pixel_count: count,
        mean,
        log_mean,
        percentiles,
        dark_fraction: dark_count as f32 / count as f32,
        dark_threshold,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_statistics_of_a_uniform_image_collapse_to_one_value() {
        let stats = luminance_statistics(&[0.5; 16], 0.1).expect("non-empty");
        assert_eq!(stats.pixel_count, 16);
        assert!((stats.mean - 0.5).abs() < 1e-6);
        assert!((stats.log_mean - 0.5).abs() < 1e-6);
        assert!(stats
            .percentiles
            .iter()
            .all(|p| (p.value - 0.5).abs() < 1e-6));
        assert_eq!(stats.dark_fraction, 0.0);
    }

    #[test]
    fn test_dark_fraction_counts_pixels_at_or_below_the_threshold() {
        let stats = luminance_statistics(&[0.0, 0.05, 0.1, 1.0], 0.1).expect("non-empty");
        assert!((stats.dark_fraction - 0.75).abs() < 1e-6);
        let median = stats.percentiles.iter().find(|p| p.percent == 50).unwrap();
        assert!((median.value - 0.05).abs() < 1e-6);
    }

    #[test]
    fn test_empty_input_has_no_statistics() {
        assert!(luminance_statistics(&[], 0.1).is_none());
    }
}
