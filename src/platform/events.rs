use super::platform::System;
use std::time::Instant;
use winit::event::{Event, WindowEvent, ElementState};
use imgui::{Condition, MouseButton};

use crate::{App, GUIData};
use rust_rendering::debugview::DebugViewMode;

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
}

impl System {
    pub fn main_loop(
        self,
        app: &mut App,
        gui_data: &mut GUIData,
    ) {
        let System {
            event_loop,
            window,
            mut imgui,
            mut platform,
        } = self;
        let mut last_frame = Instant::now();

        event_loop
            .run(move |event, window_target| {
                match event {
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
                        window_id,
                        ..
                    } => {
                        platform.handle_event(imgui.io_mut(), &window, &event);

                        match window_event {
                            WindowEvent::CursorMoved { position, .. } => {
                                gui_data.mouse_pos = [position.x as f32, position.y as f32];
                            }

                            WindowEvent::MouseWheel { delta, .. } => match delta {
                                winit::event::MouseScrollDelta::LineDelta(x, y) => {
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

                                ui.window("debug window")
                                    .size([600.0, 450.0], Condition::FirstUseEver)
                                    .build(|| {
                                        ui.text("Model Loading:");
                                        if ui.button("Open FBX Model") {
                                            if let Some(path) = rfd::FileDialog::new()
                                                .add_filter("FBX Files", &["fbx"])
                                                .pick_file()
                                            {
                                                gui_data.selected_model_path = path.to_string_lossy().to_string();
                                                gui_data.file_changed = true;
                                                log!("Selected FBX file: {}", gui_data.selected_model_path);
                                            }
                                        }
                                        ui.same_line();
                                        if ui.button("Open glTF Model") {
                                            if let Some(path) = rfd::FileDialog::new()
                                                .add_filter("glTF Files", &["gltf", "glb"])
                                                .pick_file()
                                            {
                                                gui_data.selected_model_path = path.to_string_lossy().to_string();
                                                gui_data.file_changed = true;
                                                log!("Selected glTF file: {}", gui_data.selected_model_path);
                                            }
                                        }

                                        ui.text(format!("Current Model: {}",
                                            if app.data.current_model_path.is_empty() {
                                                "None"
                                            } else {
                                                &app.data.current_model_path
                                            }
                                        ));

                                        ui.text(format!("Status: {}", gui_data.load_status));

                                        ui.separator();

                                        ui.text("Camera Controls:");
                                        if ui.button("reset camera") {
                                            unsafe {
                                                app.reset_camera();
                                            }
                                        }
                                        ui.same_line();
                                        if ui.button("reset camera up") {
                                            unsafe {
                                                app.reset_camera_up();
                                            }
                                        }
                                        if ui.button("move to light gizmo") {
                                            unsafe {
                                                app.move_camera_to_light();
                                            }
                                        }
                                        ui.separator();

                                        ui.text("Screenshot:");
                                        if ui.button("Take Screenshot") {
                                            gui_data.take_screenshot = true;
                                        }
                                        ui.separator();

                                        ui.text("Ray Tracing Controls:");

                                        let mut light_pos = [
                                            app.data.rt_debug_state.light_position.x,
                                            app.data.rt_debug_state.light_position.y,
                                            app.data.rt_debug_state.light_position.z,
                                        ];
                                        if ui.slider_config("Light X", -50.0, 50.0)
                                            .build(&mut light_pos[0])
                                        {
                                            app.data.rt_debug_state.light_position.x = light_pos[0];
                                        }
                                        if ui.slider_config("Light Y", -50.0, 50.0)
                                            .build(&mut light_pos[1])
                                        {
                                            app.data.rt_debug_state.light_position.y = light_pos[1];
                                        }
                                        if ui.slider_config("Light Z", -50.0, 50.0)
                                            .build(&mut light_pos[2])
                                        {
                                            app.data.rt_debug_state.light_position.z = light_pos[2];
                                        }

                                        let mut shadow_strength = app.data.rt_debug_state.shadow_strength;
                                        if ui.slider_config("Shadow Strength", 0.0, 1.0)
                                            .build(&mut shadow_strength)
                                        {
                                            app.data.rt_debug_state.shadow_strength = shadow_strength;
                                        }

                                        ui.text("Debug View Mode:");
                                        let mut current_mode = app.data.rt_debug_state.debug_view_mode.as_int();
                                        if ui.radio_button("Final (Lit + Shadow)", &mut current_mode, 0) {
                                            app.data.rt_debug_state.debug_view_mode = DebugViewMode::Final;
                                        }
                                        if ui.radio_button("Position (World Space)", &mut current_mode, 1) {
                                            app.data.rt_debug_state.debug_view_mode = DebugViewMode::Position;
                                        }
                                        if ui.radio_button("Normal (World Space)", &mut current_mode, 2) {
                                            app.data.rt_debug_state.debug_view_mode = DebugViewMode::Normal;
                                        }
                                        if ui.radio_button("Shadow Mask", &mut current_mode, 3) {
                                            app.data.rt_debug_state.debug_view_mode = DebugViewMode::ShadowMask;
                                        }

                                        ui.separator();

                                        ui.text("Debug Info:");
                                        ui.checkbox("Show Click Debug", &mut gui_data.show_click_debug);
                                        if ui.button("Debug Shadow Info") {
                                            gui_data.debug_shadow_info = true;
                                        }
                                        ui.text(format!(
                                            "Mouse Position: ({:.1},{:.1})",
                                            gui_data.mouse_pos[0], gui_data.mouse_pos[1]
                                        ));
                                        ui.text(format!(
                                            "is left clicked: ({:.1})",
                                            gui_data.is_left_clicked
                                        ));
                                        ui.text(format!(
                                            "is wheel clicked: ({:.1})",
                                            gui_data.is_wheel_clicked
                                        ));
                                        ui.input_text("file path", &mut gui_data.file_path)
                                            .read_only(true)
                                            .build();
                                    });

                                if gui_data.show_click_debug {
                                    static mut IMGUI_SIZE_LOGGED: bool = false;
                                    unsafe {
                                        if !IMGUI_SIZE_LOGGED {
                                            let display_size = ui.io().display_size;
                                            log!("ImGui display size: {:.1} x {:.1}", display_size[0], display_size[1]);
                                            IMGUI_SIZE_LOGGED = true;
                                        }
                                    }

                                    if let Some(rect) = gui_data.billboard_click_rect {
                                        let draw_list = ui.get_foreground_draw_list();
                                        draw_list
                                            .add_rect(
                                                [rect[0], rect[1]],
                                                [rect[2], rect[3]],
                                                [1.0, 0.0, 0.0, 0.8],
                                            )
                                            .filled(true)
                                            .build();
                                        draw_list
                                            .add_rect(
                                                [rect[0], rect[1]],
                                                [rect[2], rect[3]],
                                                [1.0, 1.0, 0.0, 1.0],
                                            )
                                            .thickness(2.0)
                                            .build();
                                    }
                                }

                                platform.prepare_render(ui, &window);
                                let draw_data = imgui.render();

                                unsafe { app.render(&window, gui_data, draw_data) }.unwrap();

                                gui_data.mouse_wheel = 0.0;
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            })
            .expect("EventLoop error");
    }
}
