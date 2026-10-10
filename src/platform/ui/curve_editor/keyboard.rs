use super::context_menu::CURVE_EXTRAPOLATION_MENU;
use super::window::SuggestionOverlay;
use crate::animation::editable::PropertyCurve;
use crate::animation::BoneId;
use crate::ecs::resource::{
    CurveEditorState, CurveInteractionMode, CurveSelectedKeyframe, FrameRequest,
};
#[cfg(feature = "ml")]
use crate::ecs::systems::phases::event_dispatch::ml::curve_suggestion::CurveSuggestionEvent;
use crate::ecs::world::World;
use crate::platform::ui::key_modifier::{current_key_modifier, KeyModifier};

#[derive(Clone, Copy, Debug, PartialEq)]
enum CurveEditorKeyAction {
    FrameSelected,
    FrameAll,
    FramePlayhead,
    SelectAll,
    OpenExtrapolationMenu,
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
];

pub(super) fn handle_curve_editor_keyboard(
    ui: &imgui::Ui,
    editor_state: &mut CurveEditorState,
    curves: &[(&PropertyCurve, [f32; 4], &str)],
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
                    let selected = select_all_keyframes(curves);
                    editor_state.selected_keyframes = selected;
                    editor_state.interaction = CurveInteractionMode::Idle;
                }
                CurveEditorKeyAction::OpenExtrapolationMenu => {
                    ui.open_popup(CURVE_EXTRAPOLATION_MENU);
                }
            }
        }
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

    if ui.is_key_pressed(imgui::Key::Tab) && !suggestion_overlays.is_empty() {
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
}
