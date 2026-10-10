use super::context_menu::CURVE_EXTRAPOLATION_MENU;
use super::window::SuggestionOverlay;
use crate::animation::editable::{
    InterpolationType, PropertyCurve, TangentContinuity, TangentType,
};
use crate::animation::BoneId;
use crate::ecs::resource::{
    CurveEditorState, CurveInteractionMode, CurveSelectedKeyframe, CurveTrackRef, FrameRequest,
};
#[cfg(feature = "ml")]
use crate::ecs::systems::phases::event_dispatch::ml::curve_suggestion::CurveSuggestionEvent;
use crate::ecs::systems::phases::event_dispatch::timeline::TimelineEvent;
use crate::ecs::world::World;
use crate::platform::ui::key_modifier::{current_key_modifier, KeyModifier};

#[derive(Clone, Copy, Debug, PartialEq)]
enum CurveEditorKeyAction {
    FrameSelected,
    FrameAll,
    FramePlayhead,
    SelectAll,
    OpenExtrapolationMenu,
    SetTangentType(TangentType),
    SetInterpolation(InterpolationType),
    ToggleTangentContinuity,
    ToggleViewMode,
}

struct CurveEditorKeyBinding {
    key: imgui::Key,
    modifier: KeyModifier,
    action: CurveEditorKeyAction,
}

const CURVE_EDITOR_KEY_BINDINGS: &[CurveEditorKeyBinding] = &[
    CurveEditorKeyBinding {
        key: imgui::Key::F,
        modifier: KeyModifier::None,
        action: CurveEditorKeyAction::FrameSelected,
    },
    CurveEditorKeyBinding {
        key: imgui::Key::Home,
        modifier: KeyModifier::None,
        action: CurveEditorKeyAction::FrameAll,
    },
    CurveEditorKeyBinding {
        key: imgui::Key::A,
        modifier: KeyModifier::None,
        action: CurveEditorKeyAction::SelectAll,
    },
    CurveEditorKeyBinding {
        key: imgui::Key::A,
        modifier: KeyModifier::Ctrl,
        action: CurveEditorKeyAction::SelectAll,
    },
    CurveEditorKeyBinding {
        key: imgui::Key::Keypad0,
        modifier: KeyModifier::None,
        action: CurveEditorKeyAction::FramePlayhead,
    },
    CurveEditorKeyBinding {
        key: imgui::Key::E,
        modifier: KeyModifier::Shift,
        action: CurveEditorKeyAction::OpenExtrapolationMenu,
    },
    CurveEditorKeyBinding {
        key: imgui::Key::Alpha1,
        modifier: KeyModifier::None,
        action: CurveEditorKeyAction::SetTangentType(TangentType::Clamped),
    },
    CurveEditorKeyBinding {
        key: imgui::Key::Alpha2,
        modifier: KeyModifier::None,
        action: CurveEditorKeyAction::SetTangentType(TangentType::Spline),
    },
    CurveEditorKeyBinding {
        key: imgui::Key::Alpha3,
        modifier: KeyModifier::None,
        action: CurveEditorKeyAction::SetTangentType(TangentType::Flat),
    },
    CurveEditorKeyBinding {
        key: imgui::Key::Alpha4,
        modifier: KeyModifier::None,
        action: CurveEditorKeyAction::SetTangentType(TangentType::Linear),
    },
    CurveEditorKeyBinding {
        key: imgui::Key::Alpha5,
        modifier: KeyModifier::None,
        action: CurveEditorKeyAction::SetInterpolation(InterpolationType::Stepped),
    },
    CurveEditorKeyBinding {
        key: imgui::Key::B,
        modifier: KeyModifier::None,
        action: CurveEditorKeyAction::ToggleTangentContinuity,
    },
    CurveEditorKeyBinding {
        key: imgui::Key::Tab,
        modifier: KeyModifier::Ctrl,
        action: CurveEditorKeyAction::ToggleViewMode,
    },
];

pub(super) struct CurveEditorKeyboardTarget<'a> {
    pub world: &'a World,
    pub curves: &'a [(&'a PropertyCurve, [f32; 4], &'a str)],
    pub track_ref: CurveTrackRef,
}

pub(super) fn handle_curve_editor_keyboard(
    ui: &imgui::Ui,
    editor_state: &mut CurveEditorState,
    target: &CurveEditorKeyboardTarget,
) {
    let focused =
        ui.is_window_focused_with_flags(imgui::WindowFocusedFlags::ROOT_AND_CHILD_WINDOWS);
    if !focused || ui.is_any_item_active() {
        return;
    }

    let modifier = current_key_modifier(ui);

    for binding in CURVE_EDITOR_KEY_BINDINGS {
        if modifier == binding.modifier && ui.is_key_pressed(binding.key) {
            match binding.action {
                CurveEditorKeyAction::FrameSelected => {
                    editor_state.frame_request = Some(FrameRequest::Selected);
                }
                CurveEditorKeyAction::FrameAll => {
                    editor_state.frame_request = Some(FrameRequest::All);
                }
                CurveEditorKeyAction::FramePlayhead => {
                    editor_state.frame_request = Some(FrameRequest::Playhead);
                }
                CurveEditorKeyAction::SelectAll => {
                    let selected = select_all_keyframes(target.curves);
                    editor_state.selected_keyframes = selected;
                    editor_state.interaction = CurveInteractionMode::Idle;
                }
                CurveEditorKeyAction::OpenExtrapolationMenu => {
                    ui.open_popup(CURVE_EXTRAPOLATION_MENU);
                }
                CurveEditorKeyAction::SetTangentType(tangent_type) => {
                    send_tangent_type(editor_state, target, tangent_type);
                }
                CurveEditorKeyAction::SetInterpolation(interpolation) => {
                    send_interpolation(editor_state, target, interpolation);
                }
                CurveEditorKeyAction::ToggleTangentContinuity => {
                    send_toggled_continuity(editor_state, target);
                }
                CurveEditorKeyAction::ToggleViewMode => {
                    editor_state.view_mode = editor_state.view_mode.toggled();
                }
            }
        }
    }
}

fn send_tangent_type(
    editor_state: &CurveEditorState,
    target: &CurveEditorKeyboardTarget,
    tangent_type: TangentType,
) {
    for selected in &editor_state.selected_keyframes {
        target.world.send_command(TimelineEvent::SetTangentType {
            track: target.track_ref,
            property_type: selected.property_type,
            keyframe_id: selected.keyframe_id,
            tangent_type,
        });
    }
}

fn send_interpolation(
    editor_state: &CurveEditorState,
    target: &CurveEditorKeyboardTarget,
    interpolation: InterpolationType,
) {
    for selected in &editor_state.selected_keyframes {
        target
            .world
            .send_command(TimelineEvent::SetKeyframeInterpolation {
                track: target.track_ref,
                property_type: selected.property_type,
                keyframe_id: selected.keyframe_id,
                interpolation,
            });
    }
}

fn send_toggled_continuity(editor_state: &CurveEditorState, target: &CurveEditorKeyboardTarget) {
    let continuity = compute_toggled_continuity(target.curves, &editor_state.selected_keyframes);
    for selected in &editor_state.selected_keyframes {
        target
            .world
            .send_command(TimelineEvent::SetTangentContinuity {
                track: target.track_ref,
                property_type: selected.property_type,
                keyframe_id: selected.keyframe_id,
                continuity,
            });
    }
}

fn compute_toggled_continuity(
    curves: &[(&PropertyCurve, [f32; 4], &str)],
    selected_keyframes: &[CurveSelectedKeyframe],
) -> TangentContinuity {
    let any_unified = selected_keyframes.iter().any(|selected| {
        curves
            .iter()
            .filter(|(curve, _, _)| curve.property_type == selected.property_type)
            .filter_map(|(curve, _, _)| curve.get_keyframe(selected.keyframe_id))
            .any(|kf| kf.continuity == TangentContinuity::Unified)
    });

    if any_unified {
        TangentContinuity::Broken
    } else {
        TangentContinuity::Unified
    }
}

pub(super) fn select_all_keyframes(
    curves: &[(&PropertyCurve, [f32; 4], &str)],
) -> Vec<CurveSelectedKeyframe> {
    let mut selected = Vec::new();
    for (curve, _, _) in curves {
        for kf in &curve.keyframes {
            selected.push(CurveSelectedKeyframe {
                property_type: curve.property_type,
                keyframe_id: kf.id,
                original_time: kf.time,
                original_value: kf.value,
            });
        }
    }
    selected
}

#[cfg(feature = "ml")]
pub(super) fn handle_suggestion_keyboard(
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

    if !io.key_ctrl && ui.is_key_pressed(imgui::Key::Tab) && !suggestion_overlays.is_empty() {
        world.send_command(CurveSuggestionEvent::Accept);
    }

    if ui.is_key_pressed(imgui::Key::Escape) && !suggestion_overlays.is_empty() {
        world.send_command(CurveSuggestionEvent::Dismiss);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::editable::{EditableKeyframe, PropertyType};

    fn make_curve(property_type: PropertyType) -> PropertyCurve {
        PropertyCurve::from_keyframes(
            0,
            property_type,
            vec![
                EditableKeyframe::new(0, 0.0, 1.0),
                EditableKeyframe::new(1, 1.0, 2.0),
            ],
            2,
        )
    }

    #[test]
    fn select_all_keyframes_empty() {
        let curves: Vec<(&PropertyCurve, [f32; 4], &str)> = vec![];
        let selected = select_all_keyframes(&curves);
        assert!(selected.is_empty());
    }

    #[test]
    fn select_all_takes_every_key_of_every_visible_curve() {
        let curve = make_curve(PropertyType::TranslationX);
        let curves = [(&curve, [1.0, 0.0, 0.0, 1.0], "x")];
        let selected = select_all_keyframes(&curves);
        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].keyframe_id, 0);
        assert_eq!(selected[0].original_time, 0.0);
        assert_eq!(selected[0].original_value, 1.0);
        assert_eq!(selected[1].keyframe_id, 1);
        assert_eq!(selected[1].original_time, 1.0);
        assert_eq!(selected[1].original_value, 2.0);
    }

    #[test]
    fn select_all_across_multiple_curves() {
        let curve_x = make_curve(PropertyType::TranslationX);
        let curve_y = make_curve(PropertyType::TranslationY);
        let curves = [
            (&curve_x, [1.0, 0.0, 0.0, 1.0], "x"),
            (&curve_y, [0.0, 1.0, 0.0, 1.0], "y"),
        ];
        let selected = select_all_keyframes(&curves);
        assert_eq!(selected.len(), 4);
    }

    #[test]
    fn toggle_breaks_when_any_selected_key_is_unified() {
        let mut curve = make_curve(PropertyType::TranslationX);
        curve.keyframes[0].continuity = TangentContinuity::Broken;
        let curves = [(&curve, [1.0, 0.0, 0.0, 1.0], "x")];
        let selected = select_all_keyframes(&curves);
        assert_eq!(
            compute_toggled_continuity(&curves, &selected),
            TangentContinuity::Broken
        );
    }

    #[test]
    fn toggle_unifies_when_every_selected_key_is_broken() {
        let mut curve = make_curve(PropertyType::TranslationX);
        for kf in &mut curve.keyframes {
            kf.continuity = TangentContinuity::Broken;
        }
        let curves = [(&curve, [1.0, 0.0, 0.0, 1.0], "x")];
        let selected = select_all_keyframes(&curves);
        assert_eq!(
            compute_toggled_continuity(&curves, &selected),
            TangentContinuity::Unified
        );
    }
}
