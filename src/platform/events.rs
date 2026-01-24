use std::time::Instant;

use imgui::MouseButton;
use winit::event::{Event, WindowEvent};

use super::platform::System;
use super::ui::{build_click_debug_overlay, build_debug_window, DebugWindowState};
use crate::app::{App, GUIData};
use crate::debugview::RayTracingDebugState;
use crate::ecs::{process_ui_events_with_events_simple, DeferredAction, UIEventQueue};
use crate::scene::camera::Camera;

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

                            let mut debug_state = {
                                let rt_debug = app.rt_debug_state();
                                DebugWindowState {
                                    model_path,
                                    load_status,
                                    light_position: rt_debug.light_position,
                                    shadow_strength: rt_debug.shadow_strength,
                                    enable_distance_attenuation: rt_debug.enable_distance_attenuation,
                                    debug_view_mode: rt_debug.debug_view_mode,
                                }
                            };

                            {
                                let mut ui_events = app.data.ecs_world.resource_mut::<UIEventQueue>();
                                build_debug_window(ui, &mut *ui_events, &mut debug_state, gui_data);
                            }

                            {
                                let mut rt_debug_mut = app.rt_debug_state_mut();
                                rt_debug_mut.shadow_strength = debug_state.shadow_strength;
                                rt_debug_mut.enable_distance_attenuation =
                                    debug_state.enable_distance_attenuation;
                                rt_debug_mut.debug_view_mode = debug_state.debug_view_mode;
                            }

                            build_click_debug_overlay(ui, gui_data);

                            platform.prepare_render(ui, &window);
                            let draw_data = imgui.render();

                            unsafe {
                                let deferred_actions = {
                                    let events: Vec<_> = {
                                        if let Some(mut ui_events) =
                                            app.data.ecs_world.get_resource_mut::<UIEventQueue>()
                                        {
                                            ui_events.drain().collect()
                                        } else {
                                            Vec::new()
                                        }
                                    };

                                    if events.is_empty() {
                                        Vec::new()
                                    } else {
                                        let model_bounds =
                                            app.data.graphics_resources.calculate_model_bounds();
                                        let world = &app.data.ecs_world;
                                        let mut camera = world.resource_mut::<Camera>();
                                        let mut rt_debug = world.resource_mut::<RayTracingDebugState>();
                                        crate::ecs::process_ui_events_with_events_simple(
                                            events,
                                            &mut camera,
                                            &mut rt_debug,
                                            model_bounds,
                                        )
                                    }
                                };

                                for action in deferred_actions {
                                    match action {
                                        DeferredAction::LoadModel { path } => {
                                            gui_data.selected_model_path = path;
                                            gui_data.file_changed = true;
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
