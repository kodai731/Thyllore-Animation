use std::time::Instant;

use imgui::MouseButton;
use winit::event::{ElementState, Event, WindowEvent};

use super::key_bindings::{default_bindings, dispatch_keyboard_shortcut, ModifierKeys};
use super::platform::System;
use super::ui::{
    build_bottom_panel, build_click_debug_overlay, build_clip_browser_window,
    build_curve_editor_window, build_hierarchy_window, build_inspector_window, build_scene_overlay,
    build_status_bar_overlay, build_timeline_window, build_viewport_window, handle_splitters,
    CurveEditorState, DebugWindowState, LayoutSnapshot, SceneOverlayState, StatusBarState,
    TimelineInteractionState, ViewportInfo,
};
use crate::app::{App, GUIData};
use crate::ecs::events::UIEvent;
use crate::ecs::resource::{
    ClipBrowserState, ClipLibrary, CurveEditorBuffer, HierarchyState, MessageLog, PanelLayout,
    PoseLibrary, TimelineState,
};
use crate::ecs::systems::clip_track_systems::query_clip_tracks;
use crate::ecs::systems::panel_layout_systems::panel_layout_clamp_to_display;
use crate::ecs::systems::phases::run_event_dispatch_phase;
use crate::ecs::{DeferredAction, UIEventQueue};

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
        let rt_debug = app.rt_debug_state();
        DebugWindowState {
            debug_view_mode: rt_debug.debug_view_mode,
        }
    };

    let mut overlay_state = SceneOverlayState {
        model_path: app.model_state().model_path.clone(),
        load_status: gui_data.load_status.clone(),
    };

    build_ui_windows(
        ui,
        app,
        gui_data,
        &mut debug_state,
        &mut overlay_state,
        status_bar_state,
    );

    {
        let mut rt_debug_mut = app.rt_debug_state_mut();
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
    overlay_state: &mut SceneOverlayState,
    status_bar_state: &mut StatusBarState,
) {
    let display_size = ui.io().display_size;

    let layout_snapshot = {
        let mut panel_layout = app.data.ecs_world.resource_mut::<PanelLayout>();
        panel_layout_clamp_to_display(&mut panel_layout, display_size[0], display_size[1]);
        LayoutSnapshot::from_layout(&panel_layout, display_size)
    };

    build_side_panel_windows(ui, app, gui_data, debug_state, &layout_snapshot);
    let viewport_info = build_viewport_and_update_state(ui, app, gui_data, &layout_snapshot);

    {
        let mut ui_events = app.data.ecs_world.resource_mut::<UIEventQueue>();
        build_scene_overlay(
            ui,
            &mut *ui_events,
            overlay_state,
            gui_data,
            &app.data.ecs_world,
            &viewport_info,
        );
    }

    build_animation_editor_windows(ui, app, &layout_snapshot);
    build_status_and_splitters(ui, app, status_bar_state, &viewport_info, &layout_snapshot);
}

fn build_side_panel_windows(
    ui: &imgui::Ui,
    app: &mut App,
    gui_data: &mut GUIData,
    debug_state: &mut DebugWindowState,
    layout_snapshot: &LayoutSnapshot,
) {
    {
        let mut msg_log = app.data.ecs_world.resource_mut::<MessageLog>();
        msg_log.sync_from_buffer();
    }

    {
        let mut ui_events = app.data.ecs_world.resource_mut::<UIEventQueue>();
        let mut msg_log = app.data.ecs_world.resource_mut::<MessageLog>();
        build_bottom_panel(
            ui,
            &mut *ui_events,
            debug_state,
            gui_data,
            &app.data.ecs_world,
            &mut *msg_log,
            layout_snapshot,
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
            layout_snapshot,
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
            layout_snapshot,
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
            layout_snapshot,
        );
    }
}

fn build_viewport_and_update_state(
    ui: &imgui::Ui,
    app: &mut App,
    gui_data: &mut GUIData,
    layout_snapshot: &LayoutSnapshot,
) -> ViewportInfo {
    let texture_id = imgui::TextureId::new(app.data.viewport.texture_id());
    let current_size = [
        app.data.viewport.width as f32,
        app.data.viewport.height as f32,
    ];
    let info = build_viewport_window(ui, texture_id, current_size, layout_snapshot);

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
}

fn build_animation_editor_windows(ui: &imgui::Ui, app: &mut App, layout_snapshot: &LayoutSnapshot) {
    let clip_track_snapshot = {
        let clip_library = app.data.ecs_world.resource::<ClipLibrary>();
        query_clip_tracks(&app.data.ecs_world, &*clip_library, &app.data.ecs_assets)
    };

    {
        let mut timeline_state = app.data.ecs_world.resource_mut::<TimelineState>();
        let mut timeline_interaction = app
            .data
            .ecs_world
            .resource_mut::<TimelineInteractionState>();
        let clip_library = app.data.ecs_world.resource::<ClipLibrary>();
        let mut ui_events = app.data.ecs_world.resource_mut::<UIEventQueue>();
        let mut curve_editor = app.data.ecs_world.resource_mut::<CurveEditorState>();
        build_timeline_window(
            ui,
            &mut *ui_events,
            &mut *timeline_state,
            &mut *timeline_interaction,
            &*clip_library,
            &mut *curve_editor,
            &clip_track_snapshot,
            layout_snapshot,
        );
    }

    {
        let timeline_state = app.data.ecs_world.resource::<TimelineState>();
        let clip_library = app.data.ecs_world.resource::<ClipLibrary>();
        let mut ui_events = app.data.ecs_world.resource_mut::<UIEventQueue>();
        let mut curve_editor = app.data.ecs_world.resource_mut::<CurveEditorState>();
        let curve_buffer = app.data.ecs_world.resource::<CurveEditorBuffer>();
        let mut pose_library = app.data.ecs_world.resource_mut::<PoseLibrary>();

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
            &mut *pose_library,
        );
    }
}

fn build_status_and_splitters(
    ui: &imgui::Ui,
    app: &mut App,
    status_bar_state: &mut StatusBarState,
    viewport_info: &ViewportInfo,
    layout_snapshot: &LayoutSnapshot,
) {
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
        viewport_info,
        &*timeline_state,
        clip_duration,
    );

    let mut panel_layout = app.data.ecs_world.resource_mut::<PanelLayout>();
    handle_splitters(ui, &mut panel_layout, layout_snapshot);
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
            UIEvent::ClipBrowserExportGltf(source_id) => handle_clip_export_gltf(app, *source_id),
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
            crate::msg_error!("Failed to load clip: {:?}", e);
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

    use crate::ecs::systems::clip_library_systems::{
        clip_library_save_to_file, clip_library_update_save_metadata,
    };
    clip_library_update_save_metadata(&mut clip_library, source_id, new_name.clone(), &path);

    match clip_library_save_to_file(&clip_library, source_id, &path) {
        Ok(()) => {
            crate::msg_info!("Saved clip '{}' to {:?}", new_name, path);
        }
        Err(e) => {
            crate::msg_error!("Failed to save clip: {:?}", e);
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
        Ok(()) => crate::msg_info!("FBX exported: {:?}", path),
        Err(e) => crate::msg_error!("FBX export failed: {:?}", e),
    }
}

fn handle_clip_export_gltf(app: &mut App, source_id: u64) {
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

    let source_glb_path = app
        .data
        .ecs_world
        .get_resource::<crate::ecs::resource::GltfModelCache>()
        .and_then(|cache| cache.source_path.clone());

    let Some(source_glb_path) = source_glb_path else {
        crate::msg_error!("glTF export failed: no source glTF/GLB model loaded");
        return;
    };

    let default_filename = format!("{}.glb", clip.name);
    let path = rfd::FileDialog::new()
        .add_filter("glTF Binary", &["glb"])
        .set_file_name(&default_filename)
        .save_file();

    let Some(path) = path else {
        return;
    };

    match crate::exporter::gltf_exporter::export_gltf_animation(
        std::path::Path::new(&source_glb_path),
        &clip,
        &skeleton,
        &path,
    ) {
        Ok(()) => crate::msg_info!("glTF exported: {:?}", path),
        Err(e) => crate::msg_error!("glTF export failed: {:?}", e),
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
            crate::msg_warn!("No baked clip to save");
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
            crate::msg_info!("Saved spring bone bake to {:?}", path);
        }
        Err(e) => {
            crate::msg_error!("Failed to save spring bone bake: {:?}", e);
        }
    }
}
