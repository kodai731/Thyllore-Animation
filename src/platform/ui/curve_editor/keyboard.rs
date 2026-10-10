use super::window::SuggestionOverlay;
use crate::animation::BoneId;
use crate::ecs::resource::CurveEditorState;
#[cfg(feature = "ml")]
use crate::ecs::systems::phases::event_dispatch::ml::curve_suggestion::CurveSuggestionEvent;
use crate::ecs::world::World;

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
