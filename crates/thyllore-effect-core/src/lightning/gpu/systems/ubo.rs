use crate::lightning::gpu::generated::LightningUBO;
use crate::lightning::gpu::generated_segments::LightningSegmentsUBO;
use crate::lightning::{build_lightning_model_matrix, LightningEffect};
use cgmath::{Matrix4, SquareMatrix};

const HORIZONTAL_OFFSETS: [f32; 9] = [0.0, 1.0, -1.0, 2.0, -2.0, 1.5, -1.5, 0.5, 0.0];

pub fn build_lightning_ubo(
    effect: &LightningEffect,
    inv_view_proj: Matrix4<f32>,
) -> (LightningUBO, LightningSegmentsUBO) {
    let model = build_lightning_model_matrix(effect);
    let inverse_model = model.invert().unwrap_or(Matrix4::identity());

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
        shape: [effect.glow_ratio, effect.edge_fraction, 8.0, 0.0],
        debug_: [0.0, 0.0, 0.0, 0.0],
        inv_view_proj,
    };

    let segments = build_segments(effect);

    (ubo, segments)
}

fn build_segments(effect: &LightningEffect) -> LightningSegmentsUBO {
    let start = effect.position;
    let end = [
        start[0] + effect.end_offset[0],
        start[1] + effect.end_offset[1],
        start[2] + effect.end_offset[2],
    ];

    let r0 = effect.core_radius;
    let r1 = effect.core_radius * effect.tip_radius_ratio;

    let mut seg_a_r0: [[f32; 4]; 256] = [[0.0; 4]; 256];
    let mut seg_b_r1: [[f32; 4]; 256] = [[0.0; 4]; 256];
    let mut seg_misc: [[f32; 4]; 256] = [[0.0; 4]; 256];

    for i in 0..8 {
        let t = i as f32 / 8.0;
        let next_t = (i + 1) as f32 / 8.0;

        let px = start[0] * (1.0 - t) + end[0] * t + HORIZONTAL_OFFSETS[i];
        let py = start[1] * (1.0 - t) + end[1] * t;
        let pz = start[2] * (1.0 - t) + end[2] * t + HORIZONTAL_OFFSETS[i];

        let nx = start[0] * (1.0 - next_t) + end[0] * next_t + HORIZONTAL_OFFSETS[i + 1];
        let ny = start[1] * (1.0 - next_t) + end[1] * next_t;
        let nz = start[2] * (1.0 - next_t) + end[2] * next_t + HORIZONTAL_OFFSETS[i + 1];

        let radius_a = r0 * (1.0 - t) + r1 * t;
        let radius_b = r0 * (1.0 - next_t) + r1 * next_t;

        let edge_width_q = effect.edge_fraction * radius_a * radius_a;

        seg_a_r0[i] = [px, py, pz, radius_a];
        seg_b_r1[i] = [nx, ny, nz, radius_b];
        seg_misc[i] = [edge_width_q, 1.0, 0.0, 0.0];
    }

    LightningSegmentsUBO {
        seg_a_r0,
        seg_b_r1,
        seg_misc,
    }
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

    #[test]
    fn test_count_is_8() {
        let effect = make_effect();
        let segments = build_segments(&effect);

        // First 8 entries should be non-zero (active), rest zero
        for i in 0..8 {
            assert_ne!(
                segments.seg_a_r0[i], [0.0; 4],
                "segment {} should be active",
                i
            );
        }
        for i in 8..256 {
            assert_eq!(
                segments.seg_a_r0[i], [0.0; 4],
                "segment {} should be zero",
                i
            );
        }
    }

    #[test]
    fn test_endpoint_match() {
        let effect = make_effect();
        let segments = build_segments(&effect);

        let first = segments.seg_a_r0[0];
        assert!(
            (first[0] - effect.position[0]).abs() < 1e-6,
            "x mismatch: {} vs {}",
            first[0],
            effect.position[0]
        );
        assert!(
            (first[1] - effect.position[1]).abs() < 1e-6,
            "y mismatch: {} vs {}",
            first[1],
            effect.position[1]
        );
        assert!(
            (first[2] - effect.position[2]).abs() < 1e-6,
            "z mismatch: {} vs {}",
            first[2],
            effect.position[2]
        );

        let last = segments.seg_b_r1[7];
        let expected_end_x = effect.position[0] + effect.end_offset[0];
        let expected_end_y = effect.position[1] + effect.end_offset[1];
        let expected_end_z = effect.position[2] + effect.end_offset[2];
        assert!(
            (last[0] - expected_end_x).abs() < 1e-6,
            "end x mismatch: {} vs {}",
            last[0],
            expected_end_x
        );
        assert!(
            (last[1] - expected_end_y).abs() < 1e-6,
            "end y mismatch: {} vs {}",
            last[1],
            expected_end_y
        );
        assert!(
            (last[2] - expected_end_z).abs() < 1e-6,
            "end z mismatch: {} vs {}",
            last[2],
            expected_end_z
        );
    }

    #[test]
    fn test_radius_strictly_decreasing() {
        let effect = make_effect();
        let segments = build_segments(&effect);

        for i in 1..8 {
            let prev_radius = segments.seg_a_r0[i - 1][3];
            let curr_radius = segments.seg_a_r0[i][3];
            assert!(
                curr_radius < prev_radius - 1e-6,
                "radius not strictly decreasing: seg[{}] radius {} >= seg[{}] radius {}",
                i,
                curr_radius,
                i - 1,
                prev_radius
            );
        }
    }

    #[test]
    fn test_edge_width_q_formula() {
        let effect = make_effect();
        let segments = build_segments(&effect);

        let r0 = effect.core_radius;
        let r1 = effect.core_radius * effect.tip_radius_ratio;

        for i in 0..8 {
            let t = i as f32 / 8.0;
            let radius_a = r0 * (1.0 - t) + r1 * t;
            let expected = effect.edge_fraction * radius_a * radius_a;
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
    fn test_deterministic() {
        let effect = make_effect();
        let segments1 = build_segments(&effect);
        let segments2 = build_segments(&effect);

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
    fn test_shape_packing() {
        let effect = make_effect();
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
        assert!(
            (ubo.shape[2] - 8.0).abs() < 1e-6,
            "shape.z should be 8.0: {}",
            ubo.shape[2]
        );
    }
}
