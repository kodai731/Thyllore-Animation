use crate::editable::components::clip::EditableAnimationClip;
use crate::editable::components::curve::{CurveExtrapolation, PropertyCurve, PropertyType};
use crate::editable::components::keyframe::{
    BezierHandle, EditableKeyframe, InterpolationType, KeyframeId,
};
use crate::editable::systems::tangent::{apply_auto_tangent, sample_bezier, split_bezier};
use crate::BoneId;

enum CurveEnd {
    Start,
    End,
}

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

pub fn curve_resample_at_fps(curve: &mut PropertyCurve, duration: f32, fps: f32) -> usize {
    if curve.keyframes.is_empty() || fps <= 0.0 || duration <= 0.0 {
        return curve.keyframes.len();
    }

    let frame_interval = 1.0 / fps;
    let last_frame = (duration * fps).round().max(1.0) as usize;
    let samples: Vec<(f32, f32)> = (0..=last_frame)
        .map(|frame| {
            let time = (frame as f32 * frame_interval).min(duration);
            (time, curve_sample(curve, time).unwrap_or(0.0))
        })
        .collect();

    curve.keyframes.clear();
    for (time, value) in samples {
        let id = curve.allocate_keyframe_id();
        curve.keyframes.push(EditableKeyframe::new(id, time, value));
    }
    curve_sort_keyframes(curve);
    curve.keyframes.len()
}

pub fn curve_sample(curve: &PropertyCurve, time: f32) -> Option<f32> {
    let keyframes = &curve.keyframes;
    let (Some(first), Some(last)) = (keyframes.first(), keyframes.last()) else {
        return None;
    };

    let first_time = first.time;
    let last_time = last.time;

    if time < first_time {
        extrapolate(keyframes, CurveEnd::Start, curve.pre_extrapolation, time)
    } else if time > last_time {
        extrapolate(keyframes, CurveEnd::End, curve.post_extrapolation, time)
    } else {
        keyframes_sample(keyframes, time)
    }
}

fn extrapolate(
    keyframes: &[EditableKeyframe],
    end: CurveEnd,
    mode: CurveExtrapolation,
    time: f32,
) -> Option<f32> {
    let first = keyframes.first()?;
    let last = keyframes.last()?;
    match mode {
        CurveExtrapolation::Constant => match end {
            CurveEnd::Start => Some(first.value),
            CurveEnd::End => Some(last.value),
        },
        CurveExtrapolation::Linear => {
            let (tangent, anchor_time, anchor_value) = match end {
                CurveEnd::Start => {
                    let tangent = keyframes
                        .get(1)
                        .map(|second| compute_start_tangent(first, second))
                        .unwrap_or(0.0);
                    (tangent, first.time, first.value)
                }
                CurveEnd::End => {
                    let tangent = keyframes
                        .len()
                        .checked_sub(2)
                        .and_then(|i| keyframes.get(i))
                        .map(|prev| compute_end_tangent(prev, last))
                        .unwrap_or(0.0);
                    (tangent, last.time, last.value)
                }
            };
            Some(anchor_value + tangent * (time - anchor_time))
        }
        CurveExtrapolation::Cycle | CurveExtrapolation::CycleWithOffset => {
            cycle_sample(keyframes, mode, time)
        }
    }
}

fn compute_start_tangent(first: &EditableKeyframe, second: &EditableKeyframe) -> f32 {
    if first.interpolation == InterpolationType::Bezier {
        let out = &first.out_tangent;
        if out.time_offset != 0.0 {
            return out.value_offset / out.time_offset;
        }
    }
    (second.value - first.value) / (second.time - first.time)
}

fn compute_end_tangent(prev: &EditableKeyframe, last: &EditableKeyframe) -> f32 {
    if last.interpolation == InterpolationType::Bezier {
        let in_h = &last.in_tangent;
        if in_h.time_offset != 0.0 {
            return in_h.value_offset / in_h.time_offset;
        }
    }
    (last.value - prev.value) / (last.time - prev.time)
}

fn cycle_sample(
    keyframes: &[EditableKeyframe],
    mode: CurveExtrapolation,
    time: f32,
) -> Option<f32> {
    let first = keyframes.first()?;
    let last = keyframes.last()?;
    let range = last.time - first.time;
    if range <= 0.0 {
        return Some(first.value);
    }
    let mapped = cycle_map_time(time, first.time, range);
    if mode == CurveExtrapolation::CycleWithOffset {
        let cycles = ((time - first.time) / range).floor();
        let offset = cycles * (last.value - first.value);
        keyframes_sample(keyframes, mapped).map(|v| v + offset)
    } else {
        keyframes_sample(keyframes, mapped)
    }
}

fn cycle_map_time(time: f32, first_time: f32, range: f32) -> f32 {
    let offset = ((time - first_time) % range + range) % range;
    first_time + offset
}

/// Sample a list of keyframes at `time`. Returns `None` if keyframes are empty.
/// Clamps `time` to the range of first and last keyframes.
/// Uses binary search (partition_point) to find the interval, then interpolates
/// linearly (or bezier if either endpoint is Bezier). Stepped holds the previous value.
pub fn keyframes_sample(keyframes: &[EditableKeyframe], time: f32) -> Option<f32> {
    if keyframes.is_empty() {
        return None;
    }

    if keyframes.len() == 1 {
        return Some(keyframes[0].value);
    }

    if time <= keyframes[0].time {
        return Some(keyframes[0].value);
    }

    if let Some(last) = keyframes.last() {
        if time >= last.time {
            return Some(last.value);
        }
    }

    let idx = keyframes.partition_point(|kf| kf.time <= time);
    let i = if idx == 0 {
        0
    } else {
        (idx - 1).min(keyframes.len().saturating_sub(2))
    };

    let k0 = &keyframes[i];
    let k1 = &keyframes[i + 1];

    if k0.interpolation == InterpolationType::Stepped {
        return Some(k0.value);
    }

    let either_bezier = k0.interpolation == InterpolationType::Bezier
        || k1.interpolation == InterpolationType::Bezier;

    Some(if either_bezier {
        let (out_h, in_h) = segment_bezier_handles(k0, k1);
        sample_bezier(k0.time, k0.value, &out_h, k1.time, k1.value, &in_h, time)
    } else {
        let t = (time - k0.time) / (k1.time - k0.time);
        k0.value + (k1.value - k0.value) * t
    })
}

pub fn segment_uses_bezier(k0: &EditableKeyframe, k1: &EditableKeyframe) -> bool {
    k0.interpolation == InterpolationType::Bezier || k1.interpolation == InterpolationType::Bezier
}

fn linear_handle(from: &EditableKeyframe, to: &EditableKeyframe) -> BezierHandle {
    let dt = to.time - from.time;
    let dv = to.value - from.value;
    BezierHandle::new(dt / 3.0, dv / 3.0)
}

fn segment_bezier_handles(
    k0: &EditableKeyframe,
    k1: &EditableKeyframe,
) -> (BezierHandle, BezierHandle) {
    let out_h = if k0.interpolation == InterpolationType::Bezier {
        k0.out_tangent.clone()
    } else {
        linear_handle(k0, k1)
    };
    let in_h = if k1.interpolation == InterpolationType::Bezier {
        k1.in_tangent.clone()
    } else {
        let h = linear_handle(k0, k1);
        BezierHandle::new(-h.time_offset, -h.value_offset)
    };
    (out_h, in_h)
}

pub fn curve_recalculate_auto_tangents(curve: &mut PropertyCurve) {
    for i in 0..curve.keyframes.len() {
        apply_auto_tangent(&mut curve.keyframes, i);
    }
}

pub fn clip_add_keyframe(
    clip: &mut EditableAnimationClip,
    bone_id: BoneId,
    property_type: PropertyType,
    time: f32,
    value: f32,
) -> Option<KeyframeId> {
    clip.get_track_mut(bone_id)
        .map(|track| curve_add_keyframe(track.get_curve_mut(property_type), time, value))
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

pub fn curve_insert_keyframe_preserving_shape(
    curve: &mut PropertyCurve,
    time: f32,
) -> Option<KeyframeId> {
    for kf in &curve.keyframes {
        if (kf.time - time).abs() < 1e-4 {
            return None;
        }
    }

    let (Some(first), Some(last)) = (curve.keyframes.first(), curve.keyframes.last()) else {
        let id = curve.allocate_keyframe_id();
        curve.keyframes.push(EditableKeyframe::new(id, time, 0.0));
        return Some(id);
    };

    if time <= first.time {
        let value = first.value;
        let id = curve.allocate_keyframe_id();
        curve.keyframes.push(EditableKeyframe::new(id, time, value));
        curve_sort_keyframes(curve);
        return Some(id);
    }

    if time >= last.time {
        let value = last.value;
        let id = curve.allocate_keyframe_id();
        curve.keyframes.push(EditableKeyframe::new(id, time, value));
        curve_sort_keyframes(curve);
        return Some(id);
    }

    let idx = curve.keyframes.partition_point(|kf| kf.time <= time);
    let i = (idx - 1).min(curve.keyframes.len().saturating_sub(2));

    let k0 = &curve.keyframes[i];
    let k1 = &curve.keyframes[i + 1];

    if k0.interpolation == InterpolationType::Stepped {
        let value = k0.value;
        let id = curve.allocate_keyframe_id();
        let mut new_kf = EditableKeyframe::new(id, time, value);
        new_kf.interpolation = InterpolationType::Stepped;
        curve.keyframes.push(new_kf);
        curve_sort_keyframes(curve);
        Some(id)
    } else if segment_uses_bezier(k0, k1) {
        let (out_h, in_h) = segment_bezier_handles(k0, k1);
        let split = split_bezier(k0.time, k0.value, &out_h, k1.time, k1.value, &in_h, time);

        let id = curve.allocate_keyframe_id();
        let mut new_kf = EditableKeyframe::with_tangents(
            id,
            time,
            split.inserted_value,
            split.inserted_in,
            split.inserted_out,
        );
        new_kf.interpolation = InterpolationType::Bezier;

        curve.keyframes[i].out_tangent = split.previous_out;
        curve.keyframes[i + 1].in_tangent = split.next_in;

        if curve.keyframes[i].interpolation != InterpolationType::Bezier {
            curve.keyframes[i].interpolation = InterpolationType::Bezier;
            if i > 0 {
                let k_prev = &curve.keyframes[i - 1];
                let h = linear_handle(k_prev, &curve.keyframes[i]);
                curve.keyframes[i].in_tangent = BezierHandle::new(-h.time_offset, -h.value_offset);
            }
        }

        if curve.keyframes[i + 1].interpolation != InterpolationType::Bezier {
            curve.keyframes[i + 1].interpolation = InterpolationType::Bezier;
            if i + 2 < curve.keyframes.len() {
                let k_next = &curve.keyframes[i + 2];
                curve.keyframes[i + 1].out_tangent = linear_handle(&curve.keyframes[i + 1], k_next);
            }
        }

        curve.keyframes.push(new_kf);
        curve_sort_keyframes(curve);
        Some(id)
    } else {
        let t = (time - k0.time) / (k1.time - k0.time);
        let value = k0.value + (k1.value - k0.value) * t;
        let id = curve.allocate_keyframe_id();
        curve.keyframes.push(EditableKeyframe::new(id, time, value));
        curve_sort_keyframes(curve);
        Some(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editable::compute_handle_length;
    use crate::editable::TangentWeightMode;

    fn make_curve_with_keyframes(times_values: &[(f32, f32)]) -> PropertyCurve {
        let mut curve = PropertyCurve::new(1, PropertyType::TranslationX);
        for &(time, value) in times_values {
            curve_add_keyframe(&mut curve, time, value);
        }
        curve
    }

    #[test]
    fn test_curve_sample_linear() {
        let mut curve = make_curve_with_keyframes(&[(0.0, 0.0), (1.0, 10.0)]);
        for kf in &mut curve.keyframes {
            kf.interpolation = InterpolationType::Linear;
        }

        let Some(v) = curve_sample(&curve, 0.5) else {
            unreachable!()
        };
        assert!((v - 5.0).abs() < 1e-4);
        let Some(v) = curve_sample(&curve, 0.0) else {
            unreachable!()
        };
        assert!((v - 0.0).abs() < 1e-4);
        let Some(v) = curve_sample(&curve, 1.0) else {
            unreachable!()
        };
        assert!((v - 10.0).abs() < 1e-4);
    }

    #[test]
    fn test_curve_sample_stepped() {
        let mut curve = make_curve_with_keyframes(&[(0.0, 0.0), (1.0, 10.0)]);
        curve.keyframes[0].interpolation = InterpolationType::Stepped;

        let Some(v) = curve_sample(&curve, 0.5) else {
            unreachable!()
        };
        assert!((v - 0.0).abs() < 1e-4);
    }

    #[test]
    fn test_curve_sample_single_keyframe() {
        let curve = make_curve_with_keyframes(&[(0.5, 7.0)]);
        let Some(v) = curve_sample(&curve, 0.0) else {
            unreachable!()
        };
        assert!((v - 7.0).abs() < 1e-4);
        let Some(v) = curve_sample(&curve, 1.0) else {
            unreachable!()
        };
        assert!((v - 7.0).abs() < 1e-4);
    }

    #[test]
    fn test_curve_sample_empty() {
        let curve = PropertyCurve::new(1, PropertyType::TranslationX);
        assert!(curve_sample(&curve, 0.5).is_none());
    }

    #[test]
    fn test_curve_add_and_remove_keyframe() {
        let mut curve = PropertyCurve::new(1, PropertyType::TranslationX);
        let id1 = curve_add_keyframe(&mut curve, 0.0, 1.0);
        let id2 = curve_add_keyframe(&mut curve, 1.0, 2.0);
        assert_eq!(curve.keyframe_count(), 2);

        assert!(curve_remove_keyframe(&mut curve, id1));
        assert_eq!(curve.keyframe_count(), 1);
        assert_eq!(curve.keyframes[0].id, id2);
    }

    #[test]
    fn test_curve_sort_after_add() {
        let mut curve = PropertyCurve::new(1, PropertyType::TranslationX);
        curve_add_keyframe(&mut curve, 2.0, 20.0);
        curve_add_keyframe(&mut curve, 0.0, 0.0);
        curve_add_keyframe(&mut curve, 1.0, 10.0);

        assert!((curve.keyframes[0].time - 0.0).abs() < 1e-6);
        assert!((curve.keyframes[1].time - 1.0).abs() < 1e-6);
        assert!((curve.keyframes[2].time - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_set_keyframe_weight_mode() {
        let mut curve = make_curve_with_keyframes(&[(0.0, 0.0), (1.0, 10.0)]);
        let kf_id = curve.keyframes[0].id;

        assert_eq!(
            curve.keyframes[0].weight_mode,
            TangentWeightMode::NonWeighted
        );

        curve.set_keyframe_weight_mode(kf_id, TangentWeightMode::Weighted);
        assert_eq!(curve.keyframes[0].weight_mode, TangentWeightMode::Weighted);

        curve.set_keyframe_weight_mode(kf_id, TangentWeightMode::NonWeighted);
        assert_eq!(
            curve.keyframes[0].weight_mode,
            TangentWeightMode::NonWeighted
        );
    }

    #[test]
    fn test_weighted_bezier_custom_handles_change_curve_shape() {
        let mut curve_a = make_curve_with_keyframes(&[(0.0, 0.0), (2.0, 10.0)]);
        let mut curve_b = make_curve_with_keyframes(&[(0.0, 0.0), (2.0, 10.0)]);

        for kf in &mut curve_a.keyframes {
            kf.interpolation = InterpolationType::Bezier;
        }
        for kf in &mut curve_b.keyframes {
            kf.interpolation = InterpolationType::Bezier;
        }

        curve_a.keyframes[0].out_tangent = BezierHandle::new(0.66, 0.0);
        curve_a.keyframes[1].in_tangent = BezierHandle::new(-0.66, 0.0);

        curve_b.keyframes[0].out_tangent = BezierHandle::new(0.66, 8.0);
        curve_b.keyframes[1].in_tangent = BezierHandle::new(-0.66, -8.0);

        let Some(val_a) = curve_sample(&curve_a, 0.5) else {
            unreachable!()
        };
        let Some(val_b) = curve_sample(&curve_b, 0.5) else {
            unreachable!()
        };

        assert!(
            (val_a - val_b).abs() > 0.5,
            "Different tangent handles should produce different curves: a={}, b={}",
            val_a,
            val_b
        );
    }

    #[test]
    fn test_auto_tangent_recalculate_at_specific_keyframe() {
        let mut curve = make_curve_with_keyframes(&[(0.0, 0.0), (1.0, 5.0), (2.0, 0.0)]);
        for kf in &mut curve.keyframes {
            kf.interpolation = InterpolationType::Bezier;
        }

        let mid_id = curve.keyframes[1].id;
        curve_recalculate_auto_tangent_at(&mut curve, mid_id);

        let mid = &curve.keyframes[1];
        assert!(
            mid.in_tangent.time_offset < 0.0,
            "In tangent should point left"
        );
        assert!(
            mid.out_tangent.time_offset > 0.0,
            "Out tangent should point right"
        );
    }

    #[test]
    fn test_recalculate_auto_tangent_weighted_preserves_length() {
        let mut curve = make_curve_with_keyframes(&[(0.0, 0.0), (1.0, 5.0), (2.0, 10.0)]);
        for kf in &mut curve.keyframes {
            kf.interpolation = InterpolationType::Bezier;
        }

        curve.keyframes[1].weight_mode = TangentWeightMode::Weighted;
        curve.keyframes[1].in_tangent = BezierHandle::new(-0.3, 0.0);
        curve.keyframes[1].out_tangent = BezierHandle::new(0.3, 0.0);

        let in_len_before = compute_handle_length(&curve.keyframes[1].in_tangent);
        let out_len_before = compute_handle_length(&curve.keyframes[1].out_tangent);

        let mid_id = curve.keyframes[1].id;
        curve_recalculate_auto_tangent_at(&mut curve, mid_id);

        let in_len_after = compute_handle_length(&curve.keyframes[1].in_tangent);
        let out_len_after = compute_handle_length(&curve.keyframes[1].out_tangent);

        assert!(
            (in_len_after - in_len_before).abs() < 1e-4,
            "Weighted auto tangent should preserve in handle length"
        );
        assert!(
            (out_len_after - out_len_before).abs() < 1e-4,
            "Weighted auto tangent should preserve out handle length"
        );
    }

    #[test]
    fn preserving_shape_bezier_curve_unchanged() {
        let mut curve = make_curve_with_keyframes(&[(0.0, 0.0), (2.0, 10.0)]);
        for kf in &mut curve.keyframes {
            kf.interpolation = InterpolationType::Bezier;
        }
        curve.keyframes[0].out_tangent = BezierHandle::new(0.5, 3.0);
        curve.keyframes[1].in_tangent = BezierHandle::new(-0.5, -3.0);

        let samples_before: Vec<f32> = (0..=20)
            .map(|i| {
                let t = i as f32 * 2.0 / 20.0;
                let Some(v) = curve_sample(&curve, t) else {
                    unreachable!()
                };
                v
            })
            .collect();

        curve_insert_keyframe_preserving_shape(&mut curve, 1.0);

        for (i, &sample_before) in samples_before.iter().enumerate() {
            let t = i as f32 * 2.0 / 20.0;
            let Some(sample_after) = curve_sample(&curve, t) else {
                unreachable!()
            };
            assert!(
                (sample_before - sample_after).abs() < 1e-4,
                "Bezier curve shape changed at t={}: before={}, after={}",
                t,
                sample_before,
                sample_after
            );
        }
    }

    #[test]
    fn preserving_shape_linear_midpoint() {
        let mut curve = make_curve_with_keyframes(&[(0.0, 0.0), (2.0, 10.0)]);
        for kf in &mut curve.keyframes {
            kf.interpolation = InterpolationType::Linear;
        }

        let Some(id) = curve_insert_keyframe_preserving_shape(&mut curve, 1.0) else {
            unreachable!()
        };
        let Some(inserted) = curve.get_keyframe(id) else {
            unreachable!()
        };
        assert!((inserted.value - 5.0).abs() < 1e-4);
    }

    #[test]
    fn preserving_shape_stepped() {
        let mut curve = make_curve_with_keyframes(&[(0.0, 3.0), (2.0, 7.0)]);
        for kf in &mut curve.keyframes {
            kf.interpolation = InterpolationType::Stepped;
        }

        let Some(id) = curve_insert_keyframe_preserving_shape(&mut curve, 1.0) else {
            unreachable!()
        };
        let Some(inserted) = curve.get_keyframe(id) else {
            unreachable!()
        };
        assert!((inserted.value - 3.0).abs() < 1e-4);
    }

    #[test]
    fn preserving_shape_before_first() {
        let mut curve = make_curve_with_keyframes(&[(1.0, 5.0), (2.0, 10.0)]);

        let Some(id) = curve_insert_keyframe_preserving_shape(&mut curve, 0.0) else {
            unreachable!()
        };
        let Some(inserted) = curve.get_keyframe(id) else {
            unreachable!()
        };
        assert!((inserted.value - 5.0).abs() < 1e-4);
        assert!((inserted.time - 0.0).abs() < 1e-4);
    }

    #[test]
    fn preserving_shape_after_last() {
        let mut curve = make_curve_with_keyframes(&[(0.0, 5.0), (1.0, 10.0)]);

        let Some(id) = curve_insert_keyframe_preserving_shape(&mut curve, 2.0) else {
            unreachable!()
        };
        let Some(inserted) = curve.get_keyframe(id) else {
            unreachable!()
        };
        assert!((inserted.value - 10.0).abs() < 1e-4);
        assert!((inserted.time - 2.0).abs() < 1e-4);
    }

    #[test]
    fn preserving_shape_same_time_returns_none() {
        let mut curve = make_curve_with_keyframes(&[(0.0, 0.0), (1.0, 10.0)]);

        let result = curve_insert_keyframe_preserving_shape(&mut curve, 0.0);
        assert!(result.is_none());
        assert_eq!(curve.keyframe_count(), 2);
    }

    #[test]
    fn preserving_shape_bezier_20_points() {
        let mut curve = make_curve_with_keyframes(&[(0.0, 0.0), (4.0, 8.0)]);
        for kf in &mut curve.keyframes {
            kf.interpolation = InterpolationType::Bezier;
        }
        curve.keyframes[0].out_tangent = BezierHandle::new(1.0, 2.0);
        curve.keyframes[1].in_tangent = BezierHandle::new(-1.0, -2.0);

        let samples_before: Vec<f32> = (0..=20)
            .map(|i| {
                let t = i as f32 * 4.0 / 20.0;
                let Some(v) = curve_sample(&curve, t) else {
                    unreachable!()
                };
                v
            })
            .collect();

        curve_insert_keyframe_preserving_shape(&mut curve, 2.0);

        for i in 0..=20 {
            let t = i as f32 * 4.0 / 20.0;
            let Some(sample_after) = curve_sample(&curve, t) else {
                unreachable!()
            };
            assert!(
                (samples_before[i] - sample_after).abs() < 1e-4,
                "Bezier curve shape changed at t={}: before={}, after={}",
                t,
                samples_before[i],
                sample_after
            );
        }
    }

    #[test]
    fn preserving_shape_keeps_mixed_linear_bezier_segments() {
        let mut curve = PropertyCurve::new(1, PropertyType::TranslationX);

        let id0 = curve_add_keyframe(&mut curve, 0.0, 0.0);
        let id1 = curve_add_keyframe(&mut curve, 1.0, 4.0);
        let id2 = curve_add_keyframe(&mut curve, 2.0, 1.0);

        curve.keyframes[0].interpolation = InterpolationType::Linear;
        curve.keyframes[2].interpolation = InterpolationType::Linear;

        curve.keyframes[1].interpolation = InterpolationType::Bezier;
        if let Some(kf) = curve.get_keyframe_mut(id1) {
            kf.in_tangent = BezierHandle::new(-0.2, 1.0);
            kf.out_tangent = BezierHandle::new(0.4, -0.5);
        }

        let samples_before: Vec<f32> = (0..40)
            .map(|i| {
                let t = i as f32 * 2.0 / 40.0;
                let Some(v) = curve_sample(&curve, t) else {
                    unreachable!()
                };
                v
            })
            .collect();

        curve_insert_keyframe_preserving_shape(&mut curve, 0.5);
        curve_insert_keyframe_preserving_shape(&mut curve, 1.5);

        for (i, &sample_before) in samples_before.iter().enumerate() {
            let t = i as f32 * 2.0 / 40.0;
            let Some(sample_after) = curve_sample(&curve, t) else {
                unreachable!()
            };
            assert!(
                (sample_before - sample_after).abs() < 1e-4,
                "Mixed curve shape changed at t={}: before={}, after={}",
                t,
                sample_before,
                sample_after
            );
        }
    }

    #[test]
    fn extrapolation_constant_default() {
        let curve = make_curve_with_keyframes(&[(0.0, 5.0), (1.0, 10.0)]);
        assert!(curve.pre_extrapolation == CurveExtrapolation::Constant);
        assert!(curve.post_extrapolation == CurveExtrapolation::Constant);

        let v = curve_sample(&curve, -1.0).unwrap();
        assert!((v - 5.0).abs() < 1e-4);

        let v = curve_sample(&curve, 2.0).unwrap();
        assert!((v - 10.0).abs() < 1e-4);
    }

    #[test]
    fn extrapolation_linear_pre() {
        let mut curve = make_curve_with_keyframes(&[(0.0, 5.0), (1.0, 10.0)]);
        curve.pre_extrapolation = CurveExtrapolation::Linear;

        let v = curve_sample(&curve, -1.0).unwrap();
        assert!((v - 0.0).abs() < 1e-4);

        let v = curve_sample(&curve, -0.5).unwrap();
        assert!((v - 2.5).abs() < 1e-4);
    }

    #[test]
    fn extrapolation_linear_post() {
        let mut curve = make_curve_with_keyframes(&[(0.0, 5.0), (1.0, 10.0)]);
        curve.post_extrapolation = CurveExtrapolation::Linear;

        let v = curve_sample(&curve, 2.0).unwrap();
        assert!((v - 15.0).abs() < 1e-4);

        let v = curve_sample(&curve, 1.5).unwrap();
        assert!((v - 12.5).abs() < 1e-4);
    }

    #[test]
    fn extrapolation_linear_bezier_handle() {
        let mut curve = PropertyCurve::new(1, PropertyType::TranslationX);
        let mut kf0 = EditableKeyframe::new(1, 0.0, 5.0);
        kf0.interpolation = InterpolationType::Bezier;
        kf0.out_tangent = BezierHandle::new(1.0, 3.0);
        curve.keyframes.push(kf0);
        curve.keyframes.push(EditableKeyframe::new(2, 1.0, 10.0));
        curve.pre_extrapolation = CurveExtrapolation::Linear;

        let v = curve_sample(&curve, -1.0).unwrap();
        assert!((v - 2.0).abs() < 1e-4);

        let mut kf_last = EditableKeyframe::new(3, 2.0, 15.0);
        kf_last.interpolation = InterpolationType::Bezier;
        kf_last.in_tangent = BezierHandle::new(-1.0, 2.0);
        curve.keyframes.push(kf_last);
        curve.post_extrapolation = CurveExtrapolation::Linear;

        let v = curve_sample(&curve, 3.0).unwrap();
        assert!((v - 13.0).abs() < 1e-4);
    }

    #[test]
    fn extrapolation_cycle_pre() {
        let mut curve = make_curve_with_keyframes(&[(0.0, 5.0), (1.0, 10.0)]);
        curve.pre_extrapolation = CurveExtrapolation::Cycle;

        let v = curve_sample(&curve, -1.0).unwrap();
        assert!((v - 5.0).abs() < 1e-4);

        let v = curve_sample(&curve, -0.5).unwrap();
        assert!((v - 7.5).abs() < 1e-4);
    }

    #[test]
    fn extrapolation_cycle_post() {
        let mut curve = make_curve_with_keyframes(&[(0.0, 5.0), (1.0, 10.0)]);
        curve.post_extrapolation = CurveExtrapolation::Cycle;

        let v = curve_sample(&curve, 2.0).unwrap();
        assert!((v - 5.0).abs() < 1e-4);

        let v = curve_sample(&curve, 1.5).unwrap();
        assert!((v - 7.5).abs() < 1e-4);
    }

    #[test]
    fn extrapolation_cycle_with_offset_pre() {
        let mut curve = make_curve_with_keyframes(&[(0.0, 5.0), (1.0, 10.0)]);
        curve.pre_extrapolation = CurveExtrapolation::CycleWithOffset;

        // value at mapped=0 is 5, so 5 + (-5) = 0
        let v = curve_sample(&curve, -1.0).unwrap();
        assert!((v - 0.0).abs() < 1e-4);

        // value at mapped=0.5 is 7.5, so 7.5 + (-5) = 2.5
        let v = curve_sample(&curve, -0.5).unwrap();
        assert!((v - 2.5).abs() < 1e-4);
    }

    #[test]
    fn extrapolation_cycle_with_offset_post() {
        let mut curve = make_curve_with_keyframes(&[(0.0, 5.0), (1.0, 10.0)]);
        curve.post_extrapolation = CurveExtrapolation::CycleWithOffset;

        // value at mapped=0 is 5, so 5 + 10 = 15
        let v = curve_sample(&curve, 2.0).unwrap();
        assert!((v - 15.0).abs() < 1e-4);

        // value at mapped=0.5 is 7.5, so 7.5 + 5 = 12.5
        let v = curve_sample(&curve, 1.5).unwrap();
        assert!((v - 12.5).abs() < 1e-4);
    }

    #[test]
    fn extrapolation_old_format_ron_deserializes_as_constant() {
        // Old format RON string without pre/post_extrapolation fields
        let ron = "(
            id: 1,
            property_type: TranslationX,
            keyframes: [],
            next_keyframe_id: 1,
        )";
        let curve: PropertyCurve = ron::from_str(ron).unwrap();
        assert!(curve.pre_extrapolation == CurveExtrapolation::Constant);
        assert!(curve.post_extrapolation == CurveExtrapolation::Constant);
    }

    #[test]
    fn extrapolation_linear_single_keyframe() {
        let mut curve = PropertyCurve::new(1, PropertyType::TranslationX);
        curve.pre_extrapolation = CurveExtrapolation::Linear;
        curve.post_extrapolation = CurveExtrapolation::Linear;
        curve.keyframes.push(EditableKeyframe::new(1, 1.0, 5.0));

        let v = curve_sample(&curve, -1.0).unwrap();
        assert!((v - 5.0).abs() < 1e-4);

        let v = curve_sample(&curve, 3.0).unwrap();
        assert!((v - 5.0).abs() < 1e-4);

        let v = curve_sample(&curve, 1.0).unwrap();
        assert!((v - 5.0).abs() < 1e-4);
    }
}
