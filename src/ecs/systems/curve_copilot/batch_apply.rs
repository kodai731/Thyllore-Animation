use anyhow::bail;

use crate::animation::editable::PropertyCurve;
use thyllore_ml_core::copilot::v2::forecast;
use thyllore_ml_core::copilot::v2::inference::{V2CurveCopilotRequest, V2CurveCopilotSession};

use super::suggestion_systems::{
    build_suggestions_from_curve, build_v2_curve_copilot_context, curve_suggestion_apply,
    resolve_anchor_time, sample_or_hold,
};

pub fn copilot_extend_curve(
    session: &mut V2CurveCopilotSession,
    curve: &mut PropertyCurve,
    origin_time: f32,
    frames: usize,
) -> anyhow::Result<usize> {
    if !(1..=64).contains(&frames) {
        bail!("frames must be 1..=64, got {}", frames);
    }

    if resolve_anchor_time(curve, origin_time).is_none() {
        bail!("copilot_extend: no anchor key for origin time {origin_time}");
    }

    let dt = 1.0 / forecast::DEPLOY_FPS as f32;
    let context = build_v2_curve_copilot_context(curve, origin_time, dt);

    let request = V2CurveCopilotRequest {
        context: &context,
        fps: forecast::DEPLOY_FPS,
    };
    let mean = session.predict_mean_curve(request)?;

    let origin_value = sample_or_hold(curve, origin_time);
    let suggestions = build_suggestions_from_curve(
        &mean,
        origin_value,
        origin_time,
        dt,
        0,
        curve.property_type,
        0,
    );

    let max_time = origin_time + frames as f32 * dt;
    let mut count = 0usize;
    for suggestion in suggestions {
        if suggestion.predicted_time <= max_time {
            curve_suggestion_apply(&suggestion, curve);
            count += 1;
        }
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use crate::animation::editable::{
        curve_add_keyframe_with_tangents, BezierHandle, InterpolationType, PropertyCurve,
        PropertyType,
    };
    use crate::ml::resolve_curve_copilot_model_path;
    use thyllore_ml_core::copilot::v2::inference::V2CurveCopilotSession;

    #[test]
    #[ignore]
    fn copilot_extend_is_deterministic() {
        let model_path = resolve_curve_copilot_model_path()
            .expect("model not found — this test is #[ignore] and meant for manual execution");

        let mut curve1 = PropertyCurve::new(0, PropertyType::TranslationX);
        let mut curve2 = PropertyCurve::new(0, PropertyType::TranslationX);

        for curve in [&mut curve1, &mut curve2] {
            curve_add_keyframe_with_tangents(
                curve,
                0.0,
                0.0,
                BezierHandle::default(),
                BezierHandle::default(),
                InterpolationType::Bezier,
            );
            curve_add_keyframe_with_tangents(
                curve,
                0.5,
                30.0,
                BezierHandle::default(),
                BezierHandle::default(),
                InterpolationType::Bezier,
            );
            curve_add_keyframe_with_tangents(
                curve,
                1.0,
                10.0,
                BezierHandle::default(),
                BezierHandle::default(),
                InterpolationType::Bezier,
            );
        }

        let mut session1 =
            V2CurveCopilotSession::from_onnx_path(&model_path).expect("failed to load model");
        let mut session2 =
            V2CurveCopilotSession::from_onnx_path(&model_path).expect("failed to load model");

        let count1 = super::copilot_extend_curve(&mut session1, &mut curve1, 1.0, 30)
            .expect("extend failed on curve1");
        super::copilot_extend_curve(&mut session2, &mut curve2, 1.0, 30)
            .expect("extend failed on curve2");

        assert!(
            count1 > 0,
            "expected at least one key added, got {}",
            count1
        );

        let keys1: Vec<(f32, f32)> = curve1.keyframes.iter().map(|k| (k.time, k.value)).collect();
        let keys2: Vec<(f32, f32)> = curve2.keyframes.iter().map(|k| (k.time, k.value)).collect();

        assert_eq!(
            keys1, keys2,
            "two runs with the same model and curve produced different key sequences"
        );
    }
}
