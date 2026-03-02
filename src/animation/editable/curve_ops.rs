use super::curve::PropertyCurve;
use super::keyframe::{BezierHandle, EditableKeyframe, InterpolationType, KeyframeId};
use super::tangent::{apply_auto_tangent, sample_bezier};

pub fn curve_add_keyframe(curve: &mut PropertyCurve, time: f32, value: f32) -> KeyframeId {
    let id = curve.allocate_keyframe_id();
    let keyframe = EditableKeyframe::new(id, time, value);
    curve.keyframes.push(keyframe);
    curve_sort_keyframes(curve);
    id
}

pub fn curve_add_keyframe_with_tangents(
    curve: &mut PropertyCurve,
    time: f32,
    value: f32,
    in_tangent: BezierHandle,
    out_tangent: BezierHandle,
    interpolation: InterpolationType,
) -> KeyframeId {
    let id = curve.allocate_keyframe_id();
    let mut keyframe = EditableKeyframe::with_tangents(id, time, value, in_tangent, out_tangent);
    keyframe.interpolation = interpolation;
    curve.keyframes.push(keyframe);
    curve_sort_keyframes(curve);
    id
}

pub fn curve_remove_keyframe(curve: &mut PropertyCurve, keyframe_id: KeyframeId) -> bool {
    if let Some(pos) = curve.keyframes.iter().position(|k| k.id == keyframe_id) {
        curve.keyframes.remove(pos);
        true
    } else {
        false
    }
}

pub fn curve_set_keyframe_time(curve: &mut PropertyCurve, keyframe_id: KeyframeId, time: f32) {
    if let Some(kf) = curve.get_keyframe_mut(keyframe_id) {
        kf.time = time;
    }
    curve_sort_keyframes(curve);
}

pub fn curve_sort_keyframes(curve: &mut PropertyCurve) {
    curve.keyframes.sort_by(|a, b| {
        a.time
            .partial_cmp(&b.time)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

pub fn curve_sample(curve: &PropertyCurve, time: f32) -> Option<f32> {
    if curve.keyframes.is_empty() {
        return None;
    }

    if curve.keyframes.len() == 1 {
        return Some(curve.keyframes[0].value);
    }

    if time <= curve.keyframes[0].time {
        return Some(curve.keyframes[0].value);
    }

    let last = curve.keyframes.last().unwrap();
    if time >= last.time {
        return Some(last.value);
    }

    let idx = curve.keyframes.partition_point(|kf| kf.time <= time);
    let i = if idx == 0 {
        0
    } else {
        (idx - 1).min(curve.keyframes.len().saturating_sub(2))
    };

    let k0 = &curve.keyframes[i];
    let k1 = &curve.keyframes[i + 1];

    Some(match k0.interpolation {
        InterpolationType::Stepped => k0.value,
        InterpolationType::Linear => {
            let t = (time - k0.time) / (k1.time - k0.time);
            k0.value + (k1.value - k0.value) * t
        }
        InterpolationType::Bezier => sample_bezier(
            k0.time,
            k0.value,
            &k0.out_tangent,
            k1.time,
            k1.value,
            &k1.in_tangent,
            time,
        ),
    })
}

pub fn curve_recalculate_auto_tangents(curve: &mut PropertyCurve) {
    for i in 0..curve.keyframes.len() {
        apply_auto_tangent(&mut curve.keyframes, i);
    }
}

pub fn curve_recalculate_auto_tangent_at(curve: &mut PropertyCurve, keyframe_id: KeyframeId) {
    if let Some(idx) = curve.keyframes.iter().position(|k| k.id == keyframe_id) {
        if idx > 0 {
            apply_auto_tangent(&mut curve.keyframes, idx - 1);
        }
        apply_auto_tangent(&mut curve.keyframes, idx);
        if idx + 1 < curve.keyframes.len() {
            apply_auto_tangent(&mut curve.keyframes, idx + 1);
        }
    }
}
