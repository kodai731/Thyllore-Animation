use crate::animation::editable::{BezierHandle, InterpolationType, PropertyCurve, PropertyType};
use crate::animation::BoneId;
use crate::ecs::resource::{
    BoneNameTokenCache, BoneTopologyCache, CurveSuggestionState, GhostCurveSuggestion,
    InferenceActorState,
};
use crate::ml::{InferenceActorId, InferenceRequestKind, InferenceResultKind};

use super::inference_actor_systems::{inference_actor_submit, inference_actor_take_results};

const MAX_CONTEXT_KEYFRAMES: usize = 8;
const FEATURES_PER_KEYFRAME: usize = 6;
const CONTEXT_SIZE: usize = MAX_CONTEXT_KEYFRAMES * FEATURES_PER_KEYFRAME;

pub fn curve_suggestion_extract_context(
    curve: &PropertyCurve,
    _property_type: PropertyType,
    max_keyframes: usize,
    clip_duration: f32,
) -> (Vec<f32>, f32, f32) {
    let keyframes = &curve.keyframes;
    let count = keyframes.len().min(max_keyframes);
    let start = keyframes.len().saturating_sub(max_keyframes);

    let window = &keyframes[start..start + count];

    let curve_mean = if count > 0 {
        window.iter().map(|kf| kf.value).sum::<f32>() / count as f32
    } else {
        0.0
    };

    let curve_std = if count > 0 {
        let variance =
            window.iter().map(|kf| (kf.value - curve_mean).powi(2)).sum::<f32>() / count as f32;
        variance.sqrt().max(1e-6)
    } else {
        1e-6
    };

    let total_size = max_keyframes * FEATURES_PER_KEYFRAME;
    let mut context = vec![0.0f32; total_size];
    let duration = clip_duration.max(0.001);
    let padding_offset = (max_keyframes - count) * FEATURES_PER_KEYFRAME;

    for (i, kf) in window.iter().enumerate() {
        let offset = padding_offset + i * FEATURES_PER_KEYFRAME;
        context[offset] = kf.time / duration;
        context[offset + 1] = (kf.value - curve_mean) / curve_std;
        context[offset + 2] = kf.in_tangent.time_offset / duration;
        context[offset + 3] = kf.in_tangent.value_offset / curve_std;
        context[offset + 4] = kf.out_tangent.time_offset / duration;
        context[offset + 5] = kf.out_tangent.value_offset / curve_std;
    }

    (context, curve_mean, curve_std)
}

fn property_type_to_id(property_type: PropertyType) -> u32 {
    match property_type {
        PropertyType::TranslationX => 0,
        PropertyType::TranslationY => 1,
        PropertyType::TranslationZ => 2,
        PropertyType::RotationX => 3,
        PropertyType::RotationY => 4,
        PropertyType::RotationZ => 5,
        PropertyType::ScaleX => 6,
        PropertyType::ScaleY => 7,
        PropertyType::ScaleZ => 8,
    }
}

pub fn curve_suggestion_submit(
    suggestion_state: &mut CurveSuggestionState,
    inference_state: &mut InferenceActorState,
    actor_id: InferenceActorId,
    curve: &PropertyCurve,
    property_type: PropertyType,
    bone_id: BoneId,
    clip_duration: f32,
    current_time: f32,
    topology_cache: &BoneTopologyCache,
    name_token_cache: &BoneNameTokenCache,
) {
    if !suggestion_state.enabled {
        return;
    }

    let (context, curve_mean, curve_std) = curve_suggestion_extract_context(
        curve,
        property_type,
        MAX_CONTEXT_KEYFRAMES,
        clip_duration,
    );

    let property_type_id = property_type_to_id(property_type);
    let topology_features = topology_cache.get(bone_id).to_vec();
    let bone_name_tokens = name_token_cache.get(bone_id).to_vec();
    let query_time = current_time / clip_duration.max(0.001);

    let context_debug: Vec<f32> = context[..6.min(context.len())].to_vec();

    let kind = InferenceRequestKind::CurveCopilotPredict {
        context,
        property_type_id,
        topology_features,
        bone_name_tokens,
        query_time,
    };

    if let Some(request_id) = inference_actor_submit(inference_state, actor_id, kind) {
        suggestion_state.pending_request_id = Some(request_id);
        suggestion_state.pending_bone_id = Some(bone_id);
        suggestion_state.pending_property_type = Some(property_type);
        suggestion_state.pending_clip_duration = Some(clip_duration);
        suggestion_state.pending_curve_mean = Some(curve_mean);
        suggestion_state.pending_curve_std = Some(curve_std);
        suggestion_state.pending_query_time = Some(current_time);
        crate::log!(
            "CurveCopilot: submitted request {}, property={:?}, query_time_norm={:.4}, curve_mean={:.4}, curve_std={:.4}, clip_dur={:.3}, context[0..6]={:?}",
            request_id, property_type, query_time, curve_mean, curve_std, clip_duration, context_debug
        );
    }
}

pub fn curve_suggestion_poll_results(
    suggestion_state: &mut CurveSuggestionState,
    inference_state: &mut InferenceActorState,
) {
    if suggestion_state.pending_request_id.is_none() {
        return;
    }

    let results = inference_actor_take_results(inference_state);

    for result in results {
        let pending_match = suggestion_state
            .pending_request_id
            .map_or(false, |id| id == result.request_id);

        if !pending_match {
            continue;
        }

        if let InferenceResultKind::CurveCopilotPredict {
            value,
            tangent_in,
            tangent_out,
            is_bezier,
            confidence,
        } = result.kind
        {
            let bone_id = suggestion_state.pending_bone_id.unwrap_or(0);
            let property_type = suggestion_state
                .pending_property_type
                .unwrap_or(PropertyType::TranslationX);

            let clip_duration = suggestion_state.pending_clip_duration.unwrap_or(1.0);
            let curve_mean = suggestion_state.pending_curve_mean.unwrap_or(0.0);
            let curve_std = suggestion_state.pending_curve_std.unwrap_or(1.0);
            let predicted_time = suggestion_state.pending_query_time.unwrap_or(0.0);

            let denorm_value = value * curve_std + curve_mean;
            let denorm_tan_in = (tangent_in.0 * clip_duration, tangent_in.1 * curve_std);
            let denorm_tan_out = (tangent_out.0 * clip_duration, tangent_out.1 * curve_std);

            suggestion_state.suggestions.push(GhostCurveSuggestion {
                bone_id,
                property_type,
                predicted_time,
                predicted_value: denorm_value,
                tangent_in: denorm_tan_in,
                tangent_out: denorm_tan_out,
                is_bezier,
                confidence,
                request_id: result.request_id,
            });

            suggestion_state.pending_request_id = None;
            suggestion_state.pending_bone_id = None;
            suggestion_state.pending_property_type = None;
            suggestion_state.pending_clip_duration = None;
            suggestion_state.pending_curve_mean = None;
            suggestion_state.pending_curve_std = None;
            suggestion_state.pending_query_time = None;

            crate::log!(
                "CurveCopilot: received suggestion, confidence={:.2}, denorm_value={:.4}, time={:.4}, tan_in=({:.4},{:.4}), tan_out=({:.4},{:.4}), is_bezier={}",
                confidence, denorm_value, predicted_time, denorm_tan_in.0, denorm_tan_in.1, denorm_tan_out.0, denorm_tan_out.1, is_bezier
            );
        }
    }
}

pub fn curve_suggestion_apply(
    suggestion: &GhostCurveSuggestion,
    curve: &mut PropertyCurve,
) {
    let interpolation = if suggestion.is_bezier {
        InterpolationType::Bezier
    } else {
        InterpolationType::Linear
    };

    let in_tangent = BezierHandle::new(suggestion.tangent_in.0, suggestion.tangent_in.1);
    let out_tangent = BezierHandle::new(suggestion.tangent_out.0, suggestion.tangent_out.1);

    curve.add_keyframe_with_tangents(
        suggestion.predicted_time,
        suggestion.predicted_value,
        in_tangent,
        out_tangent,
        interpolation,
    );
}

pub fn curve_suggestion_dismiss(suggestion_state: &mut CurveSuggestionState) {
    suggestion_state.suggestions.clear();
    suggestion_state.pending_request_id = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::editable::CurveId;

    fn create_test_curve(keyframe_count: usize) -> PropertyCurve {
        let mut curve = PropertyCurve::new(1 as CurveId, PropertyType::TranslationX);
        for i in 0..keyframe_count {
            let time = (i + 1) as f32 * 0.5;
            curve.add_keyframe(time, (i as f32).sin());
        }
        curve
    }

    #[test]
    fn test_extract_context_basic() {
        let curve = create_test_curve(5);
        let (context, _mean, _std) = curve_suggestion_extract_context(
            &curve,
            PropertyType::TranslationX,
            8,
            4.0,
        );

        assert_eq!(context.len(), CONTEXT_SIZE);

        let padding = (8 - 5) * FEATURES_PER_KEYFRAME;
        assert!(context[padding] > 0.0);
    }

    #[test]
    fn test_extract_context_right_aligned_padding() {
        let curve = create_test_curve(2);
        let (context, _mean, _std) = curve_suggestion_extract_context(
            &curve,
            PropertyType::TranslationX,
            8,
            4.0,
        );

        assert_eq!(context.len(), CONTEXT_SIZE);

        let padding_size = (8 - 2) * FEATURES_PER_KEYFRAME;
        for i in 0..padding_size {
            assert_eq!(context[i], 0.0, "leading padding at index {} should be 0", i);
        }

        assert!(context[padding_size] > 0.0, "first keyframe should be non-zero after padding");
    }

    #[test]
    fn test_extract_context_normalization() {
        let mut curve = PropertyCurve::new(1 as CurveId, PropertyType::RotationX);
        curve.add_keyframe(1.0, 90.0);
        curve.add_keyframe(2.0, 180.0);

        let (context, curve_mean, curve_std) = curve_suggestion_extract_context(
            &curve,
            PropertyType::RotationX,
            8,
            4.0,
        );

        assert!((curve_mean - 135.0).abs() < 0.001, "mean should be 135.0");
        assert!((curve_std - 45.0).abs() < 0.001, "std should be 45.0");

        let padding = (8 - 2) * FEATURES_PER_KEYFRAME;
        assert!((context[padding] - 0.25).abs() < 0.001, "time should be 1.0/4.0 = 0.25");
        assert!(
            (context[padding + 1] - (-1.0)).abs() < 0.001,
            "value should be (90.0 - 135.0) / 45.0 = -1.0"
        );
        assert!(
            (context[padding + FEATURES_PER_KEYFRAME + 1] - 1.0).abs() < 0.001,
            "value should be (180.0 - 135.0) / 45.0 = 1.0"
        );
    }

    #[test]
    fn test_extract_context_constant_curve() {
        let mut curve = PropertyCurve::new(1 as CurveId, PropertyType::RotationX);
        curve.add_keyframe(0.0, 42.0);
        curve.add_keyframe(1.0, 42.0);
        curve.add_keyframe(2.0, 42.0);

        let (_context, curve_mean, curve_std) = curve_suggestion_extract_context(
            &curve,
            PropertyType::RotationX,
            8,
            4.0,
        );

        assert!((curve_mean - 42.0).abs() < 0.001, "mean should be 42.0");
        assert!((curve_std - 1e-6).abs() < 1e-7, "std should be clamped to 1e-6");
    }

    #[test]
    fn test_suggestion_dismiss() {
        let mut state = CurveSuggestionState::default();
        state.suggestions.push(GhostCurveSuggestion {
            bone_id: 0,
            property_type: PropertyType::TranslationX,
            predicted_time: 1.0,
            predicted_value: 2.0,
            tangent_in: (0.0, 0.0),
            tangent_out: (0.0, 0.0),
            is_bezier: false,
            confidence: 0.9,
            request_id: 42,
        });
        state.pending_request_id = Some(100);

        curve_suggestion_dismiss(&mut state);

        assert!(state.suggestions.is_empty());
        assert!(state.pending_request_id.is_none());
    }

    #[test]
    fn test_suggestion_apply() {
        let suggestion = GhostCurveSuggestion {
            bone_id: 0,
            property_type: PropertyType::TranslationX,
            predicted_time: 1.5,
            predicted_value: 0.8,
            tangent_in: (-0.1, 0.0),
            tangent_out: (0.1, 0.0),
            is_bezier: true,
            confidence: 0.9,
            request_id: 1,
        };

        let mut curve = create_test_curve(3);
        let before_count = curve.keyframe_count();

        curve_suggestion_apply(&suggestion, &mut curve);

        assert_eq!(curve.keyframe_count(), before_count + 1);
    }

    #[test]
    fn test_property_type_to_id() {
        assert_eq!(property_type_to_id(PropertyType::TranslationX), 0);
        assert_eq!(property_type_to_id(PropertyType::ScaleZ), 8);
    }
}
