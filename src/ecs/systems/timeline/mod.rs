mod bone_set_key;
pub mod clip_instance;
pub mod keyframe_edit;
mod playback;
mod scene_record;
pub mod tangent_edit;
#[cfg(test)]
pub(crate) mod test_support;
mod view;

pub use bone_set_key::process_bone_set_key;
pub use playback::{
    schedule_extent_seconds, timeline_effective_duration, timeline_update,
    TIMELINE_FALLBACK_DURATION_SECONDS,
};
pub use scene_record::TimelineSceneRecord;
pub use view::{
    clip_drag_preview_times, timeline_apply_fit_zoom, timeline_apply_selection,
    timeline_select_clip, timeline_set_keyframe_selection, timeline_toggle_track_expanded,
    timeline_zoom_in, timeline_zoom_out,
};
