use imgui::Condition;

use crate::animation::editable::{EditableAnimationClip, EditableClipManager, PropertyType};
use crate::animation::BoneId;
use crate::ecs::events::{UIEvent, UIEventQueue};
use crate::ecs::resource::TimelineState;

const TRACK_LABEL_WIDTH: f32 = 120.0;
const MAX_VISIBLE_TRACKS: usize = 15;

pub fn build_timeline_window(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    state: &TimelineState,
    clip_manager: &EditableClipManager,
) {
    let display_size = ui.io().display_size;
    let timeline_height = 200.0;
    let hierarchy_width = 250.0;
    let timeline_y = display_size[1] - timeline_height - 250.0;
    let timeline_width = display_size[0] - hierarchy_width;

    ui.window("Timeline")
        .position([hierarchy_width, timeline_y], Condition::Always)
        .size([timeline_width, timeline_height], Condition::Always)
        .resizable(false)
        .movable(false)
        .collapsible(false)
        .build(|| {
            build_transport_controls(ui, ui_events, state, clip_manager);
            ui.separator();
            build_timeline_content(ui, ui_events, state, clip_manager);
        });
}

fn build_transport_controls(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    state: &TimelineState,
    clip_manager: &EditableClipManager,
) {
    if state.playing {
        if ui.button("||") {
            ui_events.send(UIEvent::TimelinePause);
        }
    } else if ui.button(">") {
        ui_events.send(UIEvent::TimelinePlay);
    }

    ui.same_line();
    if ui.button("[]") {
        ui_events.send(UIEvent::TimelineStop);
    }

    ui.same_line();
    let mut looping = state.looping;
    if ui.checkbox("Loop", &mut looping) {
        ui_events.send(UIEvent::TimelineToggleLoop);
    }

    ui.same_line();
    let current_clip = state
        .current_clip_id
        .and_then(|id| clip_manager.get(id));

    let duration = current_clip.map(|c| c.duration).unwrap_or(0.0);

    ui.text(format!(
        "Time: {:.2}s / {:.2}s",
        state.current_time, duration
    ));

    ui.same_line();
    ui.text(format!("Speed: {:.1}x", state.speed));

    ui.same_line();
    if ui.button("-") {
        ui_events.send(UIEvent::TimelineZoomOut);
    }
    ui.same_line();
    if ui.button("+") {
        ui_events.send(UIEvent::TimelineZoomIn);
    }
    ui.same_line();
    ui.text(format!("Zoom: {:.1}x", state.zoom_level));

    build_clip_selector(ui, ui_events, state, clip_manager);
}

fn build_clip_selector(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    state: &TimelineState,
    clip_manager: &EditableClipManager,
) {
    let clip_names = clip_manager.clip_names();

    if clip_names.is_empty() {
        ui.text("No clips available");
        return;
    }

    let current_name = state
        .current_clip_id
        .and_then(|id| clip_manager.get(id))
        .map(|c| c.name.as_str())
        .unwrap_or("Select Clip");

    ui.same_line();
    ui.set_next_item_width(150.0);

    if let Some(_token) = ui.begin_combo("##clip_select", current_name) {
        for (id, name) in &clip_names {
            let is_selected = state.current_clip_id == Some(*id);
            if ui.selectable_config(&name).selected(is_selected).build() {
                ui_events.send(UIEvent::TimelineSelectClip(*id));
            }
        }
    }
}

fn build_timeline_content(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    state: &TimelineState,
    clip_manager: &EditableClipManager,
) {
    let current_clip = match state.current_clip_id.and_then(|id| clip_manager.get(id)) {
        Some(clip) => clip,
        None => {
            ui.text("Select a clip to edit");
            return;
        }
    };

    let content_region = ui.content_region_avail();
    let track_area_width = content_region[0] - TRACK_LABEL_WIDTH;

    ui.child_window("timeline_content")
        .size(content_region)
        .horizontal_scrollbar(true)
        .build(|| {
            build_time_ruler(ui, state, current_clip, track_area_width);
            build_tracks(ui, ui_events, state, current_clip, track_area_width);
            build_playhead(ui, state, current_clip);
        });
}

fn build_time_ruler(
    ui: &imgui::Ui,
    state: &TimelineState,
    clip: &EditableAnimationClip,
    _track_area_width: f32,
) {
    let duration = clip.duration.max(0.1);
    ui.text(format!(
        "Duration: {:.2}s | Time: {:.2}s | Zoom: {:.1}x",
        duration, state.current_time, state.zoom_level
    ));
}

fn build_tracks(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    state: &TimelineState,
    clip: &EditableAnimationClip,
    track_area_width: f32,
) {
    let mut sorted_bone_ids: Vec<BoneId> = clip.tracks.keys().copied().collect();
    sorted_bone_ids.sort();

    let total_tracks = sorted_bone_ids.len();
    let visible_tracks: Vec<_> = sorted_bone_ids.into_iter().take(MAX_VISIBLE_TRACKS).collect();

    if total_tracks > MAX_VISIBLE_TRACKS {
        ui.text_colored(
            [1.0, 0.7, 0.3, 1.0],
            &format!("Showing {}/{} tracks", MAX_VISIBLE_TRACKS, total_tracks),
        );
    }

    for bone_id in visible_tracks {
        if let Some(track) = clip.tracks.get(&bone_id) {
            let is_expanded = state.is_track_expanded(bone_id);

            build_track_header(ui, ui_events, bone_id, &track.bone_name, is_expanded, track.has_any_keyframes());

            let cursor_pos = ui.cursor_screen_pos();
            let track_start_x = cursor_pos[0] + TRACK_LABEL_WIDTH;

            build_track_keyframes_summary(ui, state, track, track_start_x, track_area_width);

            if is_expanded {
                build_expanded_track_properties(ui, state, track, track_start_x, track_area_width);
            }
        }
    }
}

fn build_track_header(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    bone_id: BoneId,
    bone_name: &str,
    is_expanded: bool,
    has_keyframes: bool,
) {
    let expand_char = if is_expanded { "v" } else { ">" };

    if ui.small_button(&format!("{}##{}", expand_char, bone_id)) {
        ui_events.send(UIEvent::TimelineToggleTrack(bone_id));
    }

    ui.same_line();

    let display_name = if bone_name.len() > 12 {
        format!("{}...", &bone_name[..10])
    } else {
        bone_name.to_string()
    };

    let color = if has_keyframes {
        [1.0, 1.0, 1.0, 1.0]
    } else {
        [0.5, 0.5, 0.5, 1.0]
    };

    ui.text_colored(color, &display_name);
}

fn build_track_keyframes_summary(
    ui: &imgui::Ui,
    _state: &TimelineState,
    track: &crate::animation::editable::BoneTrack,
    _track_start_x: f32,
    _track_area_width: f32,
) {
    let keyframe_count = track.total_keyframe_count();
    ui.same_line();
    ui.text_colored([0.6, 0.6, 0.6, 1.0], &format!("[{} kf]", keyframe_count));
}

fn build_expanded_track_properties(
    ui: &imgui::Ui,
    state: &TimelineState,
    track: &crate::animation::editable::BoneTrack,
    _track_start_x: f32,
    _track_area_width: f32,
) {
    let properties = [
        (PropertyType::TranslationX, &track.translation_x, state.show_translation),
        (PropertyType::TranslationY, &track.translation_y, state.show_translation),
        (PropertyType::TranslationZ, &track.translation_z, state.show_translation),
        (PropertyType::RotationX, &track.rotation_x, state.show_rotation),
        (PropertyType::RotationY, &track.rotation_y, state.show_rotation),
        (PropertyType::RotationZ, &track.rotation_z, state.show_rotation),
        (PropertyType::RotationW, &track.rotation_w, state.show_rotation),
        (PropertyType::ScaleX, &track.scale_x, state.show_scale),
        (PropertyType::ScaleY, &track.scale_y, state.show_scale),
        (PropertyType::ScaleZ, &track.scale_z, state.show_scale),
    ];

    for (prop_type, curve, visible) in properties {
        if !visible || curve.is_empty() {
            continue;
        }

        let color = get_property_color(prop_type);
        ui.indent();
        ui.text_colored(color, &format!("{}: {} keyframes", prop_type.short_name(), curve.keyframe_count()));
        ui.unindent();
    }
}

fn build_playhead(
    ui: &imgui::Ui,
    state: &TimelineState,
    clip: &EditableAnimationClip,
) {
    if state.current_time >= 0.0 && state.current_time <= clip.duration {
        let progress = if clip.duration > 0.0 {
            (state.current_time / clip.duration * 100.0) as i32
        } else {
            0
        };
        ui.text_colored([1.0, 0.3, 0.3, 1.0], &format!("Playhead: {}%", progress));
    }
}

fn get_property_color(property_type: PropertyType) -> [f32; 4] {
    match property_type {
        PropertyType::TranslationX => [1.0, 0.3, 0.3, 1.0],
        PropertyType::TranslationY => [0.3, 1.0, 0.3, 1.0],
        PropertyType::TranslationZ => [0.3, 0.3, 1.0, 1.0],
        PropertyType::RotationX => [1.0, 0.5, 0.5, 1.0],
        PropertyType::RotationY => [0.5, 1.0, 0.5, 1.0],
        PropertyType::RotationZ => [0.5, 0.5, 1.0, 1.0],
        PropertyType::RotationW => [0.8, 0.8, 0.8, 1.0],
        PropertyType::ScaleX => [1.0, 0.7, 0.3, 1.0],
        PropertyType::ScaleY => [0.7, 1.0, 0.3, 1.0],
        PropertyType::ScaleZ => [0.3, 0.7, 1.0, 1.0],
    }
}
