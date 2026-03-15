#[cfg(test)]
mod tests {
    const TOLERANCE: f32 = 0.01;

    fn srgb_encode_channel(linear: f32) -> f32 {
        if linear <= 0.0031308 {
            linear * 12.92
        } else {
            1.055 * linear.powf(1.0 / 2.4) - 0.055
        }
    }

    fn srgb_encode(r: f32, g: f32, b: f32) -> [f32; 3] {
        [
            srgb_encode_channel(r),
            srgb_encode_channel(g),
            srgb_encode_channel(b),
        ]
    }

    fn aces_filmic_channel(x: f32) -> f32 {
        let a = 2.51;
        let b = 0.03;
        let c = 2.43;
        let d = 0.59;
        let e = 0.14;
        ((x * (a * x + b)) / (x * (c * x + d) + e)).clamp(0.0, 1.0)
    }

    fn aces_filmic(r: f32, g: f32, b: f32) -> [f32; 3] {
        [
            aces_filmic_channel(r),
            aces_filmic_channel(g),
            aces_filmic_channel(b),
        ]
    }

    fn reinhard(r: f32, g: f32, b: f32) -> [f32; 3] {
        [r / (r + 1.0), g / (g + 1.0), b / (b + 1.0)]
    }

    enum ToneMapOperator {
        None,
        AcesFilmic,
        Reinhard,
    }

    fn simulate_pipeline(
        linear: [f32; 3],
        exposure: f32,
        operator: ToneMapOperator,
        gamma: f32,
    ) -> [f32; 3] {
        let exposed = [
            linear[0] * exposure,
            linear[1] * exposure,
            linear[2] * exposure,
        ];

        let mapped = match operator {
            ToneMapOperator::None => [
                exposed[0].clamp(0.0, 1.0),
                exposed[1].clamp(0.0, 1.0),
                exposed[2].clamp(0.0, 1.0),
            ],
            ToneMapOperator::AcesFilmic => aces_filmic(exposed[0], exposed[1], exposed[2]),
            ToneMapOperator::Reinhard => reinhard(exposed[0], exposed[1], exposed[2]),
        };

        let gamma_corrected = [
            mapped[0].powf(1.0 / gamma),
            mapped[1].powf(1.0 / gamma),
            mapped[2].powf(1.0 / gamma),
        ];

        srgb_encode(gamma_corrected[0], gamma_corrected[1], gamma_corrected[2])
    }

    fn blender_standard_output(linear: [f32; 3]) -> [f32; 3] {
        srgb_encode(linear[0], linear[1], linear[2])
    }

    fn to_u8(srgb: [f32; 3]) -> [u8; 3] {
        [
            (srgb[0] * 255.0).round() as u8,
            (srgb[1] * 255.0).round() as u8,
            (srgb[2] * 255.0).round() as u8,
        ]
    }

    fn assert_color_eq(label: &str, actual: [f32; 3], expected: [f32; 3]) {
        let diff = [
            (actual[0] - expected[0]).abs(),
            (actual[1] - expected[1]).abs(),
            (actual[2] - expected[2]).abs(),
        ];
        let max_diff = diff[0].max(diff[1]).max(diff[2]);
        assert!(
            max_diff < TOLERANCE,
            "{}: actual {:?} (8bit {:?}) != expected {:?} (8bit {:?}), max_diff={}",
            label,
            actual,
            to_u8(actual),
            expected,
            to_u8(expected),
            max_diff
        );
    }

    struct TestColor {
        name: &'static str,
        linear: [f32; 3],
    }

    fn test_colors() -> Vec<TestColor> {
        vec![
            TestColor {
                name: "gray_#414141",
                linear: [0.051, 0.051, 0.051],
            },
            TestColor {
                name: "red_0.6",
                linear: [0.6, 0.0, 0.0],
            },
            TestColor {
                name: "green_0.4",
                linear: [0.0, 0.4, 0.0],
            },
            TestColor {
                name: "blue_1.0",
                linear: [0.0, 0.0, 1.0],
            },
        ]
    }

    #[test]
    fn test_pipeline_gamma1_matches_blender_standard() {
        for tc in test_colors() {
            let pipeline_output = simulate_pipeline(tc.linear, 1.0, ToneMapOperator::None, 1.0);
            let blender_output = blender_standard_output(tc.linear);

            assert_color_eq(
                &format!("{} (gamma=1.0, no tonemap)", tc.name),
                pipeline_output,
                blender_output,
            );
        }
    }

    #[test]
    fn test_pipeline_gamma2_2_causes_double_gamma() {
        let non_saturated_colors = vec![
            TestColor {
                name: "gray_#414141",
                linear: [0.051, 0.051, 0.051],
            },
            TestColor {
                name: "red_0.6",
                linear: [0.6, 0.0, 0.0],
            },
            TestColor {
                name: "green_0.4",
                linear: [0.0, 0.4, 0.0],
            },
            TestColor {
                name: "mid_gray",
                linear: [0.5, 0.5, 0.5],
            },
        ];

        for tc in non_saturated_colors {
            let pipeline_output = simulate_pipeline(tc.linear, 1.0, ToneMapOperator::None, 2.2);
            let blender_output = blender_standard_output(tc.linear);

            let max_component_diff = (0..3)
                .filter(|&ch| tc.linear[ch] > 0.01)
                .map(|ch| (pipeline_output[ch] - blender_output[ch]).abs())
                .fold(0.0_f32, f32::max);

            assert!(
                max_component_diff > TOLERANCE,
                "{}: gamma=2.2 should NOT match Blender (double gamma), \
                 but diff={:.4} pipeline={:?} blender={:?}",
                tc.name,
                max_component_diff,
                to_u8(pipeline_output),
                to_u8(blender_output),
            );
        }
    }

    #[test]
    fn test_srgb_encode_known_values() {
        let encoded = srgb_encode(0.051, 0.051, 0.051);
        let u8_val = to_u8(encoded);
        assert!(
            (u8_val[0] as i32 - 65).unsigned_abs() <= 2,
            "linear 0.051 should encode to ~65 (#414141), got {}",
            u8_val[0]
        );

        let encoded_red = srgb_encode(0.6, 0.0, 0.0);
        let u8_red = to_u8(encoded_red);
        assert!(
            (u8_red[0] as i32 - 201).unsigned_abs() <= 3,
            "linear 0.6 should encode to ~201, got {}",
            u8_red[0]
        );
    }

    #[test]
    fn test_aces_compresses_highlights() {
        assert!(
            aces_filmic_channel(2.0) < 2.0,
            "ACES should compress value > 1.0"
        );
        assert!(
            aces_filmic_channel(5.0) < 1.0,
            "ACES should map HDR to [0,1]"
        );

        let bright = aces_filmic_channel(1.0);
        let very_bright = aces_filmic_channel(10.0);
        assert!(
            very_bright > bright * 0.8,
            "ACES should have gentle rolloff: f(10)={} vs f(1)={}",
            very_bright,
            bright,
        );
    }

    #[test]
    fn test_background_color_bypasses_tonemap() {
        let bg_linear = [0.051_f32; 3];
        let gamma = 2.2;

        let bg_after_gamma = [
            bg_linear[0].powf(1.0 / gamma),
            bg_linear[1].powf(1.0 / gamma),
            bg_linear[2].powf(1.0 / gamma),
        ];
        let bg_final = srgb_encode(bg_after_gamma[0], bg_after_gamma[1], bg_after_gamma[2]);
        let bg_u8 = to_u8(bg_final);

        let direct_srgb = srgb_encode(bg_linear[0], bg_linear[1], bg_linear[2]);
        let direct_u8 = to_u8(direct_srgb);

        assert_ne!(
            bg_u8[0], direct_u8[0],
            "Background with gamma=2.2 + sRGB encode ({}) should differ from \
             direct sRGB encode ({}) — confirms double gamma affects background",
            bg_u8[0], direct_u8[0]
        );

        assert!(
            (direct_u8[0] as i32 - 65).unsigned_abs() <= 2,
            "Direct sRGB encode of 0.051 should give ~65 (#41), got {}",
            direct_u8[0]
        );
    }

    #[test]
    fn test_pipeline_exposure_scales_correctly() {
        let linear = [0.3, 0.3, 0.3];

        let exp1 = simulate_pipeline(linear, 1.0, ToneMapOperator::None, 1.0);
        let exp2 = simulate_pipeline(linear, 2.0, ToneMapOperator::None, 1.0);

        let expected_exp2 = blender_standard_output([0.6, 0.6, 0.6]);

        assert_color_eq("exposure=2 on 0.3 == 0.6 linear", exp2, expected_exp2);

        assert!(
            exp2[0] > exp1[0],
            "Higher exposure should produce brighter output"
        );
    }

    #[test]
    fn test_gamma1_is_required_for_srgb_framebuffer() {
        let linear = [0.5_f32, 0.5, 0.5];

        let gamma1_output = simulate_pipeline(linear, 1.0, ToneMapOperator::None, 1.0);
        let gamma22_output = simulate_pipeline(linear, 1.0, ToneMapOperator::None, 2.2);
        let blender_ref = blender_standard_output(linear);

        let gamma1_diff = (gamma1_output[0] - blender_ref[0]).abs();
        let gamma22_diff = (gamma22_output[0] - blender_ref[0]).abs();

        assert!(
            gamma1_diff < TOLERANCE,
            "gamma=1.0 should match Blender, diff={}",
            gamma1_diff
        );
        assert!(
            gamma22_diff > 0.05,
            "gamma=2.2 should NOT match Blender on sRGB framebuffer, diff={}",
            gamma22_diff
        );
    }
}
