pub(crate) mod deferred;
mod frame;
mod input;
mod ui_windows;

use winit::event::{ElementState, Event, WindowEvent};

use super::key_bindings::default_bindings;
use super::platform::System;
use super::ui::StatusBarState;
use crate::app::App;

use crate::ecs::events::UIEvent;
use crate::ecs::resource::MouseInput;

pub(crate) use frame::handle_redraw_requested;

impl System {
    pub fn main_loop(self, app: &mut App) {
        let System {
            event_loop,
            window,
            mut imgui,
            mut platform,
        } = self;
        let mut last_frame = std::time::Instant::now();
        let bindings = default_bindings();
        let mut status_bar_state = StatusBarState::default();
        #[cfg(feature = "auto-rig")]
        let mut text_to_mesh_dialog_state = crate::platform::ui::TextToMeshDialogState::default();
        #[cfg(feature = "auto-rig")]
        let mut text_to_animation_dialog_state =
            crate::platform::ui::TextToAnimationDialogState::default();

        event_loop
            .run(move |event, window_target| match event {
                Event::NewEvents(_) => {
                    let now = std::time::Instant::now();
                    imgui.io_mut().update_delta_time(now - last_frame);
                    last_frame = now;
                }

                Event::AboutToWait => {
                    platform
                        .prepare_frame(imgui.io_mut(), &window)
                        .expect("Failed to prepare frame");
                    window.request_redraw();
                }

                Event::WindowEvent {
                    event: ref window_event,
                    ..
                } => {
                    platform.handle_event(imgui.io_mut(), &window, &event);
                    dispatch_window_event(
                        window_event,
                        window_target,
                        app,
                        &mut imgui,
                        &mut platform,
                        &window,
                        &bindings,
                        &mut status_bar_state,
                        #[cfg(feature = "auto-rig")]
                        &mut text_to_mesh_dialog_state,
                        #[cfg(feature = "auto-rig")]
                        &mut text_to_animation_dialog_state,
                    );
                }

                Event::LoopExiting => {
                    unsafe { app.destroy() };
                }

                _ => {}
            })
            .expect("EventLoop error");
    }
}

fn dispatch_window_event(
    event: &WindowEvent,
    window_target: &winit::event_loop::EventLoopWindowTarget<()>,
    app: &mut App,
    imgui: &mut imgui::Context,
    platform: &mut imgui_winit_support::WinitPlatform,
    window: &winit::window::Window,
    bindings: &[super::key_bindings::KeyBinding],
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
                        .resource_mut::<crate::ecs::ModelState>()
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
            input::dispatch_keyboard_input(app, event, imgui, bindings);
        }

        WindowEvent::RedrawRequested => {
            handle_redraw_requested(
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

            if crate::ecs::systems::batch_run_is_completed(&app.data.ecs_world) {
                window_target.exit();
            }
        }

        _ => {}
    }
}
