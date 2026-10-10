use crate::animation::editable::{
    clip_add_keyframe, clip_recalculate_duration, curve_add_keyframe,
    curve_insert_keyframe_preserving_shape, curve_remove_keyframe, curve_set_keyframe_time,
    CurveExtrapolation, EditableAnimationClip, KeyframeId, PropertyCurve, PropertyType,
    SourceClipId,
};
use crate::ecs::resource::{ClipLibrary, CurveTrackRef, SelectedKeyframe};

#[derive(Clone, Copy, Debug)]
pub enum ExtrapolationEnd {
    Pre,
    Post,
    Both,
}

pub fn resolve_curve_mut(
    clip: &mut EditableAnimationClip,
    track: CurveTrackRef,
    property_type: PropertyType,
) -> Option<&mut PropertyCurve> {
    match track {
        CurveTrackRef::Bone(bone_id) => clip
            .tracks
            .get_mut(&bone_id)
            .map(|t| t.get_curve_mut(property_type)),
        CurveTrackRef::Scalar => clip.get_scalar_curve_mut(property_type),
        CurveTrackRef::Morph(i) => clip.morph_tracks.get_mut(i).map(|mt| &mut mt.curve),
    }
}

/// Runs `edit` on one curve of the library's clip; false when the clip or curve is missing.
pub fn edit_curve(
    clip_library: &mut ClipLibrary,
    clip_id: SourceClipId,
    track: CurveTrackRef,
    property_type: PropertyType,
    edit: impl FnOnce(&mut PropertyCurve),
) -> bool {
    let Some(clip) = clip_library.get_mut(clip_id) else {
        return false;
    };
    let Some(curve) = resolve_curve_mut(clip, track, property_type) else {
        return false;
    };
    edit(curve);
    true
}

/// Scalar curves are keyed by Custom codes; a bone property type there would create a curve nothing samples.
pub fn add_keyframe(
    clip: &mut EditableAnimationClip,
    track: CurveTrackRef,
    property_type: PropertyType,
    time: f32,
    value: f32,
) -> bool {
    match track {
        CurveTrackRef::Bone(bone_id) => {
            clip_add_keyframe(clip, bone_id, property_type, time, value);
        }
        CurveTrackRef::Scalar => {
            if !matches!(property_type, PropertyType::Custom(_)) {
                return false;
            }
            let curve = clip.get_or_add_scalar_curve(property_type);
            curve_add_keyframe(curve, time, value);
        }
        CurveTrackRef::Morph(i) => {
            let Some(morph_track) = clip.morph_tracks.get_mut(i) else {
                return false;
            };
            curve_add_keyframe(&mut morph_track.curve, time, value);
        }
    }
    clip_recalculate_duration(clip);
    true
}

pub fn insert_keyframe_preserving_shape(
    clip: &mut EditableAnimationClip,
    track: CurveTrackRef,
    property_type: PropertyType,
    time: f32,
) -> bool {
    let Some(curve) = resolve_curve_mut(clip, track, property_type) else {
        return false;
    };
    if curve_insert_keyframe_preserving_shape(curve, time).is_none() {
        return false;
    }
    clip_recalculate_duration(clip);
    true
}

pub fn shift_keyframes<'a>(
    clip: &mut EditableAnimationClip,
    selected: impl IntoIterator<Item = &'a SelectedKeyframe>,
    time_delta: f32,
) {
    for sel in selected {
        let Some(curve) = resolve_curve_mut(clip, sel.track, sel.property_type) else {
            continue;
        };
        if let Some(kf) = curve.get_keyframe_mut(sel.keyframe_id) {
            kf.time = (kf.time + time_delta).max(0.0);
        }
    }
    clip_recalculate_duration(clip);
}

pub fn remove_keyframes<'a>(
    clip: &mut EditableAnimationClip,
    selected: impl IntoIterator<Item = &'a SelectedKeyframe>,
) {
    for sel in selected {
        if let Some(curve) = resolve_curve_mut(clip, sel.track, sel.property_type) {
            curve_remove_keyframe(curve, sel.keyframe_id);
        }
    }
    clip_recalculate_duration(clip);
}

pub fn move_keyframe(
    clip: &mut EditableAnimationClip,
    track: CurveTrackRef,
    property_type: PropertyType,
    keyframe_id: KeyframeId,
    new_time: f32,
    new_value: f32,
) {
    if let Some(curve) = resolve_curve_mut(clip, track, property_type) {
        curve_set_keyframe_time(curve, keyframe_id, new_time);
        curve.set_keyframe_value(keyframe_id, new_value);
    }
    clip_recalculate_duration(clip);
}

pub fn remove_keyframe(
    clip: &mut EditableAnimationClip,
    track: CurveTrackRef,
    property_type: PropertyType,
    keyframe_id: KeyframeId,
) {
    if let Some(curve) = resolve_curve_mut(clip, track, property_type) {
        curve_remove_keyframe(curve, keyframe_id);
    }
    clip_recalculate_duration(clip);
}

pub fn set_extrapolation(
    curve: &mut PropertyCurve,
    end: ExtrapolationEnd,
    mode: CurveExtrapolation,
) {
    match end {
        ExtrapolationEnd::Pre => curve.pre_extrapolation = mode,
        ExtrapolationEnd::Post => curve.post_extrapolation = mode,
        ExtrapolationEnd::Both => {
            curve.pre_extrapolation = mode;
            curve.post_extrapolation = mode;
        }
    }
}
