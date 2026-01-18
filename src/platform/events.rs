use std::time::Instant;

use imgui::MouseButton;
use winit::event::{Event, WindowEvent};

use super::platform::System;
use super::ui::{build_click_debug_overlay, build_debug_window, DebugWindowState};
use crate::app::{App, GUIData};
use crate::ecs::{process_ui_events_system, DeferredAction, UIEventQueue};

fn update_mouse_input(gui_data: &mut GUIData, ui: &imgui::Ui) {
    gui_data.is_left_clicked = false;
    gui_data.is_wheel_clicked = false;

    if !gui_data.imgui_wants_mouse {
        if ui.is_mouse_down(MouseButton::Left) {
            gui_data.is_left_clicked = true;
        }
        if ui.is_mouse_down(MouseButton::Middle) {
            gui_data.is_wheel_clicked = true;
        }
    }

    let io = ui.io();
    gui_data.is_ctrl_pressed = io.key_ctrl;
}

impl System {
    pub fn main_loop(self, app: &mut App, gui_data: &mut GUIData) {
        let System {
            event_loop,
            window,
            mut imgui,
            mut platform,
        } = self;
        let mut last_frame = Instant::now();

        event_loop
            .run(move |event, window_target| match event {
                Event::NewEvents(_) => {
                    let now = Instant::now();
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

                    match window_event {
                        WindowEvent::CursorMoved { position, .. } => {
                            gui_data.mouse_pos = [position.x as f32, position.y as f32];
                        }

                        WindowEvent::MouseWheel { delta, .. } => match delta {
                            winit::event::MouseScrollDelta::LineDelta(_, y) => {
                                gui_data.mouse_wheel = *y;
                            }
                            winit::event::MouseScrollDelta::PixelDelta(pos) => {
                                gui_data.mouse_wheel = pos.y as f32;
                            }
                        },

                        WindowEvent::Resized(new_size) => {
                            if new_size.width > 0 && new_size.height > 0 {
                                app.resized = true;
                            }
                        }

                        WindowEvent::CloseRequested => window_target.exit(),

                        WindowEvent::DroppedFile(path_buf) => {
                            if let Some(path) = path_buf.to_str() {
                                gui_data.file_path = path.to_string();
                            }
                        }

                        WindowEvent::RedrawRequested => {
                            let ui = imgui.frame();
                            ui.dockspace_over_main_viewport();

                            gui_data.monitor_value = 0.0;

                            let io = ui.io();
                            gui_data.imgui_wants_mouse = io.want_capture_mouse;

                            update_mouse_input(gui_data, ui);

                            let model_path = app.animation_playback().model_path.clone();
                            let load_status = gui_data.load_status.clone();

                            let mut debug_state = DebugWindowState {
                                model_path,
                                load_status,
                                light_position: app.data.rt_debug_state.light_position,
                                shadow_strength: app.data.rt_debug_state.shadow_strength,
                                enable_distance_attenuation: app
                                    .data
                                    .rt_debug_state
                                    .enable_distance_attenuation,
                                debug_view_mode: app.data.rt_debug_state.debug_view_mode,
                                cube_size: app.data.rt_debug_state.cube_size,
                            };

                            {
                                let ui_events = app.data.ecs_world.resource_mut::<UIEventQueue>();
                                build_debug_window(ui, ui_events, &mut debug_state, gui_data);
                            }

                            app.data.rt_debug_state.shadow_strength = debug_state.shadow_strength;
                            app.data.rt_debug_state.enable_distance_attenuation =
                                debug_state.enable_distance_attenuation;
                            app.data.rt_debug_state.debug_view_mode = debug_state.debug_view_mode;
                            app.data.rt_debug_state.set_cube_size(debug_state.cube_size);

                            build_click_debug_overlay(ui, gui_data);

                            platform.prepare_render(ui, &window);
                            let draw_data = imgui.render();

                            unsafe {
                                let deferred_actions = {
                                    let ui_events =
                                        app.data.ecs_world.get_resource_mut::<UIEventQueue>();
                                    if let Some(ui_events) = ui_events {
                                        process_ui_events_system(
                                            ui_events,
                                            &mut app.data.camera,
                                            &mut app.data.rt_debug_state,
                                            &app.data.graphics_resources,
                                        )
                                    } else {
                                        Vec::new()
                                    }
                                };

                                for action in deferred_actions {
                                    match action {
                                        DeferredAction::LoadModel { path } => {
                                            gui_data.selected_model_path = path;
                                            gui_data.file_changed = true;
                                        }
                                        DeferredAction::LoadCube => {
                                            if let Err(e) = app.load_cube_model() {
                                                crate::log!("Failed to load cube model: {:?}", e);
                                            }
                                        }
                                        DeferredAction::TakeScreenshot => {
                                            gui_data.take_screenshot = true;
                                        }
                                        DeferredAction::DebugShadowInfo => {
                                            app.log_shadow_debug_info();
                                        }
                                        DeferredAction::DebugBillboardDepth => {
                                            gui_data.debug_billboard_depth = true;
                                        }
                                        DeferredAction::DumpDebugInfo => {
                                            app.dump_debug_info();
                                        }
                                    }
                                }

                                let image_index = app.begin_frame(gui_data).unwrap();
                                app.update(image_index, gui_data).unwrap();
                                app.render(image_index, gui_data, draw_data).unwrap();
                            }

                            gui_data.mouse_wheel = 0.0;
                        }
                        _ => {}
                    }
                }
                _ => {}
            })
            .expect("EventLoop error");
    }
}
