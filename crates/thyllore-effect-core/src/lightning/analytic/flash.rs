use crate::lightning::analytic::strike::{compute_lightning_segment_aabb, segment_length, Segment};
use crate::LightningEffect;
use cgmath::{Matrix4, SquareMatrix, Vector4};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LightningFlash {
    pub intensity: f32,
    pub center_uv: [f32; 2],
    pub radius_uv: f32,
}

/// Total emission of the alive segments: the sum of `intensity * length` over every segment.
pub fn compute_total_segment_emission(segments: &[Segment]) -> f32 {
    segments
        .iter()
        .map(|seg| seg.intensity * segment_length(seg))
        .sum()
}

/// Total emission of the alive segments multiplied by `flash_gain`.
pub fn compute_flash_intensity(effect: &LightningEffect, segments: &[Segment]) -> f32 {
    compute_total_segment_emission(segments) * effect.look.flash_gain
}

/// Flash veil centred on the screen box of the alive rim capsules, `flash_radius` box half-extents wide.
pub fn compute_lightning_flash(
    effect: &LightningEffect,
    segments: &[Segment],
    model: &Matrix4<f32>,
    inv_view_proj: Matrix4<f32>,
) -> LightningFlash {
    let intensity = compute_flash_intensity(effect, segments);
    if intensity <= 0.0 {
        return LightningFlash::default();
    }
    let Some(view_proj) = inv_view_proj.invert() else {
        return LightningFlash::default();
    };
    let Some(corners) = compute_lightning_segment_aabb(effect, effect.time) else {
        return LightningFlash::default();
    };

    let model_view_proj = view_proj * model;
    let mut uv_bounds: Option<([f32; 2], [f32; 2])> = None;
    for corner in corners {
        let clip = model_view_proj * Vector4::new(corner.x, corner.y, corner.z, 1.0);
        if clip.w <= 0.0 {
            continue;
        }
        let uv = [clip.x / clip.w * 0.5 + 0.5, clip.y / clip.w * 0.5 + 0.5];
        uv_bounds = Some(match uv_bounds {
            Some((low, high)) => (
                [low[0].min(uv[0]), low[1].min(uv[1])],
                [high[0].max(uv[0]), high[1].max(uv[1])],
            ),
            None => (uv, uv),
        });
    }
    let Some((uv_min, uv_max)) = uv_bounds else {
        return LightningFlash::default();
    };

    let half_extent = ((uv_max[0] - uv_min[0]).max(uv_max[1] - uv_min[1])) * 0.5;
    LightningFlash {
        intensity,
        center_uv: [(uv_min[0] + uv_max[0]) * 0.5, (uv_min[1] + uv_max[1]) * 0.5],
        radius_uv: half_extent * effect.look.flash_radius,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lightning::analytic::{build_lightning_segments, burst_start_time};
    use crate::lightning::build_lightning_model_matrix;
    use crate::LightningShape;

    fn segment(a: [f32; 3], b: [f32; 3], intensity: f32) -> Segment {
        Segment {
            a,
            b,
            r0: 0.1,
            r1: 0.1,
            intensity,
            arrival_start: 0.0,
            arrival_end: 0.0,
        }
    }

    fn make_effect() -> LightningEffect {
        LightningEffect {
            position: cgmath::Vector3::new(0.0, 10.0, 0.0),
            shape: LightningShape {
                end_offset: [0.0, -10.0, 0.0],
                core_radius: 0.5,
                tip_radius_ratio: 0.2,
                edge_fraction: 0.3,
                ..LightningShape::default()
            },
            ..LightningEffect::default()
        }
    }

    #[test]
    fn test_total_emission_zero_segments() {
        assert_eq!(compute_total_segment_emission(&[]), 0.0);
    }

    #[test]
    fn test_total_emission_one_segment() {
        let segments = [segment([0.0, 0.0, 0.0], [0.0, 3.0, 4.0], 2.0)];

        let emission = compute_total_segment_emission(&segments);

        assert!((emission - 10.0).abs() < 1e-6, "emission {}", emission);
    }

    #[test]
    fn test_flash_intensity_linear_in_gain() {
        let segments = [
            segment([0.0, 0.0, 0.0], [0.0, 3.0, 4.0], 2.0),
            segment([1.0, 0.0, 0.0], [1.0, 0.0, 2.0], 0.5),
        ];
        let mut effect = make_effect();

        effect.look.flash_gain = 1.0;
        let unit = compute_flash_intensity(&effect, &segments);
        effect.look.flash_gain = 2.5;
        let scaled = compute_flash_intensity(&effect, &segments);
        effect.look.flash_gain = 0.0;
        let disabled = compute_flash_intensity(&effect, &segments);

        assert!((unit - 11.0).abs() < 1e-5, "unit gain {}", unit);
        assert!((scaled - 2.5 * unit).abs() < 1e-5, "scaled {}", scaled);
        assert_eq!(disabled, 0.0);
    }

    #[test]
    fn test_flash_disabled_by_default_gain() {
        let mut effect = make_effect();
        effect.time = burst_start_time(&effect, 0) + effect.timing.attack_time;
        let segments = build_lightning_segments(&effect, effect.time);
        let model = build_lightning_model_matrix(&effect);

        let flash = compute_lightning_flash(&effect, &segments, &model, Matrix4::identity());

        assert!(!segments.is_empty(), "stroke should be alive");
        assert_eq!(flash, LightningFlash::default());
    }

    #[test]
    fn test_flash_centered_on_projected_box() {
        let mut effect = make_effect();
        effect.position = cgmath::Vector3::new(0.0, 0.0, 0.0);
        effect.look.flash_gain = 1.0;
        effect.time = burst_start_time(&effect, 0) + effect.timing.attack_time;
        let segments = build_lightning_segments(&effect, effect.time);
        let model = build_lightning_model_matrix(&effect);
        let corners = compute_lightning_segment_aabb(&effect, effect.time).expect("alive segments");
        let (low, high) = (corners[0], corners[7]);

        let flash = compute_lightning_flash(&effect, &segments, &model, Matrix4::identity());
        effect.look.flash_radius *= 2.0;
        let wider = compute_lightning_flash(&effect, &segments, &model, Matrix4::identity());

        let expected_center = [(low.x + high.x) * 0.25 + 0.5, (low.y + high.y) * 0.25 + 0.5];
        assert!(flash.intensity > 0.0);
        assert!((flash.center_uv[0] - expected_center[0]).abs() < 1e-4);
        assert!((flash.center_uv[1] - expected_center[1]).abs() < 1e-4);
        assert!(flash.radius_uv > 0.0);
        assert!((wider.radius_uv - 2.0 * flash.radius_uv).abs() < 1e-5);
    }
}
