use crate::animation::editable::{PropertyCurve, PropertyType};
use crate::animation::BoneId;
use crate::ecs::resource::{CurveSuggestionState, GhostCurveSuggestion, InferenceActorState};
use crate::ml::{InferenceActorId, InferenceRequestKind, InferenceResultKind};

use super::inference_actor_systems::{inference_actor_submit, inference_actor_take_results};

const MAX_CONTEXT_KEYFRAMES: usize = 5;
const FEATURES_PER_KEYFRAME: usize = 4;
const CONTEXT_SIZE: usize = MAX_CONTEXT_KEYFRAMES * FEATURES_PER_KEYFRAME;

pub fn curve_suggestion_extract_context(
    curve: &PropertyCurve,
    _property_type: PropertyType,
    max_keyframes: usize,
) -> Vec<f32> {
    let keyframes = &curve.keyframes;
    let count = keyframes.len().min(max_keyframes);
    let start = keyframes.len().saturating_sub(max_keyframes);

    let total_size = max_keyframes * FEATURES_PER_KEYFRAME;
    let mut context = vec![0.0f32; total_size];

    for (i, kf) in keyframes[start..].iter().enumerate().take(count) {
        let offset = i * FEATURES_PER_KEYFRAME;
        context[offset] = kf.time;
        context[offset + 1] = kf.value;
        context[offset + 2] = kf.in_tangent.time_offset;
        context[offset + 3] = kf.out_tangent.time_offset;
    }

    context
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
) {
    if !suggestion_state.enabled {
        return;
    }

    let context = curve_suggestion_extract_context(curve, property_type, MAX_CONTEXT_KEYFRAMES);
    let property_type_id = property_type_to_id(property_type);

    let kind = InferenceRequestKind::CurveCopilotPredict {
        context,
        property_type_id,
    };

    if let Some(request_id) = inference_actor_submit(inference_state, actor_id, kind) {
        suggestion_state.pending_request_id = Some(request_id);
        suggestion_state.pending_bone_id = Some(bone_id);
        suggestion_state.pending_property_type = Some(property_type);
        crate::log!("CurveCopilot: submitted request {}", request_id);
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

        if let InferenceResultKind::CurveCopilotPredict { points, confidence } = result.kind {
            let bone_id = suggestion_state.pending_bone_id.unwrap_or(0);
            let property_type = suggestion_state
                .pending_property_type
                .unwrap_or(PropertyType::TranslationX);

            suggestion_state.suggestions.push(GhostCurveSuggestion {
                bone_id,
                property_type,
                points,
                confidence,
                request_id: result.request_id,
            });

            suggestion_state.pending_request_id = None;
            suggestion_state.pending_bone_id = None;
            suggestion_state.pending_property_type = None;

            crate::log!(
                "CurveCopilot: received suggestion, confidence={:.2}",
                confidence
            );
        }
    }
}

pub fn curve_suggestion_apply(
    suggestion: &GhostCurveSuggestion,
    curve: &mut PropertyCurve,
) {
    for &(time, value) in &suggestion.points {
        curve.add_keyframe(time, value);
    }

    curve.recalculate_auto_tangents();
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
        let context = curve_suggestion_extract_context(
            &curve,
            PropertyType::TranslationX,
            5,
        );

        assert_eq!(context.len(), CONTEXT_SIZE);
        assert_eq!(context[0], 0.5);
    }

    #[test]
    fn test_extract_context_padding() {
        let curve = create_test_curve(2);
        let context = curve_suggestion_extract_context(
            &curve,
            PropertyType::TranslationX,
            5,
        );

        assert_eq!(context.len(), CONTEXT_SIZE);

        assert_eq!(context[0], 0.5);

        for i in (2 * FEATURES_PER_KEYFRAME)..CONTEXT_SIZE {
            assert_eq!(context[i], 0.0, "padding at index {} should be 0", i);
        }
    }

    #[test]
    fn test_suggestion_dismiss() {
        let mut state = CurveSuggestionState::default();
        state.suggestions.push(GhostCurveSuggestion {
            bone_id: 0,
            property_type: PropertyType::TranslationX,
            points: vec![(1.0, 2.0)],
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
            points: vec![(1.0, 0.5), (1.5, 0.8), (2.0, 1.0)],
            confidence: 0.9,
            request_id: 1,
        };

        let mut curve = create_test_curve(3);
        let before_count = curve.keyframe_count();

        curve_suggestion_apply(&suggestion, &mut curve);

        assert_eq!(
            curve.keyframe_count(),
            before_count + suggestion.points.len()
        );
    }

    #[test]
    fn test_property_type_to_id() {
        assert_eq!(property_type_to_id(PropertyType::TranslationX), 0);
        assert_eq!(property_type_to_id(PropertyType::ScaleZ), 8);
    }
}
