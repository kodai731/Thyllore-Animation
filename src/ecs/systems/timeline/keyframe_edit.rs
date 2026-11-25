use crate::ecs::resource::{ClipLibrary, CurveTrackRef, SelectedKeyframe};
use thyllore_anim_core::editable::{
    clip_add_keyframe, clip_recalculate_duration, curve_add_keyframe,
    curve_insert_keyframe_preserving_shape, curve_remove_keyframe, curve_set_keyframe_time,
    CurveExtrapolation, EditableAnimationClip, KeyframeId, PropertyCurve, PropertyType,
    SourceClipId,
};

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

/// Returns what `edit` returns, false for a missing clip; an edit that changed nothing leaves no trace on the clip.
pub fn edit_clip(
    clip_library: &mut ClipLibrary,
    clip_id: SourceClipId,
    edit: impl FnOnce(&mut EditableAnimationClip) -> bool,
) -> bool {
    let origin_before = clip_library.origin(clip_id);
    let was_dirty_before = clip_library.is_dirty(clip_id);

    let was_edited = clip_library.get_mut(clip_id).is_some_and(edit);
    if !was_edited {
        clip_library.set_origin(clip_id, origin_before);
    }
    if !was_edited && !was_dirty_before {
        clip_library.mark_clean(clip_id);
    }
    was_edited
}

/// Runs `edit` on one curve of the library's clip; false when the clip or curve is missing.
pub fn edit_curve(
    clip_library: &mut ClipLibrary,
    clip_id: SourceClipId,
    track: CurveTrackRef,
    property_type: PropertyType,
    edit: impl FnOnce(&mut PropertyCurve),
) -> bool {
    edit_clip(clip_library, clip_id, |clip| {
        let Some(curve) = resolve_curve_mut(clip, track, property_type) else {
            return false;
        };
        edit(curve);
        true
    })
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

/// True when a selected keyframe exists in the clip and was shifted.
pub fn shift_keyframes<'a>(
    clip: &mut EditableAnimationClip,
    selected: impl IntoIterator<Item = &'a SelectedKeyframe>,
    time_delta: f32,
) -> bool {
    let mut was_shifted = false;
    for sel in selected {
        let Some(curve) = resolve_curve_mut(clip, sel.track, sel.property_type) else {
            continue;
        };
        if let Some(kf) = curve.get_keyframe_mut(sel.keyframe_id) {
            kf.time = (kf.time + time_delta).max(0.0);
            was_shifted = true;
        }
    }
    if was_shifted {
        clip_recalculate_duration(clip);
    }

    was_shifted
}

/// True when a selected keyframe exists in the clip and was removed.
pub fn remove_keyframes<'a>(
    clip: &mut EditableAnimationClip,
    selected: impl IntoIterator<Item = &'a SelectedKeyframe>,
) -> bool {
    let mut was_removed = false;
    for sel in selected {
        if let Some(curve) = resolve_curve_mut(clip, sel.track, sel.property_type) {
            was_removed |= curve_remove_keyframe(curve, sel.keyframe_id);
        }
    }
    if was_removed {
        clip_recalculate_duration(clip);
    }

    was_removed
}

/// False when the clip has no such keyframe.
pub fn move_keyframe(
    clip: &mut EditableAnimationClip,
    track: CurveTrackRef,
    property_type: PropertyType,
    keyframe_id: KeyframeId,
    new_time: f32,
    new_value: f32,
) -> bool {
    let Some(curve) = resolve_curve_mut(clip, track, property_type) else {
        return false;
    };
    if curve.get_keyframe(keyframe_id).is_none() {
        return false;
    }

    curve_set_keyframe_time(curve, keyframe_id, new_time);
    curve.set_keyframe_value(keyframe_id, new_value);
    clip_recalculate_duration(clip);
    true
}

/// False when the clip has no such keyframe.
pub fn remove_keyframe(
    clip: &mut EditableAnimationClip,
    track: CurveTrackRef,
    property_type: PropertyType,
    keyframe_id: KeyframeId,
) -> bool {
    let Some(curve) = resolve_curve_mut(clip, track, property_type) else {
        return false;
    };
    if !curve_remove_keyframe(curve, keyframe_id) {
        return false;
    }

    clip_recalculate_duration(clip);
    true
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
