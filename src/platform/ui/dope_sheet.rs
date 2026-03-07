use std::collections::HashSet;

use super::timeline_window::ruler_padding;
use crate::animation::editable::{EditableAnimationClip, PropertyType};
use crate::animation::BoneId;
use crate::ecs::events::{UIEvent, UIEventQueue};
use crate::ecs::resource::{
    DopeSheetBoxSelect, DopeSheetInteraction, DopeSheetKeyframeDrag, DopeSheetKeyframeHit,
    SelectedKeyframe, SelectionModifier, TimelineState,
};

const DOPE_ROW_HEIGHT: f32 = 22.0;
const DOPE_SUB_ROW_HEIGHT: f32 = 18.0;
const DOPE_LABEL_WIDTH: f32 = 150.0;
const DOPE_PIXELS_PER_SECOND: f32 = 80.0;
const KEYFRAME_DIAMOND_SIZE: f32 = 5.0;
const RULER_HEIGHT: f32 = 24.0;
const MAX_VISIBLE_TRACKS: usize = 10;
const HIT_RADIUS: f32 = 6.0;

const PROPERTY_COLORS: [[f32; 4]; 9] = [
    [0.9, 0.3, 0.3, 1.0],
    [0.3, 0.9, 0.3, 1.0],
    [0.3, 0.3, 0.9, 1.0],
    [0.9, 0.6, 0.3, 1.0],
    [0.3, 0.9, 0.6, 1.0],
    [0.6, 0.3, 0.9, 1.0],
    [0.9, 0.9, 0.3, 1.0],
    [0.3, 0.9, 0.9, 1.0],
    [0.9, 0.3, 0.9, 1.0],
];

const BOX_SELECT_FILL: [f32; 4] = [0.3, 0.5, 0.9, 0.15];
const BOX_SELECT_BORDER: [f32; 4] = [0.4, 0.6, 1.0, 0.7];
const DRAG_PREVIEW_ALPHA: f32 = 0.5;

pub fn build_dope_sheet(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    timeline_state: &mut TimelineState,
    clip: &EditableAnimationClip,
    content_width: f32,
    content_height: f32,
) {
    let pixels_per_second = DOPE_PIXELS_PER_SECOND * timeline_state.zoom_level;
    let display_duration = clip.duration + ruler_padding(clip.duration);
    let sheet_width = (display_duration * pixels_per_second).max(content_width - DOPE_LABEL_WIDTH);

    timeline_state.dope_sheet_keyframe_hits.clear();

    ui.child_window("dope_sheet_area")
        .size([content_width, content_height])
        .horizontal_scrollbar(true)
        .build(|| {
            draw_dope_sheet_ruler(
                ui,
                timeline_state,
                sheet_width,
                pixels_per_second,
                display_duration,
            );

            draw_summary_row(ui, timeline_state, clip, sheet_width, pixels_per_second);

            draw_bone_rows(
                ui,
                ui_events,
                timeline_state,
                clip,
                sheet_width,
                pixels_per_second,
            );

            handle_dope_sheet_interaction(ui, ui_events, timeline_state, pixels_per_second);

            draw_interaction_overlays(ui, timeline_state, pixels_per_second);

            if ui.is_window_hovered() && ui.is_mouse_clicked(imgui::MouseButton::Right) {
                ui.open_popup("dope_sheet_context");
            }
            build_dope_sheet_context_menu(ui, ui_events, timeline_state);
        });
}

fn draw_dope_sheet_ruler(
    ui: &imgui::Ui,
    state: &TimelineState,
    sheet_width: f32,
    pixels_per_second: f32,
    display_duration: f32,
) {
    let cursor_pos = ui.cursor_screen_pos();
    let ruler_x = cursor_pos[0] + DOPE_LABEL_WIDTH;
    let draw_list = ui.get_window_draw_list();

    draw_list
        .add_rect(
            [ruler_x, cursor_pos[1]],
            [ruler_x + sheet_width, cursor_pos[1] + RULER_HEIGHT],
            [0.2, 0.2, 0.25, 1.0],
        )
        .filled(true)
        .build();

    let tick_interval = calculate_tick_interval(state.zoom_level);
    let mut time = 0.0;
    while time <= display_duration {
        let x = ruler_x + time * pixels_per_second;
        let is_major = (time / tick_interval).round() as i32 % 5 == 0;

        let tick_h = if is_major { 10.0 } else { 5.0 };
        let tick_color = if is_major {
            [0.7, 0.7, 0.7, 1.0]
        } else {
            [0.4, 0.4, 0.4, 1.0]
        };

        draw_list
            .add_line(
                [x, cursor_pos[1] + RULER_HEIGHT - tick_h],
                [x, cursor_pos[1] + RULER_HEIGHT],
                tick_color,
            )
            .build();

        if is_major {
            draw_list.add_text(
                [x - 10.0, cursor_pos[1] + 2.0],
                [0.8, 0.8, 0.8, 1.0],
                &format!("{:.1}s", time),
            );
        }

        time += tick_interval;
    }

    let playhead_x = ruler_x + state.current_time * pixels_per_second;
    draw_list
        .add_line(
            [playhead_x, cursor_pos[1]],
            [playhead_x, cursor_pos[1] + RULER_HEIGHT],
            [1.0, 0.3, 0.3, 1.0],
        )
        .thickness(2.0)
        .build();

    ui.text("Time:");
    ui.same_line_with_pos(DOPE_LABEL_WIDTH);
    ui.dummy([sheet_width, RULER_HEIGHT]);
}

fn draw_summary_row(
    ui: &imgui::Ui,
    state: &TimelineState,
    clip: &EditableAnimationClip,
    sheet_width: f32,
    pixels_per_second: f32,
) {
    let cursor_pos = ui.cursor_screen_pos();
    let row_x = cursor_pos[0] + DOPE_LABEL_WIDTH;
    let draw_list = ui.get_window_draw_list();

    draw_list
        .add_rect(
            [row_x, cursor_pos[1]],
            [row_x + sheet_width, cursor_pos[1] + DOPE_ROW_HEIGHT],
            [0.18, 0.18, 0.22, 1.0],
        )
        .filled(true)
        .build();

    let mut all_times: Vec<f32> = Vec::new();
    for track in clip.tracks.values() {
        let times = track.collect_all_keyframe_times();
        all_times.extend(times);
    }

    all_times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    all_times.dedup_by(|a, b| (*a - *b).abs() < 0.001);

    let y_center = cursor_pos[1] + DOPE_ROW_HEIGHT * 0.5;
    for time in &all_times {
        let x = row_x + time * pixels_per_second;
        draw_diamond(
            &draw_list,
            x,
            y_center,
            KEYFRAME_DIAMOND_SIZE,
            [0.8, 0.8, 0.8, 1.0],
        );
    }

    draw_playhead_line(
        &draw_list,
        row_x,
        cursor_pos[1],
        DOPE_ROW_HEIGHT,
        state.current_time,
        pixels_per_second,
    );

    ui.text_colored([0.7, 0.7, 0.7, 1.0], "Summary");
    ui.same_line_with_pos(DOPE_LABEL_WIDTH);
    ui.dummy([sheet_width, DOPE_ROW_HEIGHT]);
}

fn draw_bone_rows(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    state: &mut TimelineState,
    clip: &EditableAnimationClip,
    sheet_width: f32,
    pixels_per_second: f32,
) {
    let mut sorted_bone_ids: Vec<BoneId> = clip.tracks.keys().copied().collect();
    sorted_bone_ids.sort();

    for bone_id in sorted_bone_ids.into_iter().take(MAX_VISIBLE_TRACKS) {
        let track = match clip.tracks.get(&bone_id) {
            Some(t) => t,
            None => continue,
        };

        let is_expanded = state.is_track_expanded(bone_id);

        if is_expanded {
            draw_bone_row_expanded(
                ui,
                ui_events,
                state,
                bone_id,
                &track.bone_name,
                track,
                sheet_width,
                pixels_per_second,
            );
        } else {
            draw_bone_row_collapsed(
                ui,
                ui_events,
                state,
                bone_id,
                &track.bone_name,
                track,
                sheet_width,
                pixels_per_second,
            );
        }
    }
}

fn draw_bone_row_collapsed(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    state: &mut TimelineState,
    bone_id: BoneId,
    bone_name: &str,
    track: &crate::animation::editable::BoneTrack,
    sheet_width: f32,
    pixels_per_second: f32,
) {
    let cursor_pos = ui.cursor_screen_pos();
    let row_x = cursor_pos[0] + DOPE_LABEL_WIDTH;
    let draw_list = ui.get_window_draw_list();

    draw_list
        .add_rect(
            [row_x, cursor_pos[1]],
            [row_x + sheet_width, cursor_pos[1] + DOPE_ROW_HEIGHT],
            [0.15, 0.15, 0.18, 1.0],
        )
        .filled(true)
        .build();

    let y_center = cursor_pos[1] + DOPE_ROW_HEIGHT * 0.5;

    let all_properties = [
        PropertyType::TranslationX,
        PropertyType::TranslationY,
        PropertyType::TranslationZ,
        PropertyType::RotationX,
        PropertyType::RotationY,
        PropertyType::RotationZ,
        PropertyType::ScaleX,
        PropertyType::ScaleY,
        PropertyType::ScaleZ,
    ];

    for prop in &all_properties {
        let curve = track.get_curve(*prop);
        for kf in &curve.keyframes {
            let x = row_x + kf.time * pixels_per_second;

            let sel_key = SelectedKeyframe::new(bone_id, *prop, kf.id);
            let is_selected = state.is_keyframe_selected(&sel_key);

            let color = if is_selected {
                [1.0, 1.0, 0.2, 1.0]
            } else {
                [0.9, 0.7, 0.2, 1.0]
            };

            draw_diamond(&draw_list, x, y_center, KEYFRAME_DIAMOND_SIZE, color);

            state.dope_sheet_keyframe_hits.push(DopeSheetKeyframeHit {
                screen_x: x,
                screen_y: y_center,
                bone_id,
                property_type: *prop,
                keyframe_id: kf.id,
                time: kf.time,
            });
        }
    }

    draw_playhead_line(
        &draw_list,
        row_x,
        cursor_pos[1],
        DOPE_ROW_HEIGHT,
        state.current_time,
        pixels_per_second,
    );

    if ui.small_button(&format!(">##{}", bone_id)) {
        ui_events.send(UIEvent::TimelineToggleTrack(bone_id));
    }

    ui.same_line();
    let label = truncate_name(bone_name, 15);
    ui.text_colored([1.0, 1.0, 1.0, 1.0], &label);
    ui.same_line_with_pos(DOPE_LABEL_WIDTH);
    ui.dummy([sheet_width, DOPE_ROW_HEIGHT]);
}

fn draw_bone_row_expanded(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    state: &mut TimelineState,
    bone_id: BoneId,
    bone_name: &str,
    track: &crate::animation::editable::BoneTrack,
    sheet_width: f32,
    pixels_per_second: f32,
) {
    if ui.small_button(&format!("v##{}", bone_id)) {
        ui_events.send(UIEvent::TimelineToggleTrack(bone_id));
    }
    ui.same_line();
    let label = truncate_name(bone_name, 15);
    ui.text_colored([0.9, 0.9, 0.5, 1.0], &label);

    let all_properties = [
        PropertyType::TranslationX,
        PropertyType::TranslationY,
        PropertyType::TranslationZ,
        PropertyType::RotationX,
        PropertyType::RotationY,
        PropertyType::RotationZ,
        PropertyType::ScaleX,
        PropertyType::ScaleY,
        PropertyType::ScaleZ,
    ];

    for (prop_idx, prop) in all_properties.iter().enumerate() {
        let curve = track.get_curve(*prop);
        if curve.is_empty() {
            continue;
        }

        let cursor_pos = ui.cursor_screen_pos();
        let row_x = cursor_pos[0] + DOPE_LABEL_WIDTH;
        let draw_list = ui.get_window_draw_list();

        let bg_color = if prop_idx % 2 == 0 {
            [0.13, 0.13, 0.16, 1.0]
        } else {
            [0.16, 0.16, 0.19, 1.0]
        };

        draw_list
            .add_rect(
                [row_x, cursor_pos[1]],
                [row_x + sheet_width, cursor_pos[1] + DOPE_SUB_ROW_HEIGHT],
                bg_color,
            )
            .filled(true)
            .build();

        let y_center = cursor_pos[1] + DOPE_SUB_ROW_HEIGHT * 0.5;
        let prop_color = PROPERTY_COLORS[prop_idx % PROPERTY_COLORS.len()];

        for kf in &curve.keyframes {
            let x = row_x + kf.time * pixels_per_second;

            let sel_key = SelectedKeyframe::new(bone_id, *prop, kf.id);
            let is_selected = state.is_keyframe_selected(&sel_key);

            let color = if is_selected {
                [1.0, 1.0, 0.2, 1.0]
            } else {
                prop_color
            };

            draw_diamond(&draw_list, x, y_center, KEYFRAME_DIAMOND_SIZE - 1.0, color);

            state.dope_sheet_keyframe_hits.push(DopeSheetKeyframeHit {
                screen_x: x,
                screen_y: y_center,
                bone_id,
                property_type: *prop,
                keyframe_id: kf.id,
                time: kf.time,
            });
        }

        draw_playhead_line(
            &draw_list,
            row_x,
            cursor_pos[1],
            DOPE_SUB_ROW_HEIGHT,
            state.current_time,
            pixels_per_second,
        );

        ui.text_colored(prop_color, &format!("  {}", prop.short_name()));
        ui.same_line_with_pos(DOPE_LABEL_WIDTH);
        ui.dummy([sheet_width, DOPE_SUB_ROW_HEIGHT]);
    }
}

// --- Interaction State Machine ---

fn handle_dope_sheet_interaction(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    state: &mut TimelineState,
    pixels_per_second: f32,
) {
    let mouse_pos = ui.io().mouse_pos;
    let mouse_down = ui.io().mouse_down[0];
    let mouse_clicked = ui.is_mouse_clicked(imgui::MouseButton::Left);
    let mouse_released = ui.is_mouse_released(imgui::MouseButton::Left);

    if !ui.is_window_hovered_with_flags(imgui::WindowHoveredFlags::CHILD_WINDOWS)
        && !matches!(
            state.dope_sheet_interaction,
            DopeSheetInteraction::BoxSelecting(_) | DopeSheetInteraction::DraggingKeyframes(_)
        )
    {
        return;
    }

    // Take ownership of current interaction to avoid borrow issues
    let interaction = std::mem::replace(
        &mut state.dope_sheet_interaction,
        DopeSheetInteraction::None,
    );

    match interaction {
        DopeSheetInteraction::None => {
            if mouse_clicked {
                handle_click_begin(ui, ui_events, state, mouse_pos);
            }
        }
        DopeSheetInteraction::BoxSelecting(box_sel) => {
            let computed =
                compute_box_selection(&state.dope_sheet_keyframe_hits, &box_sel, mouse_pos);
            ui_events.send(UIEvent::TimelineSetKeyframeSelection {
                keyframes: computed,
                modifier: SelectionModifier::Replace,
            });

            if mouse_down && !mouse_released {
                state.dope_sheet_interaction = DopeSheetInteraction::BoxSelecting(box_sel);
            }
        }
        DopeSheetInteraction::DraggingKeyframes(drag) => {
            if mouse_down && !mouse_released {
                state.dope_sheet_interaction = DopeSheetInteraction::DraggingKeyframes(drag);
            } else {
                let delta_x = mouse_pos[0] - drag.drag_start_x;
                let time_delta = delta_x / pixels_per_second;
                let snapped_delta = apply_snap_to_delta(state, time_delta, pixels_per_second);

                crate::log!(
                    "[DopeSheet] drag_end: original_times={}, selected={}, delta={:.4}",
                    drag.original_times.len(),
                    state.selected_keyframes.len(),
                    snapped_delta
                );

                if snapped_delta.abs() > 0.001 {
                    ui_events.send(UIEvent::TimelineMoveSelectedKeyframes {
                        time_delta: snapped_delta,
                    });
                }
            }
        }
    }
}

fn handle_click_begin(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    state: &mut TimelineState,
    mouse_pos: [f32; 2],
) {
    let modifier = determine_selection_modifier(ui);

    if let Some(hit) = find_hit_keyframe(&state.dope_sheet_keyframe_hits, mouse_pos) {
        let sel_key = SelectedKeyframe::new(hit.bone_id, hit.property_type, hit.keyframe_id);
        let already_selected = state.is_keyframe_selected(&sel_key);

        crate::log!(
            "[DopeSheet] click_begin: hit bone={} prop={:?} kf={}, modifier={:?}, already_selected={}, current_selection={}",
            hit.bone_id, hit.property_type, hit.keyframe_id,
            modifier, already_selected, state.selected_keyframes.len()
        );

        if !already_selected {
            ui_events.send(UIEvent::TimelineSelectKeyframe {
                bone_id: hit.bone_id,
                property_type: hit.property_type,
                keyframe_id: hit.keyframe_id,
                modifier,
            });
        }

        let expected_selection = compute_expected_selection(
            &state.selected_keyframes,
            &sel_key,
            modifier,
            already_selected,
        );
        let original_times =
            collect_original_times(&expected_selection, &state.dope_sheet_keyframe_hits);

        crate::log!(
            "[DopeSheet] drag_start: expected_selection={}, original_times={}",
            expected_selection.len(),
            original_times.len()
        );

        state.dope_sheet_interaction =
            DopeSheetInteraction::DraggingKeyframes(DopeSheetKeyframeDrag {
                drag_start_x: mouse_pos[0],
                original_times,
            });
    } else {
        let original_selection = if matches!(modifier, SelectionModifier::Replace) {
            HashSet::new()
        } else {
            state.selected_keyframes.clone()
        };

        if matches!(modifier, SelectionModifier::Replace) && !state.selected_keyframes.is_empty() {
            ui_events.send(UIEvent::TimelineSetKeyframeSelection {
                keyframes: Vec::new(),
                modifier: SelectionModifier::Replace,
            });
        }

        state.dope_sheet_interaction = DopeSheetInteraction::BoxSelecting(DopeSheetBoxSelect {
            start_screen_pos: mouse_pos,
            modifier,
            original_selection,
        });
    }
}

fn find_hit_keyframe(
    hits: &[DopeSheetKeyframeHit],
    mouse_pos: [f32; 2],
) -> Option<DopeSheetKeyframeHit> {
    let mut closest: Option<(f32, &DopeSheetKeyframeHit)> = None;

    for hit in hits {
        let dx = mouse_pos[0] - hit.screen_x;
        let dy = mouse_pos[1] - hit.screen_y;
        let dist_sq = dx * dx + dy * dy;

        if dist_sq <= HIT_RADIUS * HIT_RADIUS {
            match closest {
                Some((best_dist, _)) if dist_sq < best_dist => {
                    closest = Some((dist_sq, hit));
                }
                None => {
                    closest = Some((dist_sq, hit));
                }
                _ => {}
            }
        }
    }

    closest.map(|(_, hit)| hit.clone())
}

fn compute_expected_selection(
    current: &HashSet<SelectedKeyframe>,
    sel_key: &SelectedKeyframe,
    modifier: SelectionModifier,
    already_selected: bool,
) -> HashSet<SelectedKeyframe> {
    if already_selected {
        return current.clone();
    }

    let mut result = current.clone();
    match modifier {
        SelectionModifier::Replace => {
            result.clear();
            result.insert(sel_key.clone());
        }
        SelectionModifier::Add | SelectionModifier::Toggle => {
            result.insert(sel_key.clone());
        }
    }
    result
}

fn collect_original_times(
    selection: &HashSet<SelectedKeyframe>,
    hits: &[DopeSheetKeyframeHit],
) -> Vec<(SelectedKeyframe, f32)> {
    selection
        .iter()
        .filter_map(|sel| {
            hits.iter()
                .find(|h| {
                    h.bone_id == sel.bone_id
                        && h.property_type == sel.property_type
                        && h.keyframe_id == sel.keyframe_id
                })
                .map(|h| (sel.clone(), h.time))
        })
        .collect()
}

fn compute_box_selection(
    hits: &[DopeSheetKeyframeHit],
    box_sel: &DopeSheetBoxSelect,
    mouse_pos: [f32; 2],
) -> Vec<SelectedKeyframe> {
    let min_x = box_sel.start_screen_pos[0].min(mouse_pos[0]);
    let max_x = box_sel.start_screen_pos[0].max(mouse_pos[0]);
    let min_y = box_sel.start_screen_pos[1].min(mouse_pos[1]);
    let max_y = box_sel.start_screen_pos[1].max(mouse_pos[1]);

    let hits_in_box: Vec<SelectedKeyframe> = hits
        .iter()
        .filter(|h| {
            h.screen_x >= min_x && h.screen_x <= max_x && h.screen_y >= min_y && h.screen_y <= max_y
        })
        .map(|h| SelectedKeyframe::new(h.bone_id, h.property_type, h.keyframe_id))
        .collect();

    let mut result = box_sel.original_selection.clone();
    match box_sel.modifier {
        SelectionModifier::Replace => {
            result.clear();
            for kf in hits_in_box {
                result.insert(kf);
            }
        }
        SelectionModifier::Add => {
            for kf in hits_in_box {
                result.insert(kf);
            }
        }
        SelectionModifier::Toggle => {
            for kf in hits_in_box {
                if box_sel.original_selection.contains(&kf) {
                    result.remove(&kf);
                } else {
                    result.insert(kf);
                }
            }
        }
    }

    result.into_iter().collect()
}

fn apply_snap_to_delta(state: &TimelineState, time_delta: f32, pixels_per_second: f32) -> f32 {
    if !state.snap_settings.snap_to_frame {
        return time_delta;
    }

    let frame_duration = 1.0 / state.snap_settings.frame_rate;
    let snap_threshold = state.snap_settings.snap_threshold_px / pixels_per_second;

    // Round delta to nearest frame
    let frames = (time_delta / frame_duration).round();
    let snapped = frames * frame_duration;

    if (snapped - time_delta).abs() < snap_threshold {
        snapped
    } else {
        time_delta
    }
}

// --- Visual Overlays ---

fn draw_interaction_overlays(ui: &imgui::Ui, state: &TimelineState, pixels_per_second: f32) {
    let draw_list = ui.get_window_draw_list();
    let mouse_pos = ui.io().mouse_pos;

    match &state.dope_sheet_interaction {
        DopeSheetInteraction::None => {}
        DopeSheetInteraction::BoxSelecting(box_sel) => {
            let min_x = box_sel.start_screen_pos[0].min(mouse_pos[0]);
            let max_x = box_sel.start_screen_pos[0].max(mouse_pos[0]);
            let min_y = box_sel.start_screen_pos[1].min(mouse_pos[1]);
            let max_y = box_sel.start_screen_pos[1].max(mouse_pos[1]);

            draw_list
                .add_rect([min_x, min_y], [max_x, max_y], BOX_SELECT_FILL)
                .filled(true)
                .build();
            draw_list
                .add_rect([min_x, min_y], [max_x, max_y], BOX_SELECT_BORDER)
                .build();
        }
        DopeSheetInteraction::DraggingKeyframes(drag) => {
            let delta_x = mouse_pos[0] - drag.drag_start_x;
            let time_delta = delta_x / pixels_per_second;
            let snapped_delta = apply_snap_to_delta(state, time_delta, pixels_per_second);
            let pixel_delta = snapped_delta * pixels_per_second;

            for (sel, _) in &drag.original_times {
                if let Some(hit) = state.dope_sheet_keyframe_hits.iter().find(|h| {
                    h.bone_id == sel.bone_id
                        && h.property_type == sel.property_type
                        && h.keyframe_id == sel.keyframe_id
                }) {
                    let preview_x = hit.screen_x + pixel_delta;
                    draw_diamond(
                        &draw_list,
                        preview_x,
                        hit.screen_y,
                        KEYFRAME_DIAMOND_SIZE,
                        [1.0, 1.0, 0.2, DRAG_PREVIEW_ALPHA],
                    );
                }
            }
        }
    }
}

pub fn build_dope_sheet_context_menu(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    timeline_state: &TimelineState,
) {
    if let Some(_token) = ui.begin_popup("dope_sheet_context") {
        let has_selection = !timeline_state.selected_keyframes.is_empty();

        if has_selection {
            if ui.selectable("Copy") {
                ui_events.send(UIEvent::TimelineCopyKeyframes);
            }
        } else {
            ui.text_disabled("Copy");
        }

        if ui.selectable("Paste") {
            ui_events.send(UIEvent::TimelinePasteKeyframes {
                paste_time: timeline_state.current_time,
            });
        }

        if ui.selectable("Mirror Paste") {
            ui_events.send(UIEvent::TimelineMirrorPaste {
                paste_time: timeline_state.current_time,
            });
        }

        ui.separator();

        if has_selection {
            if ui.selectable("Delete") {
                ui_events.send(UIEvent::TimelineDeleteSelectedKeyframes);
            }
        } else {
            ui.text_disabled("Delete");
        }
    }
}

fn determine_selection_modifier(ui: &imgui::Ui) -> SelectionModifier {
    let io = ui.io();
    if io.key_ctrl {
        SelectionModifier::Toggle
    } else if io.key_shift {
        SelectionModifier::Add
    } else {
        SelectionModifier::Replace
    }
}

fn draw_diamond(draw_list: &imgui::DrawListMut, x: f32, y: f32, size: f32, color: [f32; 4]) {
    let points = vec![[x, y - size], [x + size, y], [x, y + size], [x - size, y]];

    draw_list.add_polyline(points, color).filled(true).build();
}

fn draw_playhead_line(
    draw_list: &imgui::DrawListMut,
    row_x: f32,
    row_y: f32,
    row_height: f32,
    current_time: f32,
    pixels_per_second: f32,
) {
    let px = row_x + current_time * pixels_per_second;
    draw_list
        .add_line([px, row_y], [px, row_y + row_height], [1.0, 0.3, 0.3, 0.6])
        .build();
}

fn truncate_name(name: &str, max_len: usize) -> String {
    if name.len() > max_len {
        format!("{}...", &name[..max_len.saturating_sub(3)])
    } else {
        name.to_string()
    }
}

fn calculate_tick_interval(zoom_level: f32) -> f32 {
    if zoom_level < 0.5 {
        1.0
    } else if zoom_level < 1.0 {
        0.5
    } else if zoom_level < 2.0 {
        0.25
    } else {
        0.1
    }
}
