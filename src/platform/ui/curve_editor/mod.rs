use imgui::{Condition, MouseButton};
use thyllore_avatar_core::humanoid::components::role::HumanoidRole;

mod bone_label;
mod draw;
mod track_list;
mod view;

use bone_label::{format_bone_label, order_bone_ids_by_role};
use draw::*;
use track_list::*;
use view::*;

use crate::animation::editable::{
    BezierHandle, EditableAnimationClip, InterpolationType, KeyframeId, PropertyCurve,
    PropertyType, TangentType, TangentWeightMode,
};
use crate::animation::BoneId;
use crate::asset::AssetStorage;
use crate::ecs::component::{scalar_channel_for_property, ScalarChannelDomain};
use crate::ecs::resource::{
    ClipLibrary, CurveEditorBuffer, CurveEditorState, CurveEditorTarget, CurveInteractionMode,
    CurveSelectedKeyframe, CurveTrackRef, DraggingTangent, HumanoidRigState, PoseLibrary,
    TangentHandleType, TimelineState, UiPointerOwnerId,
};
use crate::ecs::systems::phases::event_dispatch::bone_track::EnsureBoneTrack;
#[cfg(feature = "ml")]
use crate::ecs::systems::phases::event_dispatch::ml::curve_suggestion::CurveSuggestionEvent;
use crate::ecs::systems::phases::event_dispatch::pose_library::PoseLibraryEvent;
use crate::ecs::systems::phases::event_dispatch::scalar_curve::ScalarCurveEvent;
use crate::ecs::systems::phases::event_dispatch::timeline::TimelineEvent;
use crate::ecs::world::World;
use crate::platform::ui::pointer::{
    read_ui_pointer, ui_pointer_available, ui_pointer_begin, PointerRegion, UiPointer,
};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

pub struct SuggestionOverlay {
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
const TIME_RULER_HEIGHT: f32 = 30.0;
const CURVE_PADDING: f32 = 10.0;
const KEYFRAME_HIT_RADIUS: f32 = 8.0;
const Y_AXIS_WIDTH: f32 = 50.0;

const ALL_PROPERTY_TYPES: &[(PropertyType, [f32; 4], &str)] = &[
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SelectionModifier {
    None,
    Toggle,
    Range,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReleasedButton {
    Left,
    Middle,
}

fn draw_curve_editor_window(
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

fn get_current_clip<'a>(
    timeline_state: &TimelineState,
    clip_library: &'a ClipLibrary,
) -> Option<&'a EditableAnimationClip> {
    timeline_state
        .current_clip_id
        .and_then(|id| clip_library.get(id))
}

fn build_curve_view(
    ui: &imgui::Ui,
    world: &World,
    timeline_state: &TimelineState,
    clip_library: &ClipLibrary,
    editor_state: &mut CurveEditorState,
    curve_buffer: &CurveEditorBuffer,
    suggestion_overlays: &[SuggestionOverlay],
    pose_library: &mut PoseLibrary,
) {
    build_curve_toolbar(ui, world, curve_buffer, pose_library, clip_library);
    ui.separator();

    let Some(clip) = get_current_clip(timeline_state, clip_library) else {
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
                ui.text("Morph track not found");
                return;
            };
            vec![(&morph_track.curve, [1.0, 0.5, 0.2, 1.0], "Weight")]
        }
        None => {
            ui.text("Select a track from the list");
            return;
        }
    };
    let Some(track_ref) = editor_state.selected_track_ref() else {
        return;
    };

    let content_region = ui.content_region_avail();
    let curve_area_width = content_region[0] - Y_AXIS_WIDTH - CURVE_PADDING * 2.0;
    let curve_area_height = content_region[1] - TIME_RULER_HEIGHT - CURVE_PADDING * 2.0;

    if curve_area_width <= 0.0 || curve_area_height <= 0.0 {
        return;
    }
    initialize_view_range(editor_state, &curves_to_draw, clip.duration);
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

    ui.set_cursor_screen_pos([cursor_pos[0], cursor_pos[1] + total_height]);

    #[cfg(feature = "ml")]
    if let Some(bone_id) = track_ref.bone_id() {
        handle_suggestion_keyboard(ui, world, bone_id, editor_state, suggestion_overlays);
    }
}

fn collect_visible_scalar_curves<'a>(
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

fn collect_visible_curves<'a>(
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

fn draw_curve_area(
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

fn draw_clipped_curve_content(
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
    draw_grid(draw_list, curve_area_width, curve_area_height, vt);

    let sample_count = calculate_sample_count(curve_area_width);
    for (curve, color, _name) in curves_to_draw {
        draw_curve_with_keyframes(draw_list, curve, *color, sample_count, vt, None);
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

    if matches!(
        editor_state.interaction,
        CurveInteractionMode::DraggingKeyframe
    ) {
        draw_keyframe_drag_preview(
            draw_list,
            read_ui_pointer(ui).pos,
            editor_state.drag_start_mouse_pos,
            vt,
            curves_to_draw,
            &editor_state.selected_keyframes,
        );
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
            &editor_state.visible_curves,
            vt,
        );
    }

    draw_suggestion_curve_overlay(
        draw_list,
        suggestion_overlays,
        &editor_state.visible_curves,
        vt,
    );
}

fn handle_curve_view_interaction(
    ui: &imgui::Ui,
    world: &World,
    editor_state: &mut CurveEditorState,
    vt: &ViewTransform,
    curves_to_draw: &[(&PropertyCurve, [f32; 4], &str)],
    cursor_pos: [f32; 2],
    curve_area_width: f32,
    clip_duration: f32,
    track_ref: CurveTrackRef,
) {
    let ruler_pos = [cursor_pos[0] + Y_AXIS_WIDTH + CURVE_PADDING, cursor_pos[1]];
    let pointer = read_ui_pointer(ui);
    let is_hovered = ui.is_item_hovered();
    let any_button_pressed = [MouseButton::Left, MouseButton::Middle, MouseButton::Right]
        .into_iter()
        .any(|button| pointer.is_clicked(button));
    let owns_press = any_button_pressed
        && is_hovered
        && ui_pointer_begin(
            ui,
            world,
            UiPointerOwnerId::CurveEditor,
            PointerRegion::LastItem,
        );
    let accepts_wheel = is_hovered
        && ui_pointer_available(
            ui,
            world,
            UiPointerOwnerId::CurveEditor,
            PointerRegion::LastItem,
        );

    handle_mouse_interaction(
        ui,
        world,
        editor_state,
        vt,
        curves_to_draw,
        &pointer,
        CurvePointerAccess {
            owns_press,
            accepts_wheel,
        },
        ruler_pos,
        curve_area_width,
        clip_duration,
    );

    if owns_press && pointer.is_clicked(MouseButton::Right) {
        let mouse_pos = pointer.pos;
        if let Some(hit) = find_keyframe_at_position(mouse_pos, curves_to_draw, vt) {
            editor_state.context_menu_keyframe = Some(CurveSelectedKeyframe {
                property_type: hit.0,
                keyframe_id: hit.1,
                original_time: hit.2,
                original_value: hit.3,
            });
            ui.open_popup("keyframe_context_menu");
        } else {
            editor_state.context_menu_click_time = vt.x_to_time(mouse_pos[0]);
            editor_state.context_menu_click_value = vt.y_to_value(mouse_pos[1]);
            ui.open_popup("curve_editor_context_menu");
        }
    }

    build_keyframe_context_menu(ui, world, editor_state, track_ref);
    build_curve_editor_context_menu(ui, world, editor_state, track_ref);
}

fn build_keyframe_context_menu(
    ui: &imgui::Ui,
    world: &World,
    editor_state: &mut CurveEditorState,
    track_ref: CurveTrackRef,
) {
    ui.popup("keyframe_context_menu", || {
        let ctx_kf = match editor_state.context_menu_keyframe.clone() {
            Some(kf) => kf,
            None => return,
        };

        if ui.selectable_config("Delete Key").build() {
            if editor_state.selected_keyframes.len() > 1 {
                for sel in &editor_state.selected_keyframes {
                    world.send_command(TimelineEvent::DeleteKeyframe {
                        track: track_ref,
                        property_type: sel.property_type.clone(),
                        keyframe_id: sel.keyframe_id,
                    });
                }
                editor_state.selected_keyframes.clear();
                editor_state.selection_anchor = None;
            } else {
                world.send_command(TimelineEvent::DeleteKeyframe {
                    track: track_ref,
                    property_type: ctx_kf.property_type.clone(),
                    keyframe_id: ctx_kf.keyframe_id,
                });
                editor_state.selected_keyframes.clear();
                editor_state.selection_anchor = None;
            }
        }

        let section_color = [0.6, 0.8, 1.0, 1.0];

        ui.separator();
        ui.text_colored(section_color, "Interpolation");
        ui.separator();

        if ui.selectable_config("  Linear").build() {
            world.send_command(TimelineEvent::SetKeyframeInterpolation {
                track: track_ref,
                property_type: ctx_kf.property_type,
                keyframe_id: ctx_kf.keyframe_id,
                interpolation: InterpolationType::Linear,
            });
        }

        if ui.selectable_config("  Bezier").build() {
            world.send_command(TimelineEvent::SetKeyframeInterpolation {
                track: track_ref,
                property_type: ctx_kf.property_type,
                keyframe_id: ctx_kf.keyframe_id,
                interpolation: InterpolationType::Bezier,
            });
        }

        if ui.selectable_config("  Stepped").build() {
            world.send_command(TimelineEvent::SetKeyframeInterpolation {
                track: track_ref,
                property_type: ctx_kf.property_type,
                keyframe_id: ctx_kf.keyframe_id,
                interpolation: InterpolationType::Stepped,
            });
        }

        ui.spacing();
        ui.text_colored(section_color, "Tangent");
        ui.separator();

        let tangent_options = [
            ("  Spline", TangentType::Spline),
            ("  Linear", TangentType::Linear),
            ("  Flat", TangentType::Flat),
            ("  Clamped", TangentType::Clamped),
            ("  Plateau", TangentType::Plateau),
            ("  Manual", TangentType::Manual),
        ];

        for (label, tangent_type) in &tangent_options {
            if ui.selectable_config(label).build() {
                world.send_command(TimelineEvent::SetTangentType {
                    track: track_ref,
                    property_type: ctx_kf.property_type,
                    keyframe_id: ctx_kf.keyframe_id,
                    tangent_type: *tangent_type,
                });
            }
        }

        ui.spacing();
        ui.text_colored(section_color, "Weight");
        ui.separator();

        if ui.selectable_config("  Non-Weighted").build() {
            world.send_command(TimelineEvent::SetTangentWeightMode {
                track: track_ref,
                property_type: ctx_kf.property_type,
                keyframe_id: ctx_kf.keyframe_id,
                weight_mode: TangentWeightMode::NonWeighted,
            });
        }

        if ui.selectable_config("  Weighted").build() {
            world.send_command(TimelineEvent::SetTangentWeightMode {
                track: track_ref,
                property_type: ctx_kf.property_type,
                keyframe_id: ctx_kf.keyframe_id,
                weight_mode: TangentWeightMode::Weighted,
            });
        }
    });
}

fn build_curve_editor_context_menu(
    ui: &imgui::Ui,
    world: &World,
    editor_state: &CurveEditorState,
    track_ref: CurveTrackRef,
) {
    ui.popup("curve_editor_context_menu", || {
        if ui.selectable_config("Add Key").build() {
            let bone_role = track_ref
                .bone_id()
                .and_then(|bone_id| find_bone_role(&collect_humanoid_bone_roles(world), bone_id));
            if let Some(property_type) = add_key_target_property(editor_state, track_ref, bone_role)
            {
                if let CurveTrackRef::Bone(bone_id) = track_ref {
                    if !current_clip_has_track(world, bone_id) {
                        world.send_command(EnsureBoneTrack { bone_id });
                    }
                }
                world.send_command(TimelineEvent::AddKeyframe {
                    track: track_ref,
                    property_type,
                    time: editor_state.context_menu_click_time.max(0.0),
                    value: editor_state.context_menu_click_value,
                });
            }
        }
    });
}

fn current_clip_has_track(world: &World, bone_id: BoneId) -> bool {
    let timeline_state = world.resource::<TimelineState>();
    let clip_library = world.resource::<ClipLibrary>();
    get_current_clip(&timeline_state, &clip_library)
        .is_some_and(|clip| clip.tracks.contains_key(&bone_id))
}

/// The scalar target only accepts registered channels: the visible set can
/// still hold bone property types from a previous bone target, and letting one
/// through would create a curve no channel answers to (grey "Custom", never
/// sampled). Lowest code wins so the choice is deterministic; the bone target
/// picks by declaration order for the same reason. A role bone only lists the
/// curves its role allows, so a translation left visible by the default set
/// must not win over the rotation the user checked.
fn add_key_target_property(
    editor_state: &CurveEditorState,
    track_ref: CurveTrackRef,
    bone_role: Option<HumanoidRole>,
) -> Option<PropertyType> {
    match track_ref {
        CurveTrackRef::Scalar => editor_state
            .visible_curves
            .iter()
            .copied()
            .filter(|p| scalar_channel_for_property(*p).is_some())
            .min_by_key(|p| match p {
                PropertyType::Custom(code) => *code,
                _ => u16::MAX,
            }),
        CurveTrackRef::Bone(_) => ALL_PROPERTY_TYPES
            .iter()
            .map(|(property_type, _, _)| *property_type)
            .filter(|property_type| {
                bone_role.is_none_or(|role| is_role_curve_allowed(role, *property_type))
            })
            .find(|property_type| editor_state.visible_curves.contains(property_type)),
        CurveTrackRef::Morph(_) => Some(PropertyType::MorphWeight),
    }
}

#[derive(Clone, Copy)]
struct CurvePointerAccess {
    owns_press: bool,
    accepts_wheel: bool,
}

fn handle_mouse_interaction(
    ui: &imgui::Ui,
    world: &World,
    editor_state: &mut CurveEditorState,
    vt: &ViewTransform,
    curves_to_draw: &[(&PropertyCurve, [f32; 4], &str)],
    pointer: &UiPointer,
    access: CurvePointerAccess,
    ruler_pos: [f32; 2],
    curve_area_width: f32,
    duration: f32,
) {
    let mouse_pos = pointer.pos;
    let mouse_clicked = access.owns_press && pointer.is_clicked(MouseButton::Left);
    let mouse_down = pointer.is_down(MouseButton::Left);
    let mouse_released = pointer.is_released(MouseButton::Left);
    let middle_clicked = access.owns_press && pointer.is_clicked(MouseButton::Middle);
    let middle_released = pointer.is_released(MouseButton::Middle);

    let in_ruler_area = mouse_pos[0] >= ruler_pos[0]
        && mouse_pos[0] <= ruler_pos[0] + curve_area_width
        && mouse_pos[1] >= ruler_pos[1]
        && mouse_pos[1] <= ruler_pos[1] + TIME_RULER_HEIGHT;

    let in_curve_area = mouse_pos[0] >= vt.curve_origin[0]
        && mouse_pos[0] <= vt.curve_origin[0] + vt.curve_width
        && mouse_pos[1] >= vt.curve_origin[1]
        && mouse_pos[1] <= vt.curve_origin[1] + vt.curve_height;

    if mouse_released {
        handle_mouse_release(
            world,
            editor_state,
            vt,
            mouse_pos,
            ReleasedButton::Left,
            curves_to_draw,
        );
    }
    if middle_released {
        handle_mouse_release(
            world,
            editor_state,
            vt,
            mouse_pos,
            ReleasedButton::Middle,
            curves_to_draw,
        );
    }

    if mouse_clicked && in_ruler_area {
        editor_state.interaction = CurveInteractionMode::ScrubbingRuler;
        let time = vt.x_to_time(mouse_pos[0]).clamp(0.0, duration);
        world.send_command(TimelineEvent::SetTime(time));
    }

    if mouse_clicked
        && in_curve_area
        && matches!(editor_state.interaction, CurveInteractionMode::Idle)
    {
        let modifier = if ui.io().key_ctrl {
            SelectionModifier::Toggle
        } else if ui.io().key_shift {
            SelectionModifier::Range
        } else {
            SelectionModifier::None
        };
        handle_curve_area_click(editor_state, mouse_pos, curves_to_draw, vt, modifier);
    }

    if middle_clicked && in_curve_area {
        editor_state.interaction = CurveInteractionMode::Panning {
            start_mouse_pos: mouse_pos,
            start_offset: [
                editor_state.view_time_offset,
                editor_state.view_value_offset,
            ],
        };
    }

    if matches!(
        editor_state.interaction,
        CurveInteractionMode::Panning { .. }
    ) && pointer.is_down(MouseButton::Middle)
    {
        handle_panning(editor_state, mouse_pos, vt);
    }

    if matches!(
        editor_state.interaction,
        CurveInteractionMode::ScrubbingRuler
    ) && mouse_down
    {
        let time = vt.x_to_time(mouse_pos[0]).clamp(0.0, duration);
        world.send_command(TimelineEvent::SetTime(time));
    }

    if access.accepts_wheel {
        handle_wheel_input(ui, editor_state, pointer, vt);
    }
}

fn handle_mouse_release(
    world: &World,
    editor_state: &mut CurveEditorState,
    vt: &ViewTransform,
    mouse_pos: [f32; 2],
    button: ReleasedButton,
    curves_to_draw: &[(&PropertyCurve, [f32; 4], &str)],
) {
    match button {
        ReleasedButton::Left => {
            if let CurveInteractionMode::DraggingTangent(ref dragging) =
                editor_state.interaction.clone()
            {
                if let Some(track_ref) = editor_state.selected_track_ref() {
                    let (in_tangent, out_tangent) =
                        compute_dragged_tangent(dragging, mouse_pos, curves_to_draw, vt);
                    world.send_command(TimelineEvent::SetKeyframeTangent {
                        track: track_ref,
                        property_type: dragging.property_type,
                        keyframe_id: dragging.keyframe_id,
                        in_tangent,
                        out_tangent,
                    });
                }
            } else if matches!(
                editor_state.interaction,
                CurveInteractionMode::DraggingKeyframe
            ) {
                if let Some(track_ref) = editor_state.selected_track_ref() {
                    let time_delta = vt.x_to_time(mouse_pos[0])
                        - vt.x_to_time(editor_state.drag_start_mouse_pos[0]);
                    let value_delta = vt.y_to_value(mouse_pos[1])
                        - vt.y_to_value(editor_state.drag_start_mouse_pos[1]);

                    for sel in &editor_state.selected_keyframes {
                        world.send_command(TimelineEvent::MoveKeyframe {
                            track: track_ref,
                            property_type: sel.property_type.clone(),
                            keyframe_id: sel.keyframe_id,
                            new_time: (sel.original_time + time_delta).max(0.0),
                            new_value: sel.original_value + value_delta,
                        });
                    }
                }
            }
            editor_state.interaction = CurveInteractionMode::Idle;
        }

        ReleasedButton::Middle => {
            editor_state.interaction = CurveInteractionMode::Idle;
        }
    }
}

fn compute_dragged_tangent(
    dragging: &DraggingTangent,
    mouse_pos: [f32; 2],
    curves_to_draw: &[(&PropertyCurve, [f32; 4], &str)],
    vt: &ViewTransform,
) -> (BezierHandle, BezierHandle) {
    for (curve, _, _) in curves_to_draw {
        if curve.property_type != dragging.property_type {
            continue;
        }

        let kf = match curve.get_keyframe(dragging.keyframe_id) {
            Some(kf) => kf,
            None => break,
        };

        let mouse_time = vt.x_to_time(mouse_pos[0]);
        let mouse_value = vt.y_to_value(mouse_pos[1]);
        let time_offset = mouse_time - kf.time;
        let value_offset = mouse_value - kf.value;
        let new_handle = BezierHandle::new(time_offset, value_offset);

        return match dragging.handle_type {
            TangentHandleType::In => (new_handle, kf.out_tangent.clone()),
            TangentHandleType::Out => (kf.in_tangent.clone(), new_handle),
        };
    }

    (BezierHandle::linear(), BezierHandle::linear())
}

fn handle_curve_area_click(
    editor_state: &mut CurveEditorState,
    mouse_pos: [f32; 2],
    curves_to_draw: &[(&PropertyCurve, [f32; 4], &str)],
    vt: &ViewTransform,
    modifier: SelectionModifier,
) {
    if let Some((handle_type, property_type, keyframe_id, original_handle)) =
        find_tangent_handle_at_position(
            mouse_pos,
            curves_to_draw,
            &editor_state.selected_keyframes,
            vt,
        )
    {
        editor_state.interaction = CurveInteractionMode::DraggingTangent(DraggingTangent {
            property_type,
            keyframe_id,
            handle_type,
            original_handle,
        });
        editor_state.drag_start_mouse_pos = mouse_pos;
        return;
    }

    let hit_keyframe = find_keyframe_at_position(mouse_pos, curves_to_draw, vt);

    if let Some((property_type, keyframe_id, time, value)) = hit_keyframe {
        let new_selected = CurveSelectedKeyframe {
            property_type: property_type.clone(),
            keyframe_id,
            original_time: time,
            original_value: value,
        };

        let should_drag = match modifier {
            SelectionModifier::Toggle => {
                let existing = editor_state
                    .selected_keyframes
                    .iter()
                    .position(|s| s.keyframe_id == keyframe_id && s.property_type == property_type);

                let drag = if let Some(pos) = existing {
                    editor_state.selected_keyframes.remove(pos);
                    false
                } else {
                    editor_state.selected_keyframes.push(new_selected.clone());
                    true
                };
                editor_state.selection_anchor = Some((property_type, keyframe_id));
                drag
            }

            SelectionModifier::Range => apply_shift_range_selection(
                editor_state,
                curves_to_draw,
                &property_type,
                keyframe_id,
                time,
                value,
            ),

            SelectionModifier::None => {
                let already_selected = editor_state
                    .selected_keyframes
                    .iter()
                    .any(|s| s.keyframe_id == keyframe_id && s.property_type == property_type);

                if !already_selected {
                    editor_state.selected_keyframes.clear();
                    editor_state.selected_keyframes.push(new_selected.clone());
                    editor_state.selection_anchor = Some((property_type, keyframe_id));
                }
                true
            }
        };

        if should_drag {
            refresh_selected_keyframe_positions(
                &mut editor_state.selected_keyframes,
                curves_to_draw,
            );
            editor_state.interaction = CurveInteractionMode::DraggingKeyframe;
            editor_state.drag_start_mouse_pos = mouse_pos;
        }
    } else if modifier == SelectionModifier::None {
        editor_state.selected_keyframes.clear();
        editor_state.selection_anchor = None;
    }
}

fn refresh_selected_keyframe_positions(
    selected: &mut [CurveSelectedKeyframe],
    curves: &[(&PropertyCurve, [f32; 4], &str)],
) {
    for sel in selected.iter_mut() {
        for (curve, _, _) in curves {
            if curve.property_type != sel.property_type {
                continue;
            }
            if let Some(kf) = curve.get_keyframe(sel.keyframe_id) {
                sel.original_time = kf.time;
                sel.original_value = kf.value;
            }
            break;
        }
    }
}

fn apply_shift_range_selection(
    editor_state: &mut CurveEditorState,
    curves_to_draw: &[(&PropertyCurve, [f32; 4], &str)],
    property_type: &PropertyType,
    keyframe_id: KeyframeId,
    time: f32,
    value: f32,
) -> bool {
    let anchor = editor_state.selection_anchor.clone();

    if let Some((anchor_prop, anchor_id)) = anchor {
        if anchor_prop == *property_type {
            let anchor_time = find_keyframe_time(curves_to_draw, &anchor_prop, anchor_id);
            if let Some(anchor_time) = anchor_time {
                let range_keys =
                    collect_keyframes_in_range(curves_to_draw, property_type, anchor_time, time);
                for key in range_keys {
                    let already_exists = editor_state.selected_keyframes.iter().any(|s| {
                        s.keyframe_id == key.keyframe_id && s.property_type == key.property_type
                    });
                    if !already_exists {
                        editor_state.selected_keyframes.push(key);
                    }
                }
                return true;
            }
        }
    }

    let already_exists = editor_state
        .selected_keyframes
        .iter()
        .any(|s| s.keyframe_id == keyframe_id && s.property_type == *property_type);
    if !already_exists {
        editor_state.selected_keyframes.push(CurveSelectedKeyframe {
            property_type: property_type.clone(),
            keyframe_id,
            original_time: time,
            original_value: value,
        });
    }
    editor_state.selection_anchor = Some((property_type.clone(), keyframe_id));
    true
}

fn find_keyframe_time(
    curves: &[(&PropertyCurve, [f32; 4], &str)],
    property_type: &PropertyType,
    keyframe_id: KeyframeId,
) -> Option<f32> {
    for (curve, _, _) in curves {
        if curve.property_type == *property_type {
            return curve.get_keyframe(keyframe_id).map(|kf| kf.time);
        }
    }
    None
}

fn collect_keyframes_in_range(
    curves: &[(&PropertyCurve, [f32; 4], &str)],
    property_type: &PropertyType,
    time_a: f32,
    time_b: f32,
) -> Vec<CurveSelectedKeyframe> {
    let min_time = time_a.min(time_b);
    let max_time = time_a.max(time_b);

    for (curve, _, _) in curves {
        if curve.property_type == *property_type {
            return curve
                .keyframes
                .iter()
                .filter(|kf| kf.time >= min_time && kf.time <= max_time)
                .map(|kf| CurveSelectedKeyframe {
                    property_type: property_type.clone(),
                    keyframe_id: kf.id,
                    original_time: kf.time,
                    original_value: kf.value,
                })
                .collect();
        }
    }
    Vec::new()
}

fn find_keyframe_at_position(
    mouse_pos: [f32; 2],
    curves: &[(&PropertyCurve, [f32; 4], &str)],
    vt: &ViewTransform,
) -> Option<(PropertyType, KeyframeId, f32, f32)> {
    for (curve, _, _) in curves {
        for kf in &curve.keyframes {
            let x = vt.time_to_x(kf.time);
            let y = vt.value_to_y(kf.value);

            let dx = mouse_pos[0] - x;
            let dy = mouse_pos[1] - y;
            let distance = (dx * dx + dy * dy).sqrt();

            if distance <= KEYFRAME_HIT_RADIUS {
                return Some((curve.property_type.clone(), kf.id, kf.time, kf.value));
            }
        }
    }

    None
}

const TANGENT_HANDLE_HIT_RADIUS: f32 = 10.0;

fn find_tangent_handle_at_position(
    mouse_pos: [f32; 2],
    curves: &[(&PropertyCurve, [f32; 4], &str)],
    selected_keyframes: &[CurveSelectedKeyframe],
    vt: &ViewTransform,
) -> Option<(TangentHandleType, PropertyType, KeyframeId, BezierHandle)> {
    for selected in selected_keyframes {
        for (curve, _, _) in curves {
            if curve.property_type != selected.property_type {
                continue;
            }

            let kf = match curve.get_keyframe(selected.keyframe_id) {
                Some(kf) => kf,
                None => break,
            };

            if kf.interpolation != InterpolationType::Bezier {
                break;
            }

            let in_x = vt.time_to_x(kf.time + kf.in_tangent.time_offset);
            let in_y = vt.value_to_y(kf.value + kf.in_tangent.value_offset);
            let dx = mouse_pos[0] - in_x;
            let dy = mouse_pos[1] - in_y;
            if (dx * dx + dy * dy).sqrt() <= TANGENT_HANDLE_HIT_RADIUS {
                return Some((
                    TangentHandleType::In,
                    curve.property_type,
                    kf.id,
                    kf.in_tangent.clone(),
                ));
            }

            let out_x = vt.time_to_x(kf.time + kf.out_tangent.time_offset);
            let out_y = vt.value_to_y(kf.value + kf.out_tangent.value_offset);
            let dx = mouse_pos[0] - out_x;
            let dy = mouse_pos[1] - out_y;
            if (dx * dx + dy * dy).sqrt() <= TANGENT_HANDLE_HIT_RADIUS {
                return Some((
                    TangentHandleType::Out,
                    curve.property_type,
                    kf.id,
                    kf.out_tangent.clone(),
                ));
            }

            break;
        }
    }

    None
}

#[cfg(feature = "ml")]
fn handle_suggestion_keyboard(
    ui: &imgui::Ui,
    world: &World,
    bone_id: BoneId,
    editor_state: &CurveEditorState,
    suggestion_overlays: &[SuggestionOverlay],
) {
    let io = ui.io();
    let shift = io.key_shift;

    if shift && ui.is_key_pressed(imgui::Key::C) {
        for property_type in &editor_state.visible_curves {
            world.send_command(CurveSuggestionEvent::Request {
                bone_id,
                property_type: *property_type,
            });
        }
    }

    if ui.is_key_pressed(imgui::Key::Tab) && !suggestion_overlays.is_empty() {
        world.send_command(CurveSuggestionEvent::Accept);
    }

    if ui.is_key_pressed(imgui::Key::Escape) && !suggestion_overlays.is_empty() {
        world.send_command(CurveSuggestionEvent::Dismiss);
    }
}

fn build_curve_toolbar(
    ui: &imgui::Ui,
    world: &World,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_key_on_a_role_bone_skips_translations_left_visible_by_default() {
        let mut editor_state = CurveEditorState::default();
        editor_state.visible_curves.remove(&PropertyType::RotationX);
        editor_state.visible_curves.remove(&PropertyType::RotationZ);

        let property = add_key_target_property(
            &editor_state,
            CurveTrackRef::Bone(3),
            Some(HumanoidRole::Neck),
        );

        assert_eq!(property, Some(PropertyType::RotationY));
    }

    #[test]
    fn add_key_on_a_plain_bone_keeps_declaration_order() {
        let editor_state = CurveEditorState::default();

        let property = add_key_target_property(&editor_state, CurveTrackRef::Bone(3), None);

        assert_eq!(property, Some(PropertyType::TranslationX));
    }

    #[test]
    fn test_format_morph_track_name_short() {
        assert_eq!(format_morph_track_name("mesh", "smile"), "mesh/smile");
    }

    #[test]
    fn test_format_morph_track_name_truncated() {
        let result = format_morph_track_name("very_long_source_mesh", "very_long_channel");
        assert_eq!(result.len(), 20);
        assert!(result.starts_with("..."));
        assert!(result.ends_with("channel"));
    }

    #[test]
    fn test_format_morph_track_name_truncates_multibyte_on_char_boundary() {
        let result = format_morph_track_name("顔メッシュ", "まばたき左目を閉じる強め表情差分");
        assert_eq!(result.chars().count(), 20);
        assert!(result.starts_with("..."));
        assert!(result.ends_with("表情差分"));
    }

    #[test]
    fn mapped_bones_list_rotation_curves_before_any_key_exists() {
        use crate::ecs::systems::{
            build_humanoid_rig, copy_test_humanoid_fixture, find_first_skeleton,
            test_humanoid_world,
        };

        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let (fbx_path, _) = copy_test_humanoid_fixture(temp_dir.path());
        let (world, assets) = test_humanoid_world(&fbx_path);
        let skeleton = find_first_skeleton(&assets)
            .expect("fixture has a skeleton")
            .clone();
        world.resource_mut::<HumanoidRigState>().rig =
            Some(build_humanoid_rig(&fbx_path, &skeleton, None).expect("fixture is humanoid"));

        let bone_roles = collect_humanoid_bone_roles(&world);
        let role_bone = |role: HumanoidRole| {
            bone_roles
                .iter()
                .find(|(_, bone_role)| *bone_role == role)
                .map(|(bone_id, _)| *bone_id)
                .expect("role is mapped")
        };
        let hips = role_bone(HumanoidRole::Hips);
        let head = role_bone(HumanoidRole::Head);
        let clip = EditableAnimationClip::new(0, "empty".to_string());

        assert!(collect_listed_bone_ids(&clip, &bone_roles).contains(&head));
        assert!(is_curve_listed(
            &clip,
            &bone_roles,
            head,
            PropertyType::RotationZ
        ));
        assert!(!is_curve_listed(
            &clip,
            &bone_roles,
            head,
            PropertyType::TranslationY
        ));
        assert!(is_curve_listed(
            &clip,
            &bone_roles,
            hips,
            PropertyType::TranslationY
        ));
        assert!(!is_curve_listed(
            &clip,
            &bone_roles,
            hips,
            PropertyType::ScaleX
        ));
    }

    #[test]
    fn unmapped_bones_list_only_keyed_curves() {
        let mut clip = EditableAnimationClip::new(0, "bone".to_string());
        let track = clip.add_track(0, "Spine".to_string());
        crate::animation::editable::curve_add_keyframe(&mut track.rotation_x, 0.0, 1.0);

        assert!(is_curve_listed(&clip, &[], 0, PropertyType::RotationX));
        assert!(!is_curve_listed(&clip, &[], 0, PropertyType::RotationY));
    }
}

fn build_curve_editor_window(
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
fn collect_suggestion_overlays(world: &World) -> Vec<SuggestionOverlay> {
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
fn collect_suggestion_overlays(_: &World) -> Vec<SuggestionOverlay> {
    Vec::new()
}

crate::ui_window!("curve_editor", Floating, 0, build_curve_editor_window);
