use std::time::Instant;

use imgui::MouseButton;
use winit::event::{ElementState, Event, WindowEvent};
use winit::keyboard::{Key, NamedKey};

use super::platform::System;
use super::ui::{build_click_debug_overlay, build_clip_browser_window, build_curve_editor_window, build_debug_window, build_hierarchy_window, build_inspector_window, build_timeline_window, build_viewport_window, collect_clip_track_snapshot, CurveEditorState, DebugWindowState};
use crate::ecs::resource::CurveEditorBuffer;
use crate::ecs::resource::ClipLibrary;
use crate::app::{App, GUIData};
use crate::debugview::RayTracingDebugState;
use crate::ecs::resource::{HierarchyState, SceneState, TimelineState};
use crate::ecs::events::UIEvent;
use crate::ecs::resource::KeyframeCopyBuffer;
use crate::ecs::resource::{ClipBrowserState, EditHistory};
use crate::ecs::systems::{
    apply_redo, apply_undo, camera_move_to_look_at, collapse_entity,
    expand_entity, process_clip_instance_events,
    process_keyframe_clipboard_events, rename_entity,
    timeline_process_events, update_entity_scale,
    update_entity_translation, update_entity_visible,
};
use crate::ecs::component::ClipSchedule;
use crate::ecs::world::Transform;
use crate::ecs::{process_ui_events_with_events_simple, DeferredAction, UIEventQueue};
use crate::scene::camera::Camera;

fn update_mouse_input(gui_data: &mut GUIData, ui: &imgui::Ui) {
    gui_data.is_left_clicked = false;
    gui_data.is_right_clicked = false;
    gui_data.is_wheel_clicked = false;

    let io = ui.io();
    gui_data.mouse_pos = io.mouse_pos;

    let allow_input =
        !gui_data.imgui_wants_mouse || gui_data.viewport_hovered;
    if allow_input {
        if ui.is_mouse_down(MouseButton::Left) {
            gui_data.is_left_clicked = true;
        }
        if ui.is_mouse_down(MouseButton::Right) {
            gui_data.is_right_clicked = true;
        }
        if ui.is_mouse_down(MouseButton::Middle) {
            gui_data.is_wheel_clicked = true;
        }
    }

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
                            if event.state == ElementState::Pressed && gui_data.is_ctrl_pressed {
                                if let Key::Character(ref c) = event.logical_key {
                                    if c.eq_ignore_ascii_case("s") {
                                        let mut ui_events = app.data.ecs_world.resource_mut::<UIEventQueue>();
                                        ui_events.send(UIEvent::SaveScene);
                                    }
                                }
                            }
                        }

                        WindowEvent::RedrawRequested => {
                            let ui = imgui.frame();
                            ui.dockspace_over_main_viewport();

                            gui_data.monitor_value = 0.0;

                            let io = ui.io();
                            gui_data.imgui_wants_mouse = io.want_capture_mouse;

                            update_mouse_input(gui_data, ui);

                            let model_path = app.model_state().model_path.clone();
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
                                let hierarchy_state = app.data.ecs_world.resource::<HierarchyState>();
                                let mut ui_events = app.data.ecs_world.resource_mut::<UIEventQueue>();
                                build_hierarchy_window(ui, &mut *ui_events, &app.data.ecs_world, &*hierarchy_state);
                            }

                            {
                                let clip_library = app.data.ecs_world.resource::<ClipLibrary>();
                                let mut browser_state = app.data.ecs_world.resource_mut::<ClipBrowserState>();
                                let mut ui_events = app.data.ecs_world.resource_mut::<UIEventQueue>();
                                build_clip_browser_window(ui, &mut *ui_events, &*clip_library, &mut *browser_state, &app.data.ecs_world);
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

                            {
                                let texture_id = imgui::TextureId::new(app.data.viewport.texture_id());
                                let current_size = [app.data.viewport.width as f32, app.data.viewport.height as f32];
                                let viewport_info = build_viewport_window(ui, texture_id, current_size);

                                app.data.viewport.focused = viewport_info.focused;
                                app.data.viewport.hovered = viewport_info.hovered;
                                gui_data.viewport_focused = viewport_info.focused;
                                gui_data.viewport_hovered = viewport_info.hovered;
                                gui_data.viewport_position = viewport_info.position;
                                gui_data.viewport_size = viewport_info.size;

                                let new_width = viewport_info.size[0] as u32;
                                let new_height = viewport_info.size[1] as u32;
                                if new_width > 0 && new_height > 0
                                    && (new_width != app.data.viewport.width || new_height != app.data.viewport.height)
                                {
                                    gui_data.viewport_resize_pending = Some((new_width, new_height));
                                }
                            }

                            let clip_track_snapshot = {
                                let clip_library = app.data.ecs_world.resource::<ClipLibrary>();
                                collect_clip_track_snapshot(&app.data.ecs_world, &*clip_library)
                            };

                            {
                                let mut timeline_state = app.data.ecs_world.resource_mut::<TimelineState>();
                                let clip_library = app.data.ecs_world.resource::<ClipLibrary>();
                                let mut ui_events = app.data.ecs_world.resource_mut::<UIEventQueue>();
                                let mut curve_editor = app.data.ecs_world.resource_mut::<CurveEditorState>();
                                build_timeline_window(ui, &mut *ui_events, &mut *timeline_state, &*clip_library, &mut *curve_editor, &clip_track_snapshot);
                            }

                            {
                                let timeline_state = app.data.ecs_world.resource::<TimelineState>();
                                let clip_library = app.data.ecs_world.resource::<ClipLibrary>();
                                let mut ui_events = app.data.ecs_world.resource_mut::<UIEventQueue>();
                                let mut curve_editor = app.data.ecs_world.resource_mut::<CurveEditorState>();
                                let curve_buffer = app.data.ecs_world.resource::<CurveEditorBuffer>();
                                build_curve_editor_window(ui, &mut *ui_events, &*timeline_state, &*clip_library, &mut *curve_editor, &*curve_buffer);
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
                                        process_hierarchy_events_inline(&events, app);
                                        process_timeline_events_inline(&events, app);
                                        process_keyframe_clipboard_events_inline(
                                            &events, app,
                                        );
                                        process_buffer_events_inline(
                                            &events, app,
                                        );
                                        process_clip_instance_events_inline(&events, app);
                                        process_clip_browser_events_inline(&events, app);
                                        process_edit_history_events_inline(&events, app);
                                        process_scene_events_inline(&events, app);
                                        process_debug_constraint_events_inline(
                                            &events, app,
                                        );
                                        process_constraint_edit_events_inline(
                                            &events, app,
                                        );

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

fn process_hierarchy_events_inline(events: &[UIEvent], app: &mut App) {
    use cgmath::Vector3;

    for event in events {
        match event {
            UIEvent::SelectEntity(entity) => {
                let mut hierarchy_state = app.data.ecs_world.resource_mut::<HierarchyState>();
                hierarchy_state.select(*entity);
            }

            UIEvent::DeselectAll => {
                let mut hierarchy_state = app.data.ecs_world.resource_mut::<HierarchyState>();
                hierarchy_state.deselect_all();
            }

            UIEvent::ToggleEntitySelection(entity) => {
                let mut hierarchy_state = app.data.ecs_world.resource_mut::<HierarchyState>();
                hierarchy_state.toggle_selection(*entity);
            }

            UIEvent::ExpandEntity(entity) => {
                expand_entity(&mut app.data.ecs_world, *entity);
            }

            UIEvent::CollapseEntity(entity) => {
                collapse_entity(&mut app.data.ecs_world, *entity);
            }

            UIEvent::SetSearchFilter(filter) => {
                let mut hierarchy_state = app.data.ecs_world.resource_mut::<HierarchyState>();
                hierarchy_state.search_filter = filter.clone();
            }

            UIEvent::SetEntityVisible(entity, visible) => {
                update_entity_visible(&mut app.data.ecs_world, *entity, *visible);
            }

            UIEvent::SetEntityTranslation(entity, translation) => {
                update_entity_translation(&mut app.data.ecs_world, *entity, *translation);
            }

            UIEvent::SetEntityRotation(entity, rotation) => {
                if let Some(transform) = app.data.ecs_world.get_component_mut::<Transform>(*entity) {
                    transform.rotation = *rotation;
                }
            }

            UIEvent::SetEntityScale(entity, scale) => {
                update_entity_scale(&mut app.data.ecs_world, *entity, *scale);
            }

            UIEvent::RenameEntity(entity, new_name) => {
                rename_entity(&mut app.data.ecs_world, *entity, new_name.clone());
            }

            UIEvent::FocusOnEntity(entity) => {
                let target = app
                    .data
                    .ecs_world
                    .get_component::<Transform>(*entity)
                    .map(|t| t.translation);

                if let Some(target) = target {
                    let offset = Vector3::new(5.0, 3.0, 5.0);
                    let mut camera = app.data.ecs_world.resource_mut::<Camera>();
                    camera_move_to_look_at(&mut camera, target, offset);
                }
            }

            _ => {}
        }
    }
}

fn process_timeline_events_inline(events: &[UIEvent], app: &mut App) {
    let mut timeline_state =
        app.data.ecs_world.resource_mut::<TimelineState>();
    let mut clip_library =
        app.data.ecs_world.resource_mut::<ClipLibrary>();

    let clip_id = timeline_state.current_clip_id;
    let before_clip =
        clip_id.and_then(|id| clip_library.get(id).cloned());

    let modified = timeline_process_events(
        events,
        &mut timeline_state,
        &mut *clip_library,
    );

    if modified {
        if let (Some(cid), Some(before)) = (clip_id, before_clip) {
            if let Some(after) = clip_library.get(cid).cloned() {
                if app
                    .data
                    .ecs_world
                    .contains_resource::<EditHistory>()
                {
                    let mut edit_history = app
                        .data
                        .ecs_world
                        .resource_mut::<EditHistory>();
                    edit_history.push_clip_edit(
                        cid,
                        before,
                        after,
                        "timeline clip edit",
                    );
                }
            }
        }
    }
}

fn process_clip_instance_events_inline(
    events: &[UIEvent],
    app: &mut App,
) {
    let schedule_snapshots = collect_clip_schedule_snapshots(events, app);

    process_clip_instance_events(events, &mut app.data.ecs_world);

    record_schedule_changes(schedule_snapshots, app);
}

fn collect_clip_schedule_snapshots(
    events: &[UIEvent],
    app: &App,
) -> Vec<(crate::ecs::world::Entity, ClipSchedule)> {
    use std::collections::HashSet;

    let mut entities = HashSet::new();
    for event in events {
        match event {
            UIEvent::ClipInstanceMove { entity, .. }
            | UIEvent::ClipInstanceTrimStart { entity, .. }
            | UIEvent::ClipInstanceTrimEnd { entity, .. }
            | UIEvent::ClipInstanceToggleMute { entity, .. }
            | UIEvent::ClipInstanceDelete { entity, .. }
            | UIEvent::ClipInstanceSetWeight { entity, .. }
            | UIEvent::ClipInstanceSetBlendMode { entity, .. }
            | UIEvent::ClipGroupCreate { entity, .. }
            | UIEvent::ClipGroupDelete { entity, .. }
            | UIEvent::ClipGroupAddInstance { entity, .. }
            | UIEvent::ClipGroupRemoveInstance { entity, .. }
            | UIEvent::ClipGroupToggleMute { entity, .. }
            | UIEvent::ClipGroupSetWeight { entity, .. } => {
                entities.insert(*entity);
            }
            _ => {}
        }
    }

    entities
        .into_iter()
        .filter_map(|entity| {
            app.data
                .ecs_world
                .get_component::<ClipSchedule>(entity)
                .cloned()
                .map(|s| (entity, s))
        })
        .collect()
}

fn record_schedule_changes(
    snapshots: Vec<(crate::ecs::world::Entity, ClipSchedule)>,
    app: &mut App,
) {
    if snapshots.is_empty() {
        return;
    }

    if !app.data.ecs_world.contains_resource::<EditHistory>() {
        return;
    }

    for (entity, before) in snapshots {
        let after = app
            .data
            .ecs_world
            .get_component::<ClipSchedule>(entity)
            .cloned();

        if let Some(after) = after {
            let changed = before.instances.len() != after.instances.len()
                || before.groups.len() != after.groups.len()
                || format!("{:?}", before) != format!("{:?}", after);

            if changed {
                let mut edit_history =
                    app.data.ecs_world.resource_mut::<EditHistory>();
                edit_history.push_schedule_edit(
                    entity,
                    before,
                    after,
                    "clip schedule edit",
                );
            }
        }
    }
}

fn process_buffer_events_inline(events: &[UIEvent], app: &mut App) {
    for event in events {
        match event {
            UIEvent::TimelineCaptureBuffer => {
                let timeline_state =
                    app.data.ecs_world.resource::<TimelineState>();
                let clip_library =
                    app.data.ecs_world.resource::<ClipLibrary>();
                let curve_editor =
                    app.data.ecs_world.resource::<CurveEditorState>();
                let mut curve_buffer =
                    app.data.ecs_world.resource_mut::<CurveEditorBuffer>();

                if let Some(bone_id) = curve_editor.selected_bone_id {
                    if let Some(clip_id) = timeline_state.current_clip_id {
                        if let Some(clip) = clip_library.get(clip_id) {
                            curve_buffer.capture_buffer(
                                clip,
                                bone_id,
                                &curve_editor.visible_curves,
                                clip.duration,
                                100,
                            );
                        }
                    }
                }
            }

            UIEvent::TimelineSwapBuffer => {
                let curve_editor =
                    app.data.ecs_world.resource::<CurveEditorState>();
                let timeline_state =
                    app.data.ecs_world.resource::<TimelineState>();
                let mut clip_library =
                    app.data.ecs_world.resource_mut::<ClipLibrary>();
                let mut curve_buffer =
                    app.data.ecs_world.resource_mut::<CurveEditorBuffer>();

                if let Some(bone_id) = curve_editor.selected_bone_id {
                    if let Some(clip_id) = timeline_state.current_clip_id {
                        if let Some(clip) = clip_library.get_mut(clip_id) {
                            curve_buffer.swap_buffer(clip, bone_id);
                        }
                    }
                }
            }

            _ => {}
        }
    }
}

fn process_keyframe_clipboard_events_inline(
    events: &[UIEvent],
    app: &mut App,
) {
    let timeline_state = app.data.ecs_world.resource::<TimelineState>();
    let mut clip_library = app.data.ecs_world.resource_mut::<ClipLibrary>();
    let mut copy_buffer =
        app.data.ecs_world.resource_mut::<KeyframeCopyBuffer>();

    process_keyframe_clipboard_events(
        events,
        &*timeline_state,
        &mut *clip_library,
        &mut *copy_buffer,
    );
}

fn process_clip_browser_events_inline(
    events: &[UIEvent],
    app: &mut App,
) {
    for event in events {
        match event {
            UIEvent::ClipInstanceAdd {
                entity,
                source_id,
                start_time,
            } => {
                let duration = {
                    let clip_library =
                        app.data.ecs_world.resource::<ClipLibrary>();
                    clip_library
                        .get(*source_id)
                        .map(|c| c.duration)
                        .unwrap_or(1.0)
                };

                if let Some(schedule) = app
                    .data
                    .ecs_world
                    .get_component_mut::<ClipSchedule>(*entity)
                {
                    let mut inst = crate::animation::editable::ClipInstance::new(
                        0, *source_id, duration,
                    );
                    inst.start_time = *start_time;
                    schedule.add_instance(*source_id, duration);

                    if let Some(last) = schedule.instances.last_mut()
                    {
                        last.start_time = *start_time;
                    }
                }
            }

            UIEvent::ClipBrowserCreateEmpty => {
                let mut clip_library =
                    app.data.ecs_world.resource_mut::<ClipLibrary>();
                let id = clip_library
                    .create_empty("New Clip".to_string());
                crate::log!(
                    "Created empty clip (id={})",
                    id
                );
            }

            UIEvent::ClipBrowserDuplicate(source_id) => {
                let mut clip_library =
                    app.data.ecs_world.resource_mut::<ClipLibrary>();
                if let Some(original) =
                    clip_library.get(*source_id).cloned()
                {
                    let mut duplicate = original;
                    duplicate.name =
                        format!("{} (copy)", duplicate.name);
                    let new_id =
                        clip_library.register_clip(duplicate);
                    crate::log!(
                        "Duplicated clip {} -> {}",
                        source_id,
                        new_id
                    );
                }
            }

            UIEvent::ClipBrowserDelete(source_id) => {
                let ref_count = count_source_references(
                    *source_id,
                    &app.data.ecs_world,
                );
                if ref_count == 0 {
                    let mut clip_library = app
                        .data
                        .ecs_world
                        .resource_mut::<ClipLibrary>();
                    clip_library.remove(*source_id);
                    crate::log!(
                        "Deleted clip (id={})",
                        source_id
                    );
                } else {
                    crate::log!(
                        "Cannot delete clip {}: {} references remain",
                        source_id,
                        ref_count
                    );
                }
            }

            _ => {}
        }
    }
}

fn count_source_references(
    source_id: crate::animation::editable::SourceClipId,
    world: &crate::ecs::world::World,
) -> usize {
    let entities = world.component_entities::<ClipSchedule>();
    let mut count = 0;
    for entity in entities {
        if let Some(schedule) =
            world.get_component::<ClipSchedule>(entity)
        {
            count += schedule
                .instances
                .iter()
                .filter(|i| i.source_id == source_id)
                .count();
        }
    }
    count
}

fn process_edit_history_events_inline(
    events: &[UIEvent],
    app: &mut App,
) {
    for event in events {
        match event {
            UIEvent::Undo => {
                if !app
                    .data
                    .ecs_world
                    .contains_resource::<EditHistory>()
                {
                    return;
                }

                let mut edit_history =
                    app.data.ecs_world.resource_mut::<EditHistory>();
                if !edit_history.can_undo() {
                    return;
                }
                let edit_history_ptr: *mut EditHistory =
                    &mut *edit_history;
                drop(edit_history);

                let mut clip_library =
                    app.data.ecs_world.resource_mut::<ClipLibrary>();
                let clip_library_ptr: *mut ClipLibrary =
                    &mut *clip_library;
                drop(clip_library);

                unsafe {
                    apply_undo(
                        &mut *edit_history_ptr,
                        &mut *clip_library_ptr,
                        &mut app.data.ecs_world,
                    );
                }
            }

            UIEvent::Redo => {
                if !app
                    .data
                    .ecs_world
                    .contains_resource::<EditHistory>()
                {
                    return;
                }

                let mut edit_history =
                    app.data.ecs_world.resource_mut::<EditHistory>();
                if !edit_history.can_redo() {
                    return;
                }
                let edit_history_ptr: *mut EditHistory =
                    &mut *edit_history;
                drop(edit_history);

                let mut clip_library =
                    app.data.ecs_world.resource_mut::<ClipLibrary>();
                let clip_library_ptr: *mut ClipLibrary =
                    &mut *clip_library;
                drop(clip_library);

                unsafe {
                    apply_redo(
                        &mut *edit_history_ptr,
                        &mut *clip_library_ptr,
                        &mut app.data.ecs_world,
                    );
                }
            }

            _ => {}
        }
    }
}

fn process_scene_events_inline(events: &[UIEvent], app: &mut App) {
    for event in events {
        if let UIEvent::SaveScene = event {
            let scene_path =
                std::path::PathBuf::from("assets/scenes/default.scene.ron");

            match crate::scene::save_scene(
                &scene_path,
                &app.data.ecs_world,
            ) {
                Ok(()) => {
                    crate::log!("Scene saved to {:?}", scene_path);
                }
                Err(e) => {
                    crate::log!("Failed to save scene: {:?}", e);
                }
            }
        }
    }
}

fn process_debug_constraint_events_inline(
    events: &[UIEvent],
    app: &mut App,
) {
    use crate::ecs::systems::debug_constraint_systems::{
        clear_test_constraints, create_test_constraints,
    };

    for event in events {
        match event {
            UIEvent::CreateTestConstraints => {
                create_test_constraints(
                    &mut app.data.ecs_world,
                    &app.data.ecs_assets,
                );
            }
            UIEvent::ClearTestConstraints => {
                clear_test_constraints(&mut app.data.ecs_world);
            }
            _ => {}
        }
    }
}

fn process_constraint_edit_events_inline(
    events: &[UIEvent],
    app: &mut App,
) {
    use crate::ecs::systems::constraint_edit_systems::{
        handle_constraint_add, handle_constraint_remove,
        handle_constraint_update,
    };

    for event in events {
        match event {
            UIEvent::ConstraintAdd {
                entity,
                constraint_type_index,
            } => {
                handle_constraint_add(
                    &mut app.data.ecs_world,
                    *entity,
                    *constraint_type_index,
                    &app.data.ecs_assets,
                );
            }
            UIEvent::ConstraintRemove {
                entity,
                constraint_id,
            } => {
                handle_constraint_remove(
                    &mut app.data.ecs_world,
                    *entity,
                    *constraint_id,
                );
            }
            UIEvent::ConstraintUpdate {
                entity,
                constraint_id,
                constraint,
            } => {
                handle_constraint_update(
                    &mut app.data.ecs_world,
                    *entity,
                    *constraint_id,
                    constraint,
                );
            }
            _ => {}
        }
    }
}
