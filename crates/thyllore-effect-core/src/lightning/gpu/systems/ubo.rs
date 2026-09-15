use crate::lightning::analytic::build_lightning_segments;
use crate::lightning::gpu::generated::LightningUBO;
use crate::lightning::gpu::generated_segments::LightningSegmentsUBO;
use crate::lightning::{build_lightning_model_matrix, LightningEffect};
use cgmath::{Matrix4, SquareMatrix};

pub fn build_lightning_ubo(
    effect: &LightningEffect,
    inv_view_proj: Matrix4<f32>,
) -> (LightningUBO, LightningSegmentsUBO) {
    let model = build_lightning_model_matrix(effect);
    let inverse_model = model.invert().unwrap_or(Matrix4::identity());

    let segments = build_lightning_segments(effect, effect.time);
    let count = segments.len();

    let mut seg_a_r0: [[f32; 4]; 256] = [[0.0; 4]; 256];
    let mut seg_b_r1: [[f32; 4]; 256] = [[0.0; 4]; 256];
    let mut seg_misc: [[f32; 4]; 256] = [[0.0; 4]; 256];

    for (i, seg) in segments.iter().enumerate() {
        seg_a_r0[i] = [seg.a[0], seg.a[1], seg.a[2], seg.r0];
        seg_b_r1[i] = [seg.b[0], seg.b[1], seg.b[2], seg.r1];
        let edge_width_q = effect.edge_fraction * seg.r0 * seg.r0;
        seg_misc[i] = [edge_width_q, seg.intensity, 0.0, 0.0];
    }

    let ubo = LightningUBO {
        model,
        inverse_model,
        core: [
            effect.core_color[0],
            effect.core_color[1],
            effect.core_color[2],
            effect.core_intensity,
        ],
        glow: [
            effect.glow_color[0],
            effect.glow_color[1],
            effect.glow_color[2],
            effect.glow_intensity,
        ],
        shape: [effect.glow_ratio, effect.edge_fraction, count as f32, 0.0],
        debug_: [0.0, 0.0, 0.0, 0.0],
        inv_view_proj,
    };

    let segments_ubo = LightningSegmentsUBO {
        seg_a_r0,
        seg_b_r1,
        seg_misc,
    };

    (ubo, segments_ubo)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_effect() -> LightningEffect {
        LightningEffect {
            position: cgmath::Vector3::new(0.0, 10.0, 0.0),
            end_offset: [0.0, -10.0, 0.0],
            core_radius: 0.5,
            tip_radius_ratio: 0.2,
            edge_fraction: 0.3,
            ..LightningEffect::default()
        }
    }

    fn sustain_midpoint_time(effect: &LightningEffect) -> f32 {
        crate::lightning::analytic::burst_start_time(effect, 0)
            + effect.attack_time
            + effect.sustain_time * 0.5
    }

    #[test]
    fn test_count_zero_before_burst() {
        let mut effect = make_effect();
        effect.time = crate::lightning::analytic::burst_start_time(&effect, 0) - 1.0;
        let (ubo, segments) = build_lightning_ubo(&effect, Matrix4::identity());

        assert!(
            (ubo.shape[2] - 0.0).abs() < 1e-6,
            "shape.z should be 0 before burst: {}",
            ubo.shape[2]
        );

        for i in 0..256 {
            assert_eq!(
                segments.seg_a_r0[i], [0.0; 4],
                "seg_a_r0[{}] should be zero",
                i
            );
            assert_eq!(
                segments.seg_b_r1[i], [0.0; 4],
                "seg_b_r1[{}] should be zero",
                i
            );
            assert_eq!(
                segments.seg_misc[i], [0.0; 4],
                "seg_misc[{}] should be zero",
                i
            );
        }
    }

    #[test]
    fn test_count_positive_during_sustain() {
        let mut effect = make_effect();
        effect.time = sustain_midpoint_time(&effect);
        let (ubo, segments) = build_lightning_ubo(&effect, Matrix4::identity());

        let count = ubo.shape[2] as usize;
        assert!(count > 0, "shape.z should be > 0 during sustain: {}", count);

        for i in 0..count {
            assert_ne!(
                segments.seg_a_r0[i], [0.0; 4],
                "seg_a_r0[{}] should be active",
                i
            );
        }

        for i in count..256 {
            assert_eq!(
                segments.seg_a_r0[i], [0.0; 4],
                "seg_a_r0[{}] should be zero",
                i
            );
        }

        let (k, tau) = crate::lightning::analytic::active_burst(&effect, effect.time)
            .expect("should have active burst");
        let timing_intensity = crate::lightning::analytic::stroke_intensity(&effect, tau)
            * crate::lightning::analytic::flicker_factor(&effect, effect.seed, k, tau);
        let burst_seed = crate::lightning::hash_u32(&[effect.seed, k]);
        let reseed = crate::lightning::analytic::reseed_index(&effect, tau);
        let base_segments =
            crate::lightning::analytic::build_strike_segments(&effect, burst_seed, reseed);

        for i in 0..count {
            let expected = base_segments[i].intensity * timing_intensity;
            assert!(
                (segments.seg_misc[i][1] - expected).abs() < 1e-5,
                "seg_misc[{}].y intensity mismatch: {} vs {}",
                i,
                segments.seg_misc[i][1],
                expected
            );
        }
    }

    #[test]
    fn test_deterministic() {
        let mut effect = make_effect();
        effect.time = sustain_midpoint_time(&effect);
        let (ubo1, segments1) = build_lightning_ubo(&effect, Matrix4::identity());
        let (ubo2, segments2) = build_lightning_ubo(&effect, Matrix4::identity());

        assert_eq!(ubo1.shape, ubo2.shape, "shape differs");
        for i in 0..256 {
            assert_eq!(
                segments1.seg_a_r0[i], segments2.seg_a_r0[i],
                "seg_a_r0 differs at {}",
                i
            );
            assert_eq!(
                segments1.seg_b_r1[i], segments2.seg_b_r1[i],
                "seg_b_r1 differs at {}",
                i
            );
            assert_eq!(
                segments1.seg_misc[i], segments2.seg_misc[i],
                "seg_misc differs at {}",
                i
            );
        }
    }

    #[test]
    fn test_edge_width_q_formula() {
        let mut effect = make_effect();
        effect.time = sustain_midpoint_time(&effect);
        let (_ubo, segments) = build_lightning_ubo(&effect, Matrix4::identity());

        let count = build_lightning_segments(&effect, effect.time).len();
        for i in 0..count {
            let r0 = segments.seg_a_r0[i][3];
            let expected = effect.edge_fraction * r0 * r0;
            assert!(
                (segments.seg_misc[i][0] - expected).abs() < 1e-6,
                "edge_width_q mismatch at segment {}: {} vs {}",
                i,
                segments.seg_misc[i][0],
                expected
            );
        }
    }

    #[test]
    fn test_shape_packing() {
        let mut effect = make_effect();
        effect.time = sustain_midpoint_time(&effect);
        let (ubo, _) = build_lightning_ubo(&effect, Matrix4::identity());

        assert!(
            (ubo.shape[0] - effect.glow_ratio).abs() < 1e-6,
            "shape.x should be glow_ratio: {} vs {}",
            ubo.shape[0],
            effect.glow_ratio
        );
        assert!(
            (ubo.shape[1] - effect.edge_fraction).abs() < 1e-6,
            "shape.y should be edge_fraction: {} vs {}",
            ubo.shape[1],
            effect.edge_fraction
        );

        let expected_count = build_lightning_segments(&effect, effect.time).len();
        assert!(
            (ubo.shape[2] - expected_count as f32).abs() < 1e-6,
            "shape.z should be segment count {}: {}",
            expected_count,
            ubo.shape[2]
        );
    }
}
