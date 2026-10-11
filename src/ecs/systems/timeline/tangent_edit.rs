use crate::animation::editable::{
    align_opposite_handle, apply_tangent_by_type, initialize_weighted_handle_lengths, BezierHandle,
    InterpolationType, KeyframeId, PropertyCurve, TangentContinuity, TangentType,
    TangentWeightMode,
};

pub fn set_manual_tangents(
    curve: &mut PropertyCurve,
    keyframe_id: KeyframeId,
    in_tangent: BezierHandle,
    out_tangent: BezierHandle,
) {
    if let Some(kf) = curve.get_keyframe_mut(keyframe_id) {
        kf.tangent_type = TangentType::Manual;
    }
    curve.set_keyframe_tangents(keyframe_id, in_tangent, out_tangent);
}

pub fn set_tangent_type(
    curve: &mut PropertyCurve,
    keyframe_id: KeyframeId,
    tangent_type: TangentType,
) {
    let Some(idx) = keyframe_index(curve, keyframe_id) else {
        return;
    };
    curve.keyframes[idx].interpolation = InterpolationType::Bezier;
    curve.keyframes[idx].tangent_type = tangent_type;
    apply_tangent_by_type(&mut curve.keyframes, idx);
}

pub fn set_tangent_weight_mode(
    curve: &mut PropertyCurve,
    keyframe_id: KeyframeId,
    weight_mode: TangentWeightMode,
) {
    curve.set_keyframe_weight_mode(keyframe_id, weight_mode);
    if weight_mode != TangentWeightMode::Weighted {
        return;
    }
    let Some(idx) = keyframe_index(curve, keyframe_id) else {
        return;
    };
    let dt = average_keyframe_interval(curve).max(0.1);
    initialize_weighted_handle_lengths(&mut curve.keyframes[idx], dt);
}

pub fn set_tangent_continuity(
    curve: &mut PropertyCurve,
    keyframe_id: KeyframeId,
    continuity: TangentContinuity,
) {
    let Some(keyframe) = curve.get_keyframe_mut(keyframe_id) else {
        return;
    };
    keyframe.continuity = continuity;
    if continuity == TangentContinuity::Unified {
        keyframe.in_tangent = align_opposite_handle(&keyframe.out_tangent, &keyframe.in_tangent);
    }
}

fn keyframe_index(curve: &PropertyCurve, keyframe_id: KeyframeId) -> Option<usize> {
    curve.keyframes.iter().position(|k| k.id == keyframe_id)
}

fn average_keyframe_interval(curve: &PropertyCurve) -> f32 {
    if curve.keyframes.len() <= 1 {
        return 1.0;
    }
    let first = curve.keyframes.first().expect("guarded by len > 1").time;
    let last = curve.keyframes.last().expect("guarded by len > 1").time;
    (last - first) / (curve.keyframes.len() as f32 - 1.0)
}
