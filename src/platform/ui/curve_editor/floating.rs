use imgui::Condition;

use super::window::build_curve_editor_tab;
use crate::asset::AssetStorage;
use crate::ecs::resource::{
    ClipLibrary, CurveEditorBuffer, CurveEditorPlacement, CurveEditorState, PoseLibrary,
    TimelineState,
};
use crate::ecs::world::World;
use crate::platform::ui::theme::{draw_raised_window_frame, push_raised_window_style};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

const WINDOW_TITLE: &str = "Curve Editor";
const INITIAL_SIZE: [f32; 2] = [1280.0, 720.0];

fn build_floating_curve_editor(
    ui: &imgui::Ui,
    world: &World,
    _: &AssetStorage,
    _: &GraphicsResources,
) {
    let mut editor_state = world.resource_mut::<CurveEditorState>();
    if editor_state.placement != CurveEditorPlacement::Window {
        return;
    }
    let timeline_state = world.resource::<TimelineState>();
    let clip_library = world.resource::<ClipLibrary>();
    let curve_buffer = world.resource::<CurveEditorBuffer>();
    let mut pose_library = world.resource_mut::<PoseLibrary>();

    editor_state.is_open = true;
    let mut keep_open = true;
    let _raised = push_raised_window_style(ui);
    ui.window(WINDOW_TITLE)
        .size(INITIAL_SIZE, Condition::FirstUseEver)
        .opened(&mut keep_open)
        .build(|| {
            draw_raised_window_frame(ui);
            build_curve_editor_tab(
                ui,
                world,
                &timeline_state,
                &clip_library,
                &mut editor_state,
                &curve_buffer,
                &mut pose_library,
            );
        });

    if !keep_open {
        editor_state.placement = CurveEditorPlacement::TimelineTab;
        editor_state.needs_focus = true;
    }
}

crate::ui_window!("curve_editor", Floating, 7, build_floating_curve_editor);
