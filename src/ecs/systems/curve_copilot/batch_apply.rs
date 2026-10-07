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

/// Follow-through into a hold: keys between `origin_time` and `until` take `blend` of the
/// Copilot forecast's deviation from the held value.
pub fn copilot_settle_curve(
    session: &mut V2CurveCopilotSession,
    curve: &mut PropertyCurve,
    origin_time: f32,
    until: f32,
    max_frames: usize,
    blend: f32,
) -> anyhow::Result<usize> {
    let dt = 1.0 / forecast::DEPLOY_FPS as f32;
    let frames_until_hold_end = ((until - origin_time) / dt).floor() as usize;
    let frames = max_frames.min(frames_until_hold_end.saturating_sub(1));
    if frames == 0 {
        return Ok(0);
    }
    if resolve_anchor_time(curve, origin_time).is_none() {
        bail!("copilot_settle: no anchor key for origin time {origin_time}");
    }

    let context = build_v2_curve_copilot_context(curve, origin_time, dt);
    let mean = session.predict_mean_curve(V2CurveCopilotRequest {
        context: &context,
        fps: forecast::DEPLOY_FPS,
    })?;

    let hold_value = sample_or_hold(curve, origin_time);
    let suggestions = build_suggestions_from_curve(
        &mean,
        hold_value,
        origin_time,
        dt,
        0,
        curve.property_type,
        0,
    );
    let max_time = origin_time + frames as f32 * dt;
    let mut count = 0usize;
    for suggestion in suggestions {
        if suggestion.predicted_time > max_time {
            continue;
        }
        let blended = crate::ecs::resource::GhostCurveSuggestion {
            predicted_value: hold_value + blend * (suggestion.predicted_value - hold_value),
            tangent_in: (suggestion.tangent_in.0, blend * suggestion.tangent_in.1),
            tangent_out: (suggestion.tangent_out.0, blend * suggestion.tangent_out.1),
            ..suggestion
        };
        curve_suggestion_apply(&blended, curve);
        count += 1;
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

#[cfg(test)]
mod punch_probe {
    use std::path::Path;

    use crate::animation::editable::{curve_sample, PropertyCurve, PropertyType};
    use crate::ml::resolve_curve_copilot_model_path;
    use thyllore_ml_core::copilot::v2::forecast;
    use thyllore_ml_core::copilot::v2::inference::{V2CurveCopilotRequest, V2CurveCopilotSession};

    use super::super::suggestion_systems::build_v2_curve_copilot_context;

    fn slope(curve: &PropertyCurve, time: f32, dt: f32) -> f32 {
        let a = curve_sample(curve, time).unwrap_or(0.0);
        let b = curve_sample(curve, time + dt).unwrap_or(a);
        (b - a) / dt
    }

    #[test]
    #[ignore]
    fn forecast_versus_hermite_on_left_punch_2() {
        let model_path = resolve_curve_copilot_model_path().expect("model path");
        let mut session = V2CurveCopilotSession::from_onnx_path(&model_path).expect("model");
        let clip_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/templates/left_punch_2.anim.ron");
        let clip = crate::scene::load_animation_clip(&clip_path).expect("clip");
        let dt = 1.0 / forecast::DEPLOY_FPS;

        let mut rows = Vec::new();
        for track in clip.tracks.values() {
            for property in [
                PropertyType::RotationX,
                PropertyType::RotationY,
                PropertyType::RotationZ,
            ] {
                let curve = track.get_curve(property);
                if curve.keyframes.iter().all(|k| k.value.abs() < 1e-6) {
                    continue;
                }
                let times: Vec<f32> = curve.keyframes.iter().map(|k| k.time).collect();
                for (i, &origin) in times.iter().enumerate() {
                    let Some(&next) = times.get(i + 1) else {
                        continue;
                    };
                    let context = build_v2_curve_copilot_context(curve, origin, dt);
                    let mean = session
                        .predict_mean_curve(V2CurveCopilotRequest {
                            context: &context,
                            fps: forecast::DEPLOY_FPS,
                        })
                        .expect("forecast");
                    let offset = forecast::continuity_offset(
                        &mean,
                        curve_sample(curve, origin).unwrap_or(0.0),
                    );

                    let hermite_slope = slope(curve, origin, dt);
                    let forecast_slope = (mean[1] - mean[0]) / dt;
                    let forecast_slope_5 = (mean[5] - mean[0]) / (5.0 * dt);
                    let segment_frames = (((next - origin) / dt).round() as usize).min(mean.len());
                    let mut sum_sq = 0.0f32;
                    let mut max_abs = 0.0f32;
                    for f in 0..segment_frames {
                        let t = origin + (f as f32 + 1.0) * dt;
                        let h = curve_sample(curve, t).unwrap_or(0.0);
                        let c = mean[f] + offset;
                        let d = c - h;
                        sum_sq += d * d;
                        max_abs = max_abs.max(d.abs());
                    }
                    let rms = (sum_sq / segment_frames.max(1) as f32).sqrt();
                    let next_value = curve.keyframes[i + 1].value;
                    let forecast_at_next = mean[segment_frames.saturating_sub(1)] + offset;
                    rows.push(format!(
                        "{}.{:?} @{:.2}->{:.2}: key {:+.0}->{:+.0} | slope hermite {:+.0} forecast {:+.0} (5f {:+.0}) deg/s | forecast@next {:+.1} | rms {:.1} max {:.1}",
                        track.bone_name, property, origin, next, curve.keyframes[i].value, next_value,
                        hermite_slope, forecast_slope, forecast_slope_5, forecast_at_next, rms, max_abs
                    ));
                }
            }
        }
        rows.sort();
        for row in &rows {
            println!("{row}");
        }
    }
}
