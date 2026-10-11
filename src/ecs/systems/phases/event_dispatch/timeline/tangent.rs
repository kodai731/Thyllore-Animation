use crate::ecs::resource::{ClipLibrary, TimelineState};
use crate::ecs::systems::timeline::keyframe_edit::edit_curve;
use crate::ecs::systems::timeline::tangent_edit::{
    set_manual_tangents, set_tangent_continuity, set_tangent_type, set_tangent_weight_mode,
};

use super::TimelineEvent;

/// Applies the tangent and interpolation edits of the batch; true when a curve changed.
pub(super) fn apply_tangent_edits(
    events: &[TimelineEvent],
    timeline_state: &TimelineState,
    clip_library: &mut ClipLibrary,
) -> bool {
    let Some(clip_id) = timeline_state.current_clip_id else {
        return false;
    };

    let mut clip_modified = false;
    for event in events {
        clip_modified |= match event {
            TimelineEvent::SetKeyframeInterpolation {
                track,
                property_type,
                keyframe_id,
                interpolation,
            } => edit_curve(clip_library, clip_id, *track, *property_type, |curve| {
                curve.set_keyframe_interpolation(*keyframe_id, *interpolation)
            }),

            TimelineEvent::SetKeyframeTangent {
                track,
                property_type,
                keyframe_id,
                in_tangent,
                out_tangent,
            } => edit_curve(clip_library, clip_id, *track, *property_type, |curve| {
                set_manual_tangents(curve, *keyframe_id, in_tangent.clone(), out_tangent.clone())
            }),

            TimelineEvent::SetTangentType {
                track,
                property_type,
                keyframe_id,
                tangent_type,
            } => edit_curve(clip_library, clip_id, *track, *property_type, |curve| {
                set_tangent_type(curve, *keyframe_id, *tangent_type)
            }),

            TimelineEvent::SetTangentWeightMode {
                track,
                property_type,
                keyframe_id,
                weight_mode,
            } => edit_curve(clip_library, clip_id, *track, *property_type, |curve| {
                set_tangent_weight_mode(curve, *keyframe_id, *weight_mode)
            }),

            TimelineEvent::SetTangentContinuity {
                track,
                property_type,
                keyframe_id,
                continuity,
            } => edit_curve(clip_library, clip_id, *track, *property_type, |curve| {
                set_tangent_continuity(curve, *keyframe_id, *continuity)
            }),

            _ => false,
        };
    }

    clip_modified
}
