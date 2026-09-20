use winit::event::{ElementState, WindowEvent};

use crate::app::App;
use crate::ecs::events::UIEvent;
use crate::ecs::resource::MouseInput;
use crate::platform::ui::StatusBarState;

pub(crate) fn dispatch_window_event(
    event: &WindowEvent,
    window_target: &winit::event_loop::EventLoopWindowTarget<()>,
    app: &mut App,
    imgui: &mut imgui::Context,
    platform: &mut imgui_winit_support::WinitPlatform,
    window: &winit::window::Window,
    bindings: &[crate::platform::key_bindings::KeyBinding],
    status_bar_state: &mut StatusBarState,
    #[cfg(feature = "auto-rig")]
    text_to_mesh_dialog: &mut crate::platform::ui::TextToMeshDialogState,
    #[cfg(feature = "auto-rig")]
    text_to_animation_dialog: &mut crate::platform::ui::TextToAnimationDialogState,
) {
    match event {
        WindowEvent::CloseRequested => window_target.exit(),

        WindowEvent::Resized(size) if size.width > 0 && size.height > 0 => {
            app.resized = true;
        }

        WindowEvent::CursorMoved { position, .. } => {
            let mut mouse = app.data.ecs_world.resource_mut::<MouseInput>();
            mouse.position = [position.x as f32, position.y as f32];
        }

        WindowEvent::MouseWheel { delta, .. } => {
            let mut mouse = app.data.ecs_world.resource_mut::<MouseInput>();
            mouse.wheel = match delta {
                winit::event::MouseScrollDelta::LineDelta(_, y) => *y,
                winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
            };
        }

        WindowEvent::DroppedFile(path_buf) => {
            if let Some(path) = path_buf.to_str() {
                if path.to_ascii_lowercase().ends_with(".png") {
                    // A dropped PNG fills the texture-fit path field (selection
                    // only — applying stays on the explicit Apply button).
                    app.data
                        .ecs_world
                        .resource_mut::<crate::ecs::FlameUIState>()
                        .texture_fit_path = path.to_string();
                } else {
                    let mut ui_events = app
                        .data
                        .ecs_world
                        .resource_mut::<crate::ecs::UIEventQueue>();
                    ui_events.send(UIEvent::LoadModel {
                        path: path.to_string(),
                    });
                }
            }
        }

        WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
            super::input::dispatch_keyboard_input(app, event, imgui, bindings);
        }

        WindowEvent::RedrawRequested => {
            super::frame::handle_redraw_requested(
                imgui,
                platform,
                window,
                app,
                status_bar_state,
                #[cfg(feature = "auto-rig")]
                text_to_mesh_dialog,
                #[cfg(feature = "auto-rig")]
                text_to_animation_dialog,
            );

            if app
                .resource::<crate::ecs::resource::AppExit>()
                .is_requested()
            {
                window_target.exit();
            }
        }

        _ => {}
    }
}
