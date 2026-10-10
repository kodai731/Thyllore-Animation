use crate::animation::editable::{
    curve_sample, EditableAnimationClip, PropertyCurve, PropertyType,
};
use crate::animation::BoneId;
use crate::asset::AssetStorage;
use crate::ecs::component::ScalarChannelDomain;
use crate::ecs::resource::{
    ClipLibrary, CurveEditorBuffer, CurveEditorState, CurveEditorTarget, CurveInteractionMode,
    CurveTrackRef, CurveValueDisplay, CurveViewMode, FrameRequest, PoseLibrary, TimelineState,
};
use crate::ecs::systems::phases::event_dispatch::pose_library::PoseLibraryEvent;
use crate::ecs::systems::phases::event_dispatch::timeline::TimelineEvent;
use crate::ecs::world::World;
use crate::platform::ui::pointer::read_ui_pointer;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;
use imgui::Condition;
use thyllore_avatar_core::humanoid::components::role::HumanoidRole;

use super::context_menu::build_curve_extrapolation_menu;
use super::dopesheet::{draw_dopesheet, draw_dopesheet_drag_preview};
use super::draw::*;
use super::interaction::*;
use super::keyboard::*;
use super::numeric_input::build_selected_key_fields;
use super::track_list::*;
use super::view::*;

pub(super) struct SuggestionOverlay {
    pub property_type: PropertyType,
    pub time: f32,
    pub value: f32,
    pub tangent_in: (f32, f32),
    pub tangent_out: (f32, f32),
    pub confidence: f32,
}

const MIN_WINDOW_WIDTH: f32 = 400.0;
const MIN_WINDOW_HEIGHT: f32 = 300.0;
const TRACK_LIST_WIDTH: f32 = 180.0;
pub(super) const TIME_RULER_HEIGHT: f32 = 30.0;
pub(super) const CURVE_PADDING: f32 = 10.0;
pub(super) const Y_AXIS_WIDTH: f32 = 50.0;

pub(super) const ALL_PROPERTY_TYPES: &[(PropertyType, [f32; 4], &str)] = &[
    (PropertyType::TranslationX, [1.0, 0.3, 0.3, 1.0], "Pos.X"),
    (PropertyType::TranslationY, [0.3, 1.0, 0.3, 1.0], "Pos.Y"),
    (PropertyType::TranslationZ, [0.3, 0.3, 1.0, 1.0], "Pos.Z"),
    (PropertyType::RotationX, [1.0, 0.6, 0.6, 1.0], "Rot.X"),
    (PropertyType::RotationY, [0.6, 1.0, 0.6, 1.0], "Rot.Y"),
    (PropertyType::RotationZ, [0.6, 0.6, 1.0, 1.0], "Rot.Z"),
    (PropertyType::ScaleX, [1.0, 0.8, 0.4, 1.0], "Scl.X"),
    (PropertyType::ScaleY, [0.8, 1.0, 0.4, 1.0], "Scl.Y"),
    (PropertyType::ScaleZ, [0.4, 0.8, 1.0, 1.0], "Scl.Z"),
];

pub(super) fn draw_curve_editor_window(
    ui: &imgui::Ui,
    world: &World,
    timeline_state: &TimelineState,
    clip_library: &ClipLibrary,
    editor_state: &mut CurveEditorState,
    curve_buffer: &CurveEditorBuffer,
    suggestion_overlays: &[SuggestionOverlay],
    pose_library: &mut PoseLibrary,
    scalar_domain: Option<&'static ScalarChannelDomain>,
    bone_roles: &[(BoneId, HumanoidRole)],
) {
    if !editor_state.is_open {
        return;
    }

    let display_size = ui.io().display_size;
    let initial_pos = [
        (display_size[0] - editor_state.window_size[0]) * 0.5,
        (display_size[1] - editor_state.window_size[1]) * 0.5,
    ];

    let mut is_open = editor_state.is_open;
    let should_focus = editor_state.needs_focus;

    let mut window = ui
        .window("Curve Editor")
        .position(initial_pos, Condition::FirstUseEver)
        .size(editor_state.window_size, Condition::FirstUseEver)
        .size_constraints(
            [MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT],
            [display_size[0], display_size[1]],
        )
        .bg_alpha(1.0)
        .opened(&mut is_open);

    if should_focus {
        window = window.focused(true);
    }

    window.build(|| {
        editor_state.window_size = ui.window_size();

        let content_region = ui.content_region_avail();

        ui.child_window("left_panel")
            .size([TRACK_LIST_WIDTH, content_region[1]])
            .border(true)
            .build(|| {
                build_track_list(
                    ui,
                    world,
                    timeline_state,
                    clip_library,
                    editor_state,
                    scalar_domain,
                    bone_roles,
                );
            });

        ui.same_line();

        let curve_view_width = content_region[0] - TRACK_LIST_WIDTH - 10.0;
        ui.child_window("curve_view")
            .size([curve_view_width, content_region[1]])
            .border(true)
            .build(|| {
                build_curve_view(
                    ui,
                    world,
                    timeline_state,
                    clip_library,
                    editor_state,
                    curve_buffer,
                    suggestion_overlays,
                    pose_library,
                );
            });
    });

    editor_state.is_open = is_open;
}

pub(super) fn get_current_clip<'a>(
    timeline_state: &TimelineState,
    clip_library: &'a ClipLibrary,
) -> Option<&'a EditableAnimationClip> {
    timeline_state
        .current_clip_id
        .and_then(|id| clip_library.get(id))
}

pub(super) fn build_curve_view(
    ui: &imgui::Ui,
    world: &World,
    timeline_state: &TimelineState,
    clip_library: &ClipLibrary,
    editor_state: &mut CurveEditorState,
    curve_buffer: &CurveEditorBuffer,
    suggestion_overlays: &[SuggestionOverlay],
    pose_library: &mut PoseLibrary,
) {
    build_curve_toolbar(
        ui,
        world,
        editor_state,
        curve_buffer,
        pose_library,
        clip_library,
    );

    let Some(clip) = get_current_clip(timeline_state, clip_library) else {
        ui.separator();
        ui.text("No clip selected");
        return;
    };

    let curves_to_draw = match editor_state.selected_target {
        Some(CurveEditorTarget::Bone(bone_id)) => clip
            .tracks
            .get(&bone_id)
            .map(|track| collect_visible_curves(track, editor_state))
            .unwrap_or_default(),
        Some(CurveEditorTarget::Scalars) => collect_visible_scalar_curves(clip, editor_state),
        Some(CurveEditorTarget::Morph(i)) => {
            let Some(morph_track) = clip.morph_tracks.get(i) else {
                ui.separator();
                ui.text("Morph track not found");
                return;
            };
            vec![(&morph_track.curve, [1.0, 0.5, 0.2, 1.0], "Weight")]
        }
        None => {
            ui.separator();
            ui.text("Select a track from the list");
            return;
        }
    };
    let Some(track_ref) = editor_state.selected_track_ref() else {
        ui.separator();
        return;
    };

    build_selected_key_fields(ui, world, editor_state, &curves_to_draw, track_ref);
    ui.separator();

    let content_region = ui.content_region_avail();
    let curve_area_width = content_region[0] - Y_AXIS_WIDTH - CURVE_PADDING * 2.0;
    let curve_area_height = content_region[1] - TIME_RULER_HEIGHT - CURVE_PADDING * 2.0;

    if curve_area_width <= 0.0 || curve_area_height <= 0.0 {
        return;
    }
    initialize_view_range(editor_state, &curves_to_draw, clip.duration);
    apply_frame_request(editor_state, &curves_to_draw, timeline_state.current_time);
    let cursor_pos = ui.cursor_screen_pos();

    let curve_origin = [
        cursor_pos[0] + Y_AXIS_WIDTH + CURVE_PADDING,
        cursor_pos[1] + TIME_RULER_HEIGHT + CURVE_PADDING,
    ];

    let vt = ViewTransform {
        curve_origin,
        curve_width: curve_area_width,
        curve_height: curve_area_height,
        duration: editor_state.view_duration,
        val_range: editor_state.view_val_range,
        zoom_x: editor_state.zoom_x,
        zoom_y: editor_state.zoom_y,
        view_time_offset: editor_state.view_time_offset,
        view_value_offset: editor_state.view_value_offset,
        value_display: editor_state.value_display,
    };

    draw_curve_area(
        ui,
        &vt,
        cursor_pos,
        curve_area_width,
        curve_area_height,
        timeline_state,
        editor_state,
        &curves_to_draw,
        curve_buffer,
        suggestion_overlays,
        track_ref,
        pose_library,
    );

    let total_width = Y_AXIS_WIDTH + CURVE_PADDING + curve_area_width + CURVE_PADDING;
    let total_height = TIME_RULER_HEIGHT + CURVE_PADDING + curve_area_height + CURVE_PADDING;

    ui.set_cursor_screen_pos([cursor_pos[0], cursor_pos[1]]);
    ui.invisible_button("curve_interaction_area", [total_width, total_height]);
    if ui.is_item_hovered() {
        draw_normalized_value_tooltip(ui, &vt, &curves_to_draw);
    }

    handle_curve_view_interaction(
        ui,
        world,
        editor_state,
        &vt,
        &curves_to_draw,
        cursor_pos,
        curve_area_width,
        clip.duration,
        track_ref,
    );

    let keyboard_target = CurveEditorKeyboardTarget {
        world,
        curves: &curves_to_draw,
        track_ref,
    };
    handle_curve_editor_keyboard(ui, editor_state, &keyboard_target);
    build_curve_extrapolation_menu(ui, world, editor_state, &curves_to_draw, track_ref);

    ui.set_cursor_screen_pos([cursor_pos[0], cursor_pos[1] + total_height]);

    #[cfg(feature = "ml")]
    if let Some(bone_id) = track_ref.bone_id() {
        handle_suggestion_keyboard(ui, world, bone_id, editor_state, suggestion_overlays);
    }
}

pub(super) fn draw_normalized_value_tooltip(
    ui: &imgui::Ui,
    vt: &ViewTransform,
    curves_to_draw: &[(&PropertyCurve, [f32; 4], &str)],
) {
    if vt.value_display != CurveValueDisplay::Normalized {
        return;
    }

    let mouse_pos = read_ui_pointer(ui).pos;
    let Some(property_type) = find_curve_at_position(mouse_pos, curves_to_draw, vt) else {
        return;
    };
    let Some((curve, color, _)) = curves_to_draw
        .iter()
        .find(|(curve, _, _)| curve.property_type == property_type)
    else {
        return;
    };
    let Some(value) = curve_sample(curve, vt.x_to_time(mouse_pos[0])) else {
        return;
    };

    ui.tooltip(|| ui.text_colored(*color, format_value_label(value)));
}

pub(super) fn collect_visible_scalar_curves<'a>(
    clip: &'a EditableAnimationClip,
    editor_state: &CurveEditorState,
) -> Vec<(&'a PropertyCurve, [f32; 4], &'static str)> {
    let mut curves = Vec::new();
    for curve in &clip.scalar_curves {
        if !editor_state.visible_curves.contains(&curve.property_type) {
            continue;
        }
        if curve.is_empty() {
            continue;
        }
        let (color, name) = scalar_curve_style(curve.property_type);
        curves.push((curve, color, name));
    }
    curves
}

pub(super) fn collect_visible_curves<'a>(
    track: &'a crate::animation::editable::BoneTrack,
    editor_state: &CurveEditorState,
) -> Vec<(&'a PropertyCurve, [f32; 4], &'static str)> {
    let mut curves = Vec::new();
    for (prop_type, color, name) in ALL_PROPERTY_TYPES {
        if editor_state.visible_curves.contains(prop_type) {
            let curve = track.get_curve(*prop_type);
            if !curve.is_empty() {
                curves.push((curve, *color, *name));
            }
        }
    }
    curves
}

pub(super) fn draw_curve_area(
    ui: &imgui::Ui,
    vt: &ViewTransform,
    cursor_pos: [f32; 2],
    curve_area_width: f32,
    curve_area_height: f32,
    timeline_state: &TimelineState,
    editor_state: &CurveEditorState,
    curves_to_draw: &[(&PropertyCurve, [f32; 4], &str)],
    curve_buffer: &CurveEditorBuffer,
    suggestion_overlays: &[SuggestionOverlay],
    track_ref: CurveTrackRef,
    pose_library: &PoseLibrary,
) {
    let draw_list = ui.get_window_draw_list();

    if editor_state.view_mode == CurveViewMode::Curves {
        let y_axis_origin = [
            cursor_pos[0],
            cursor_pos[1] + TIME_RULER_HEIGHT + CURVE_PADDING,
        ];
        draw_y_axis_labels(
            &draw_list,
            y_axis_origin,
            Y_AXIS_WIDTH,
            curve_area_height,
            vt,
        );
    }

    let ruler_pos = [cursor_pos[0] + Y_AXIS_WIDTH + CURVE_PADDING, cursor_pos[1]];
    draw_time_ruler(&draw_list, ruler_pos, curve_area_width, vt);

    let co = vt.curve_origin;
    draw_list
        .add_rect(
            co,
            [co[0] + curve_area_width, co[1] + curve_area_height],
            [0.12, 0.12, 0.15, 1.0],
        )
        .filled(true)
        .build();

    draw_list.with_clip_rect_intersect(
        co,
        [co[0] + curve_area_width, co[1] + curve_area_height],
        || {
            draw_clipped_curve_content(
                ui,
                &draw_list,
                vt,
                curve_area_width,
                curve_area_height,
                timeline_state,
                editor_state,
                curves_to_draw,
                curve_buffer,
                suggestion_overlays,
                track_ref,
                pose_library,
            );
        },
    );
}

pub(super) fn draw_clipped_curve_content(
    ui: &imgui::Ui,
    draw_list: &imgui::DrawListMut,
    vt: &ViewTransform,
    curve_area_width: f32,
    curve_area_height: f32,
    timeline_state: &TimelineState,
    editor_state: &CurveEditorState,
    curves_to_draw: &[(&PropertyCurve, [f32; 4], &str)],
    curve_buffer: &CurveEditorBuffer,
    suggestion_overlays: &[SuggestionOverlay],
    track_ref: CurveTrackRef,
    pose_library: &PoseLibrary,
) {
    match editor_state.view_mode {
        CurveViewMode::Curves => draw_curves_with_selection(
            draw_list,
            vt,
            curve_area_width,
            curve_area_height,
            editor_state,
            curves_to_draw,
        ),
        CurveViewMode::Dopesheet => draw_dopesheet(
            draw_list,
            curves_to_draw,
            vt,
            &editor_state.selected_keyframes,
            curve_area_width,
        ),
    }

    draw_pose_markers(draw_list, vt, curve_area_height, pose_library);

    let playhead_x = vt.time_to_x(timeline_state.current_time);
    draw_list
        .add_line(
            [playhead_x, vt.curve_origin[1]],
            [playhead_x, vt.curve_origin[1] + curve_area_height],
            [1.0, 0.2, 0.2, 1.0],
        )
        .thickness(2.0)
        .build();

    if let CurveInteractionMode::DraggingKeyframe { axis_lock } = editor_state.interaction {
        let mouse_pos = read_ui_pointer(ui).pos;
        let drag_lock = effective_axis_lock(editor_state.view_mode, axis_lock);
        let time_snap = current_time_snap(ui, timeline_state.snap_settings.frame_rate);
        match editor_state.view_mode {
            CurveViewMode::Curves => draw_keyframe_drag_preview(
                draw_list,
                mouse_pos,
                editor_state.drag_start_mouse_pos,
                vt,
                curves_to_draw,
                &editor_state.selected_keyframes,
                drag_lock,
                time_snap,
            ),
            CurveViewMode::Dopesheet => draw_dopesheet_drag_preview(
                draw_list,
                mouse_pos,
                editor_state.drag_start_mouse_pos,
                vt,
                curves_to_draw,
                &editor_state.selected_keyframes,
                drag_lock,
                time_snap,
            ),
        }
    }

    if let CurveInteractionMode::DraggingTangent(ref dragging) = editor_state.interaction {
        let mouse_pos = read_ui_pointer(ui).pos;
        draw_tangent_drag_curve_preview(draw_list, dragging, mouse_pos, curves_to_draw, vt);
    }

    if let Some(bone_id) = track_ref.bone_id() {
        draw_buffer_curve_overlay(
            draw_list,
            curve_buffer,
            bone_id,
            curves_to_draw,
            &editor_state.visible_curves,
            vt,
        );
    }

    draw_suggestion_curve_overlay(
        draw_list,
        suggestion_overlays,
        curves_to_draw,
        &editor_state.visible_curves,
        vt,
    );

    if let CurveInteractionMode::BoxSelecting { start, .. } = editor_state.interaction {
        let mouse_pos = read_ui_pointer(ui).pos;
        let min: [f32; 2] = [start[0].min(mouse_pos[0]), start[1].min(mouse_pos[1])];
        let max: [f32; 2] = [start[0].max(mouse_pos[0]), start[1].max(mouse_pos[1])];
        let width = (max[0] - min[0]).abs();
        let height = (max[1] - min[1]).abs();
        if width > 1.0 || height > 1.0 {
            draw_list
                .add_rect(min, max, [0.3, 0.6, 1.0, 0.15])
                .filled(true)
                .build();
            draw_list
                .add_rect(min, max, [0.3, 0.6, 1.0, 1.0])
                .thickness(1.0)
                .build();
        }
    }
}

fn draw_curves_with_selection(
    draw_list: &imgui::DrawListMut,
    vt: &ViewTransform,
    curve_area_width: f32,
    curve_area_height: f32,
    editor_state: &CurveEditorState,
    curves_to_draw: &[(&PropertyCurve, [f32; 4], &str)],
) {
    draw_grid(draw_list, curve_area_width, curve_area_height, vt);
    draw_normalized_range_shade(draw_list, curve_area_width, curve_area_height, vt);

    let sample_count = calculate_sample_count(curve_area_width);
    for (curve, color, _name) in curves_to_draw {
        let curve_vt = vt.for_curve(curve);
        draw_curve_with_keyframes(draw_list, curve, *color, sample_count, &curve_vt, None);
    }

    if !editor_state.selected_keyframes.is_empty() {
        draw_selected_keyframes_highlight(
            draw_list,
            curves_to_draw,
            &editor_state.selected_keyframes,
            vt,
        );
        draw_tangent_handles(
            draw_list,
            curves_to_draw,
            &editor_state.selected_keyframes,
            vt,
        );
    }
}

pub(super) fn build_curve_toolbar(
    ui: &imgui::Ui,
    world: &World,
    editor_state: &mut CurveEditorState,
    curve_buffer: &CurveEditorBuffer,
    pose_library: &mut PoseLibrary,
    clip_library: &ClipLibrary,
) {
    if ui.small_button("Capture") {
        world.send_command(TimelineEvent::CaptureBuffer);
    }

    ui.same_line();
    if !curve_buffer.is_empty() {
        if ui.small_button("Swap") {
            world.send_command(TimelineEvent::SwapBuffer);
        }
    } else {
        ui.text_disabled("Swap");
    }

    if !curve_buffer.is_empty() {
        ui.same_line();
        ui.text_colored(
            [0.5, 0.8, 0.5, 1.0],
            &format!("Buf: {}", curve_buffer.snapshots.len()),
        );
    }

    ui.same_line_with_spacing(0.0, 20.0);
    ui.text("|");
    ui.same_line();

    if ui.small_button("Save Pose") {
        let name = format!("Pose {}", pose_library.poses.len() + 1);
        world.send_command(PoseLibraryEvent::SaveCurrent { name });
    }

    ui.same_line();
    if !pose_library.poses.is_empty() {
        let preview = pose_library
            .selected_pose_id
            .and_then(|id| clip_library.get(id))
            .map(|c| c.name.as_str())
            .unwrap_or("(none)");

        ui.set_next_item_width(120.0);
        if let Some(_token) = ui.begin_combo("##pose_select", preview) {
            let pose_ids = pose_library.pose_ids();
            for &pose_id in &pose_ids {
                let name = clip_library
                    .get(pose_id)
                    .map(|c| c.name.as_str())
                    .unwrap_or("(unknown)");

                let is_selected = pose_library.selected_pose_id == Some(pose_id);
                let label = format!("{}##pose_{}", name, pose_id);

                if ui.selectable_config(&label).selected(is_selected).build() {
                    pose_library.selected_pose_id = Some(pose_id);
                }
            }
        }

        ui.same_line();
    }

    if let Some(id) = pose_library.selected_pose_id {
        if ui.small_button("Apply##pose") {
            world.send_command(PoseLibraryEvent::Apply(id));
        }
        ui.same_line();
        if ui.small_button("Del##pose") {
            world.send_command(PoseLibraryEvent::Delete(id));
        }
    } else {
        ui.text_disabled("Apply");
        ui.same_line();
        ui.text_disabled("Del");
    }

    ui.same_line_with_spacing(0.0, 20.0);
    let mut is_normalized = editor_state.value_display == CurveValueDisplay::Normalized;
    if ui.checkbox("Normalize", &mut is_normalized) {
        editor_state.value_display = if is_normalized {
            CurveValueDisplay::Normalized
        } else {
            CurveValueDisplay::Actual
        };
        editor_state.frame_request = Some(FrameRequest::All);
    }

    ui.same_line_with_spacing(0.0, 20.0);
    ui.radio_button("Curves", &mut editor_state.view_mode, CurveViewMode::Curves);
    ui.same_line();
    ui.radio_button(
        "Dopesheet",
        &mut editor_state.view_mode,
        CurveViewMode::Dopesheet,
    );
}

pub(super) fn build_curve_editor_window(
    ui: &imgui::Ui,
    world: &World,
    _: &AssetStorage,
    _: &GraphicsResources,
) {
    let scalar_domain = {
        let current = world.resource::<TimelineState>().current_clip_id;
        current.and_then(|_| {
            crate::ecs::component::scalar_channel_domains()
                .iter()
                .copied()
                .find(|domain| {
                    (domain.entities)(world).iter().any(|&entity| {
                        crate::ecs::systems::scalar_clip_systems::find_entity_clip_id(world, entity)
                            == current
                    })
                })
        })
    };
    let suggestion_overlays = collect_suggestion_overlays(world);
    let bone_roles = collect_humanoid_bone_roles(world);

    let timeline_state = world.resource::<TimelineState>();
    let clip_library = world.resource::<ClipLibrary>();
    let mut curve_editor = world.resource_mut::<CurveEditorState>();
    let curve_buffer = world.resource::<CurveEditorBuffer>();
    let mut pose_library = world.resource_mut::<PoseLibrary>();
    draw_curve_editor_window(
        ui,
        world,
        &timeline_state,
        &clip_library,
        &mut curve_editor,
        &curve_buffer,
        &suggestion_overlays,
        &mut pose_library,
        scalar_domain,
        &bone_roles,
    );
    curve_editor.needs_focus = false;
}

#[cfg(feature = "ml")]
pub(super) fn collect_suggestion_overlays(world: &World) -> Vec<SuggestionOverlay> {
    world
        .get_resource::<crate::ecs::resource::CurveSuggestionState>()
        .map(|state| {
            state
                .suggestions
                .iter()
                .map(|s| SuggestionOverlay {
                    property_type: s.property_type,
                    time: s.predicted_time,
                    value: s.predicted_value,
                    tangent_in: s.tangent_in,
                    tangent_out: s.tangent_out,
                    confidence: s.confidence,
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(not(feature = "ml"))]
pub(super) fn collect_suggestion_overlays(_: &World) -> Vec<SuggestionOverlay> {
    Vec::new()
}
