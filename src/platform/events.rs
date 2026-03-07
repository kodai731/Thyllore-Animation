use std::time::Instant;

use imgui::MouseButton;
use winit::event::{ElementState, Event, WindowEvent};

use super::key_bindings::{default_bindings, dispatch_keyboard_shortcut, ModifierKeys};
use super::platform::System;
use super::ui::{
    build_click_debug_overlay, build_clip_browser_window, build_curve_editor_window,
    build_debug_window, build_hierarchy_window, build_inspector_window, build_status_bar_overlay,
    build_timeline_window, build_viewport_window, CurveEditorState, DebugWindowState,
    StatusBarState,
};
use crate::app::{App, GUIData};
use crate::ecs::events::UIEvent;
use crate::ecs::resource::{
    ClipBrowserState, ClipLibrary, CurveEditorBuffer, HierarchyState, TimelineState,
};
use crate::ecs::systems::clip_track_systems::query_clip_tracks;
use crate::ecs::systems::phases::run_event_dispatch_phase;
use crate::ecs::DeferredAction;
use crate::ecs::UIEventQueue;

fn update_mouse_input(gui_data: &mut GUIData, ui: &imgui::Ui) {
    let io = ui.io();
    gui_data.mouse_pos = io.mouse_pos;
    gui_data.is_left_clicked = ui.is_mouse_down(MouseButton::Left);
    gui_data.is_right_clicked = ui.is_mouse_down(MouseButton::Right);
    gui_data.is_wheel_clicked = ui.is_mouse_down(MouseButton::Middle);
    gui_data.is_ctrl_pressed = io.key_ctrl;
    gui_data.is_shift_pressed = io.key_shift;
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
        let bindings = default_bindings();
        let mut status_bar_state = StatusBarState::default();

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

                        WindowEvent::KeyboardInput { event, .. } => {
                            if event.state == ElementState::Pressed {
                                let modifiers = ModifierKeys {
                                    ctrl: gui_data.is_ctrl_pressed,
                                    shift: gui_data.is_shift_pressed,
                                };
                                if let Some(ui_event) = dispatch_keyboard_shortcut(
                                    &event.logical_key,
                                    modifiers,
                                    imgui.io().want_capture_keyboard,
                                    &bindings,
                                ) {
                                    let mut ui_events =
                                        app.data.ecs_world.resource_mut::<UIEventQueue>();
                                    ui_events.send(ui_event);
                                }
                            }
                        }

                        WindowEvent::RedrawRequested => {
                            handle_redraw_requested(
                                &mut imgui,
                                &mut platform,
                                &window,
                                app,
                                gui_data,
                                &mut status_bar_state,
                            );
                        }
                        _ => {}
                    }
                }
                _ => {}
            })
            .expect("EventLoop error");
    }
}

fn handle_redraw_requested(
    imgui: &mut imgui::Context,
    platform: &mut imgui_winit_support::WinitPlatform,
    window: &winit::window::Window,
    app: &mut App,
    gui_data: &mut GUIData,
    status_bar_state: &mut StatusBarState,
) {
    let ui = imgui.frame();
    ui.dockspace_over_main_viewport();

    gui_data.monitor_value = 0.0;

    let io = ui.io();
    gui_data.imgui_wants_mouse = io.want_capture_mouse;

    update_mouse_input(gui_data, ui);

    let mut debug_state = {
        let model_path = app.model_state().model_path.clone();
        let load_status = gui_data.load_status.clone();
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

    build_ui_windows(ui, app, gui_data, &mut debug_state, status_bar_state);

    {
        let mut rt_debug_mut = app.rt_debug_state_mut();
        rt_debug_mut.shadow_strength = debug_state.shadow_strength;
        rt_debug_mut.enable_distance_attenuation = debug_state.enable_distance_attenuation;
        rt_debug_mut.debug_view_mode = debug_state.debug_view_mode;
    }

    build_click_debug_overlay(ui, gui_data);

    platform.prepare_render(ui, window);
    let draw_data = imgui.render();

    unsafe {
        process_ui_events_and_render_frame(app, gui_data, window, draw_data);
    }

    gui_data.mouse_wheel = 0.0;
}

fn build_ui_windows(
    ui: &imgui::Ui,
    app: &mut App,
    gui_data: &mut GUIData,
    debug_state: &mut DebugWindowState,
    status_bar_state: &mut StatusBarState,
) {
    {
        let mut ui_events = app.data.ecs_world.resource_mut::<UIEventQueue>();
        build_debug_window(
            ui,
            &mut *ui_events,
            debug_state,
            gui_data,
            &app.data.ecs_world,
        );
    }

    {
        let hierarchy_state = app.data.ecs_world.resource::<HierarchyState>();
        let mut ui_events = app.data.ecs_world.resource_mut::<UIEventQueue>();
        build_hierarchy_window(
            ui,
            &mut *ui_events,
            &app.data.ecs_world,
            &*hierarchy_state,
            &app.data.ecs_assets,
        );
    }

    {
        let clip_library = app.data.ecs_world.resource::<ClipLibrary>();
        let mut browser_state = app.data.ecs_world.resource_mut::<ClipBrowserState>();
        let mut ui_events = app.data.ecs_world.resource_mut::<UIEventQueue>();
        build_clip_browser_window(
            ui,
            &mut *ui_events,
            &*clip_library,
            &mut *browser_state,
            &app.data.ecs_world,
        );
    }

    {
        let hierarchy_state = app.data.ecs_world.resource::<HierarchyState>();
        let mut ui_events = app.data.ecs_world.resource_mut::<UIEventQueue>();
        build_inspector_window(
            ui,
            &mut *ui_events,
            &app.data.ecs_world,
            &*hierarchy_state,
            &app.data.ecs_assets,
            &app.data.graphics_resources,
        );
    }

    let viewport_info = {
        let texture_id = imgui::TextureId::new(app.data.viewport.texture_id());
        let current_size = [
            app.data.viewport.width as f32,
            app.data.viewport.height as f32,
        ];
        let info = build_viewport_window(ui, texture_id, current_size);

        app.data.viewport.focused = info.focused;
        app.data.viewport.hovered = info.hovered;
        gui_data.viewport_focused = info.focused;
        gui_data.viewport_hovered = info.hovered;
        gui_data.viewport_position = info.position;
        gui_data.viewport_size = info.size;

        let new_width = info.size[0] as u32;
        let new_height = info.size[1] as u32;
        if new_width > 0
            && new_height > 0
            && (new_width != app.data.viewport.width || new_height != app.data.viewport.height)
        {
            gui_data.viewport_resize_pending = Some((new_width, new_height));
        }
        info
    };

    let clip_track_snapshot = {
        let clip_library = app.data.ecs_world.resource::<ClipLibrary>();
        query_clip_tracks(&app.data.ecs_world, &*clip_library, &app.data.ecs_assets)
    };

    {
        let mut timeline_state = app.data.ecs_world.resource_mut::<TimelineState>();
        let clip_library = app.data.ecs_world.resource::<ClipLibrary>();
        let mut ui_events = app.data.ecs_world.resource_mut::<UIEventQueue>();
        let mut curve_editor = app.data.ecs_world.resource_mut::<CurveEditorState>();
        build_timeline_window(
            ui,
            &mut *ui_events,
            &mut *timeline_state,
            &*clip_library,
            &mut *curve_editor,
            &clip_track_snapshot,
        );
    }

    {
        let timeline_state = app.data.ecs_world.resource::<TimelineState>();
        let clip_library = app.data.ecs_world.resource::<ClipLibrary>();
        let mut ui_events = app.data.ecs_world.resource_mut::<UIEventQueue>();
        let mut curve_editor = app.data.ecs_world.resource_mut::<CurveEditorState>();
        let curve_buffer = app.data.ecs_world.resource::<CurveEditorBuffer>();

        #[cfg(feature = "ml")]
        let suggestion_overlays: Vec<super::ui::SuggestionOverlay> = {
            if let Some(state) = app
                .data
                .ecs_world
                .get_resource::<crate::ecs::resource::CurveSuggestionState>()
            {
                state
                    .suggestions
                    .iter()
                    .map(|s| super::ui::SuggestionOverlay {
                        property_type: s.property_type,
                        time: s.predicted_time,
                        value: s.predicted_value,
                        tangent_in: s.tangent_in,
                        tangent_out: s.tangent_out,
                        is_bezier: s.is_bezier,
                        confidence: s.confidence,
                    })
                    .collect()
            } else {
                Vec::new()
            }
        };
        #[cfg(not(feature = "ml"))]
        let suggestion_overlays: Vec<super::ui::SuggestionOverlay> = Vec::new();

        build_curve_editor_window(
            ui,
            &mut *ui_events,
            &*timeline_state,
            &*clip_library,
            &mut *curve_editor,
            &*curve_buffer,
            &suggestion_overlays,
        );
    }

    {
        let delta_time = (app.start.elapsed().as_secs_f32() - app.last_update_time).max(0.001);
        let timeline_state = app.data.ecs_world.resource::<TimelineState>();
        let clip_duration = timeline_state
            .current_clip_id
            .and_then(|id| {
                let lib = app.data.ecs_world.resource::<ClipLibrary>();
                lib.get(id).map(|c| c.duration)
            })
            .unwrap_or(0.0);
        build_status_bar_overlay(
            ui,
            status_bar_state,
            delta_time,
            &viewport_info,
            &*timeline_state,
            clip_duration,
        );
    }
}

unsafe fn process_ui_events_and_render_frame(
    app: &mut App,
    gui_data: &mut GUIData,
    window: &winit::window::Window,
    draw_data: &imgui::DrawData,
) {
    let model_bounds = app.data.graphics_resources.calculate_model_bounds();
    let (platform_events, deferred_actions) = run_event_dispatch_phase(
        &mut app.data.ecs_world,
        &mut app.data.ecs_assets,
        model_bounds,
    );

    process_platform_file_events(&platform_events, app);

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
            DeferredAction::DumpAnimationDebug => {
                let clip_library = app.data.ecs_world.resource::<ClipLibrary>();
                if let Err(e) = crate::ecs::systems::animation_debug_dump::dump_animation_debug(
                    &app.data.ecs_world,
                    &app.data.ecs_assets,
                    &*clip_library,
                ) {
                    crate::log!("Animation debug dump failed: {:?}", e);
                }
            }
        }
    }

    let frame_result = (|| -> anyhow::Result<()> {
        let image_index = app.begin_frame(gui_data)?;
        app.update(image_index, gui_data)?;
        app.render(image_index, gui_data, draw_data)?;
        Ok(())
    })();

    if let Err(e) = frame_result {
        let msg = e.to_string();
        if msg.contains("SWAPCHAIN_OUT_OF_DATE") {
            app.recreate_swapchain(window).unwrap();
        } else {
            panic!("Frame error: {:?}", e);
        }
    }
}

fn process_platform_file_events(events: &[UIEvent], app: &mut App) {
    for event in events {
        match event {
            UIEvent::ClipBrowserLoadFromFile => handle_clip_load_from_file(app),
            UIEvent::ClipBrowserSaveToFile(source_id) => handle_clip_save_to_file(app, *source_id),
            UIEvent::ClipBrowserExportFbx(source_id) => handle_clip_export_fbx(app, *source_id),
            UIEvent::SpringBoneSaveBake => handle_spring_bone_save(app),
            _ => {}
        }
    }
}

fn handle_clip_load_from_file(app: &mut App) {
    let path = rfd::FileDialog::new()
        .add_filter("Animation RON", &["anim.ron", "ron"])
        .pick_file();

    let Some(path) = path else {
        return;
    };

    let bone_name_to_id = app
        .data
        .ecs_assets
        .skeletons
        .values()
        .next()
        .map(|sa| sa.skeleton.bone_name_to_id.clone());

    let mut clip_library = app.data.ecs_world.resource_mut::<ClipLibrary>();
    match crate::ecs::systems::clip_library_systems::clip_library_load_from_file(
        &mut clip_library,
        &mut app.data.ecs_assets,
        &path,
        bone_name_to_id.as_ref(),
    ) {
        Ok(_new_id) => {}
        Err(e) => {
            crate::log!("Failed to load clip: {:?}", e);
        }
    }
}

fn handle_clip_save_to_file(app: &mut App, source_id: u64) {
    let current_name = {
        let lib = app.data.ecs_world.resource::<ClipLibrary>();
        lib.get(source_id)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "clip".to_string())
    };

    let path = rfd::FileDialog::new()
        .add_filter("Animation RON", &["anim.ron", "ron"])
        .set_file_name(format!("{}.anim.ron", current_name))
        .save_file();

    let Some(path) = path else {
        return;
    };

    let new_name = extract_clip_name_from_path(&path);
    let mut clip_library = app.data.ecs_world.resource_mut::<ClipLibrary>();

    if let Some(clip) = clip_library.get_mut(source_id) {
        clip.name = new_name.clone();
        clip.source_path = Some(path.to_string_lossy().to_string());
    }

    use crate::ecs::systems::clip_library_systems::clip_library_save_to_file;
    match clip_library_save_to_file(&clip_library, source_id, &path) {
        Ok(()) => {
            crate::log!("Saved clip '{}' to {:?}", new_name, path);
        }
        Err(e) => {
            crate::log!("Failed to save clip: {:?}", e);
        }
    }
}

fn handle_clip_export_fbx(app: &mut App, source_id: u64) {
    let clip = {
        let lib = app.data.ecs_world.resource::<ClipLibrary>();
        lib.get(source_id).cloned()
    };
    let skeleton = app
        .data
        .ecs_assets
        .skeletons
        .values()
        .next()
        .map(|sa| sa.skeleton.clone());

    let (Some(clip), Some(skeleton)) = (clip, skeleton) else {
        return;
    };

    let default_filename = format!("{}.fbx", clip.name);
    let path = rfd::FileDialog::new()
        .add_filter("FBX Binary", &["fbx"])
        .set_file_name(&default_filename)
        .save_file();

    let Some(path) = path else {
        return;
    };

    let has_fbx_cache = app
        .data
        .ecs_world
        .contains_resource::<crate::ecs::resource::FbxModelCache>();
    let (fbx_model_ref, needs_coord_conversion) = if has_fbx_cache {
        let cache = app
            .data
            .ecs_world
            .resource::<crate::ecs::resource::FbxModelCache>();
        (cache.fbx_model.clone(), cache.needs_coord_conversion)
    } else {
        (None, false)
    };

    let (axes, fps) = if let Some(ref fbx_model) = fbx_model_ref {
        (fbx_model.axes.clone(), fbx_model.fps)
    } else {
        (crate::loader::fbx::fbx::FbxAxesInfo::default(), 24.0)
    };

    let result = if let Some(ref fbx_model) = fbx_model_ref {
        crate::exporter::fbx_exporter::export_full_fbx(fbx_model, Some(&clip), &skeleton, &path)
    } else {
        crate::exporter::fbx_animation::export_animation_fbx(
            &clip,
            &skeleton,
            &path,
            needs_coord_conversion,
            axes,
            fps,
        )
    };

    match result {
        Ok(()) => crate::log!("FBX exported: {:?}", path),
        Err(e) => crate::log!("FBX export failed: {:?}", e),
    }
}

fn extract_clip_name_from_path(path: &std::path::Path) -> String {
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("clip");

    filename
        .strip_suffix(".anim.ron")
        .or_else(|| filename.strip_suffix(".ron"))
        .unwrap_or(filename)
        .to_string()
}

fn handle_spring_bone_save(app: &mut App) {
    use crate::ecs::resource::SpringBoneState;
    use crate::ecs::systems::clip_library_systems::clip_library_save_to_file;

    let spring_state = app.data.ecs_world.resource::<SpringBoneState>();
    let baked_id = match spring_state.baked_clip_source_id {
        Some(id) => id,
        None => {
            crate::log!("No baked clip to save");
            return;
        }
    };
    drop(spring_state);

    let path = rfd::FileDialog::new()
        .add_filter("Animation RON", &["anim.ron", "ron"])
        .set_file_name("spring_baked.anim.ron")
        .save_file();

    let Some(path) = path else {
        return;
    };

    let clip_library = app.data.ecs_world.resource::<ClipLibrary>();
    match clip_library_save_to_file(&clip_library, baked_id, &path) {
        Ok(()) => {
            crate::log!("Saved spring bone bake to {:?}", path);
        }
        Err(e) => {
            crate::log!("Failed to save spring bone bake: {:?}", e);
        }
    }
}
