use std::time::Instant;

use imgui::MouseButton;
use winit::event::{ElementState, Event, WindowEvent};
use winit::keyboard::{Key, NamedKey};

use super::platform::System;
use super::ui::{
    build_click_debug_overlay, build_clip_browser_window, build_curve_editor_window,
    build_debug_window, build_hierarchy_window, build_inspector_window, build_timeline_window,
    build_viewport_window, collect_clip_track_snapshot, handle_splitters, CurveEditorState,
    DebugWindowState, LayoutSnapshot,
};
use crate::app::{App, GUIData};
use crate::debugview::gizmo::{BoneGizmoData, BoneSelectionState};
use crate::debugview::RayTracingDebugState;
use crate::ecs::component::ClipSchedule;
use crate::ecs::events::UIEvent;
use crate::ecs::resource::Camera;
use crate::ecs::resource::ClipLibrary;
use crate::ecs::resource::CurveEditorBuffer;
use crate::ecs::resource::KeyframeCopyBuffer;
use crate::ecs::resource::PanelLayout;
use crate::ecs::resource::{ClipBrowserState, EditHistory};
use crate::ecs::resource::{HierarchyState, SceneState, TimelineState};
use crate::ecs::systems::{
    apply_redo, apply_undo, camera_move_to_look_at, collapse_entity, expand_entity,
    hierarchy_collapse_bone, hierarchy_deselect_all, hierarchy_deselect_bone,
    hierarchy_expand_bone, hierarchy_select, hierarchy_select_bone, hierarchy_toggle_selection,
    process_clip_instance_events, process_keyframe_clipboard_events, rename_entity,
    timeline_process_events, update_entity_scale, update_entity_translation, update_entity_visible,
};
use crate::ecs::world::Transform;
use crate::ecs::{process_ui_events_with_events_simple, DeferredAction, UIEventQueue};

fn update_mouse_input(gui_data: &mut GUIData, ui: &imgui::Ui) {
    gui_data.is_left_clicked = false;
    gui_data.is_right_clicked = false;
    gui_data.is_wheel_clicked = false;

    let io = ui.io();
    gui_data.mouse_pos = io.mouse_pos;

    let allow_input = !gui_data.imgui_wants_mouse || gui_data.viewport_hovered;
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
                                        let mut ui_events =
                                            app.data.ecs_world.resource_mut::<UIEventQueue>();
                                        ui_events.send(UIEvent::SaveScene);
                                    }
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

    build_ui_windows(ui, app, gui_data, &mut debug_state);

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
) {
    let display_size = ui.io().display_size;

    let layout_snapshot = {
        let mut panel_layout = app.data.ecs_world.resource_mut::<PanelLayout>();
        panel_layout.clamp_to_display(display_size[0], display_size[1]);
        LayoutSnapshot::from_layout(&panel_layout, display_size)
    };

    {
        let mut ui_events = app.data.ecs_world.resource_mut::<UIEventQueue>();
        build_debug_window(
            ui,
            &mut *ui_events,
            debug_state,
            gui_data,
            &app.data.ecs_world,
            &layout_snapshot,
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
            &layout_snapshot,
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
            &layout_snapshot,
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
            &layout_snapshot,
        );
    }

    {
        let texture_id = imgui::TextureId::new(app.data.viewport.texture_id());
        let current_size = [
            app.data.viewport.width as f32,
            app.data.viewport.height as f32,
        ];
        let viewport_info = build_viewport_window(ui, texture_id, current_size, &layout_snapshot);

        app.data.viewport.focused = viewport_info.focused;
        app.data.viewport.hovered = viewport_info.hovered;
        gui_data.viewport_focused = viewport_info.focused;
        gui_data.viewport_hovered = viewport_info.hovered;
        gui_data.viewport_position = viewport_info.position;
        gui_data.viewport_size = viewport_info.size;

        let new_width = viewport_info.size[0] as u32;
        let new_height = viewport_info.size[1] as u32;
        if new_width > 0
            && new_height > 0
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
        build_timeline_window(
            ui,
            &mut *ui_events,
            &mut *timeline_state,
            &*clip_library,
            &mut *curve_editor,
            &clip_track_snapshot,
            &layout_snapshot,
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
        let mut panel_layout = app.data.ecs_world.resource_mut::<PanelLayout>();
        handle_splitters(ui, &mut panel_layout, &layout_snapshot);
    }
}

unsafe fn process_ui_events_and_render_frame(
    app: &mut App,
    gui_data: &mut GUIData,
    window: &winit::window::Window,
    draw_data: &imgui::DrawData,
) {
    let deferred_actions = {
        let events: Vec<_> = {
            if let Some(mut ui_events) = app.data.ecs_world.get_resource_mut::<UIEventQueue>() {
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
            process_keyframe_clipboard_events_inline(&events, app);
            process_buffer_events_inline(&events, app);
            process_clip_instance_events_inline(&events, app);
            process_clip_browser_events_inline(&events, app);
            process_edit_history_events_inline(&events, app);
            process_scene_events_inline(&events, app);
            process_debug_constraint_events_inline(&events, app);
            process_constraint_edit_events_inline(&events, app);
            process_constraint_bake_events_inline(&events, app);
            process_spring_bone_bake_events_inline(&events, app);
            process_spring_bone_edit_events_inline(&events, app);
            #[cfg(feature = "ml")]
            process_curve_suggestion_events_inline(&events, app);
            #[cfg(feature = "text-to-motion")]
            process_text_to_motion_events_inline(&events, app);

            let model_bounds = app.data.graphics_resources.calculate_model_bounds();
            let world = &app.data.ecs_world;
            let mut camera = world.resource_mut::<Camera>();
            let mut rt_debug = world.resource_mut::<RayTracingDebugState>();
            process_ui_events_with_events_simple(events, &mut camera, &mut rt_debug, model_bounds)
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

fn process_hierarchy_events_inline(events: &[UIEvent], app: &mut App) {
    use cgmath::Vector3;

    for event in events {
        match event {
            UIEvent::SelectEntity(entity) => {
                let mut hierarchy_state = app.data.ecs_world.resource_mut::<HierarchyState>();
                hierarchy_select(&mut hierarchy_state, *entity);
            }

            UIEvent::DeselectAll => {
                let mut hierarchy_state = app.data.ecs_world.resource_mut::<HierarchyState>();
                hierarchy_deselect_all(&mut hierarchy_state);
            }

            UIEvent::ToggleEntitySelection(entity) => {
                let mut hierarchy_state = app.data.ecs_world.resource_mut::<HierarchyState>();
                hierarchy_toggle_selection(&mut hierarchy_state, *entity);
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

            UIEvent::SetHierarchyDisplayMode(mode) => {
                let mut hierarchy_state = app.data.ecs_world.resource_mut::<HierarchyState>();
                hierarchy_state.display_mode = *mode;
            }

            UIEvent::SelectBone(bone_id) => {
                let bone_idx = *bone_id as usize;

                let descendants: Vec<usize> = app
                    .data
                    .ecs_assets
                    .skeletons
                    .values()
                    .next()
                    .map(|skel_asset| {
                        skel_asset
                            .skeleton
                            .collect_descendants(*bone_id)
                            .into_iter()
                            .map(|id| id as usize)
                            .collect()
                    })
                    .unwrap_or_default();

                {
                    let mut hierarchy_state = app.data.ecs_world.resource_mut::<HierarchyState>();
                    hierarchy_select_bone(&mut hierarchy_state, *bone_id);
                }

                if let Some(mut selection) =
                    app.data.ecs_world.get_resource_mut::<BoneSelectionState>()
                {
                    selection.selected_bone_indices.clear();
                    selection.selected_bone_indices.insert(bone_idx);
                    for desc_idx in descendants {
                        selection.selected_bone_indices.insert(desc_idx);
                    }
                    selection.active_bone_index = Some(bone_idx);
                }
            }

            UIEvent::DeselectBone => {
                {
                    let mut hierarchy_state = app.data.ecs_world.resource_mut::<HierarchyState>();
                    hierarchy_deselect_bone(&mut hierarchy_state);
                }

                if let Some(mut selection) =
                    app.data.ecs_world.get_resource_mut::<BoneSelectionState>()
                {
                    selection.selected_bone_indices.clear();
                    selection.active_bone_index = None;
                }
            }

            UIEvent::ExpandBone(bone_id) => {
                let mut hierarchy_state = app.data.ecs_world.resource_mut::<HierarchyState>();
                hierarchy_expand_bone(&mut hierarchy_state, *bone_id);
            }

            UIEvent::CollapseBone(bone_id) => {
                let mut hierarchy_state = app.data.ecs_world.resource_mut::<HierarchyState>();
                hierarchy_collapse_bone(&mut hierarchy_state, *bone_id);
            }

            UIEvent::SetEntityVisible(entity, visible) => {
                update_entity_visible(&mut app.data.ecs_world, *entity, *visible);
            }

            UIEvent::SetEntityTranslation(entity, translation) => {
                update_entity_translation(&mut app.data.ecs_world, *entity, *translation);
            }

            UIEvent::SetEntityRotation(entity, rotation) => {
                if let Some(transform) = app.data.ecs_world.get_component_mut::<Transform>(*entity)
                {
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

            UIEvent::SetBoneDisplayStyle(style) => {
                if let Some(mut bone_gizmo) = app.data.ecs_world.get_resource_mut::<BoneGizmoData>()
                {
                    bone_gizmo.display_style = *style;
                }
            }

            UIEvent::SetBoneInFront(in_front) => {
                if let Some(mut bone_gizmo) = app.data.ecs_world.get_resource_mut::<BoneGizmoData>()
                {
                    bone_gizmo.in_front = *in_front;
                }
            }

            UIEvent::SetBoneDistanceScaling(enabled) => {
                if let Some(mut bone_gizmo) = app.data.ecs_world.get_resource_mut::<BoneGizmoData>()
                {
                    bone_gizmo.distance_scaling_enabled = *enabled;
                }
            }

            UIEvent::SetBoneDistanceScaleFactor(factor) => {
                if let Some(mut bone_gizmo) = app.data.ecs_world.get_resource_mut::<BoneGizmoData>()
                {
                    bone_gizmo.distance_scaling_factor = *factor;
                }
            }

            _ => {}
        }
    }
}

fn process_timeline_events_inline(events: &[UIEvent], app: &mut App) {
    let mut timeline_state = app.data.ecs_world.resource_mut::<TimelineState>();
    let mut clip_library = app.data.ecs_world.resource_mut::<ClipLibrary>();

    let clip_id = timeline_state.current_clip_id;
    let before_clip = clip_id.and_then(|id| clip_library.get(id).cloned());

    let modified = timeline_process_events(events, &mut timeline_state, &mut *clip_library);

    if modified {
        if let (Some(cid), Some(before)) = (clip_id, before_clip) {
            if let Some(after) = clip_library.get(cid).cloned() {
                if app.data.ecs_world.contains_resource::<EditHistory>() {
                    let mut edit_history = app.data.ecs_world.resource_mut::<EditHistory>();
                    edit_history.push_clip_edit(cid, before, after, "timeline clip edit");
                }
            }
        }
    }

    drop(clip_library);
    drop(timeline_state);

    if modified {
        transition_to_baked_override_if_needed(app);
    }

    for event in events {
        if let UIEvent::TimelineSelectClip(source_id) = event {
            let lib = app.data.ecs_world.resource::<ClipLibrary>();
            let duration = lib.get(*source_id).map(|c| c.duration).unwrap_or(1.0);
            let asset_id = lib.get_asset_id_for_source(*source_id);
            crate::log!(
                "[ClipSelect] source_id={}, asset_id={:?}, duration={:.3}",
                source_id,
                asset_id,
                duration,
            );
            drop(lib);

            let schedule_entities = app.data.ecs_world.component_entities::<ClipSchedule>();
            for entity in &schedule_entities {
                if let Some(schedule) = app
                    .data
                    .ecs_world
                    .get_component_mut::<ClipSchedule>(*entity)
                {
                    if let Some(first) = schedule.instances.first_mut() {
                        first.source_id = *source_id;
                        first.clip_out = duration;
                    }
                }
            }
        }
    }
}

fn process_clip_instance_events_inline(events: &[UIEvent], app: &mut App) {
    let schedule_snapshots = collect_clip_schedule_snapshots(events, app);

    process_clip_instance_events(events, &mut app.data.ecs_world);

    for event in events {
        if let UIEvent::ClipInstanceSelect {
            entity,
            instance_id,
        } = event
        {
            let source_id = app
                .data
                .ecs_world
                .get_component::<ClipSchedule>(*entity)
                .and_then(|schedule| {
                    schedule
                        .instances
                        .iter()
                        .find(|i| i.instance_id == *instance_id)
                        .map(|i| i.source_id)
                });
        }
    }

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
                let mut edit_history = app.data.ecs_world.resource_mut::<EditHistory>();
                edit_history.push_schedule_edit(entity, before, after, "clip schedule edit");
            }
        }
    }
}

fn process_buffer_events_inline(events: &[UIEvent], app: &mut App) {
    for event in events {
        match event {
            UIEvent::TimelineCaptureBuffer => {
                let timeline_state = app.data.ecs_world.resource::<TimelineState>();
                let clip_library = app.data.ecs_world.resource::<ClipLibrary>();
                let curve_editor = app.data.ecs_world.resource::<CurveEditorState>();
                let mut curve_buffer = app.data.ecs_world.resource_mut::<CurveEditorBuffer>();

                if let Some(bone_id) = curve_editor.selected_bone_id {
                    if let Some(clip_id) = timeline_state.current_clip_id {
                        if let Some(clip) = clip_library.get(clip_id) {
                            crate::ecs::systems::curve_editor_capture_buffer(
                                &mut curve_buffer,
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
                let curve_editor = app.data.ecs_world.resource::<CurveEditorState>();
                let timeline_state = app.data.ecs_world.resource::<TimelineState>();
                let mut clip_library = app.data.ecs_world.resource_mut::<ClipLibrary>();
                let mut curve_buffer = app.data.ecs_world.resource_mut::<CurveEditorBuffer>();

                if let Some(bone_id) = curve_editor.selected_bone_id {
                    if let Some(clip_id) = timeline_state.current_clip_id {
                        if let Some(clip) = clip_library.get_mut(clip_id) {
                            crate::ecs::systems::curve_editor_swap_buffer(
                                &mut curve_buffer,
                                clip,
                                bone_id,
                            );
                        }
                    }
                }
            }

            _ => {}
        }
    }
}

fn process_keyframe_clipboard_events_inline(events: &[UIEvent], app: &mut App) {
    let timeline_state = app.data.ecs_world.resource::<TimelineState>();
    let mut clip_library = app.data.ecs_world.resource_mut::<ClipLibrary>();
    let mut copy_buffer = app.data.ecs_world.resource_mut::<KeyframeCopyBuffer>();

    process_keyframe_clipboard_events(
        events,
        &*timeline_state,
        &mut *clip_library,
        &mut *copy_buffer,
    );
}

fn process_clip_browser_events_inline(events: &[UIEvent], app: &mut App) {
    for event in events {
        match event {
            UIEvent::ClipInstanceAdd {
                entity,
                source_id,
                start_time,
            } => {
                let duration = {
                    let clip_library = app.data.ecs_world.resource::<ClipLibrary>();
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
                    let mut inst =
                        crate::animation::editable::ClipInstance::new(0, *source_id, duration);
                    inst.start_time = *start_time;
                    crate::ecs::systems::clip_schedule_systems::clip_schedule_add_instance(
                        schedule, *source_id, duration,
                    );

                    if let Some(last) = schedule.instances.last_mut() {
                        last.start_time = *start_time;
                    }
                }
            }

            UIEvent::ClipBrowserCreateEmpty => {
                let mut clip_library = app.data.ecs_world.resource_mut::<ClipLibrary>();
                let editable = crate::animation::editable::EditableAnimationClip::new(
                    0,
                    "New Clip".to_string(),
                );
                let id =
                    crate::ecs::systems::clip_library_systems::clip_library_register_and_activate(
                        &mut clip_library,
                        &mut app.data.ecs_assets,
                        editable,
                    );
                crate::log!("Created empty clip (id={})", id);
            }

            UIEvent::ClipBrowserDuplicate(source_id) => {
                let mut clip_library = app.data.ecs_world.resource_mut::<ClipLibrary>();
                if let Some(original) = clip_library.get(*source_id).cloned() {
                    let mut duplicate = original;
                    duplicate.name = format!("{} (copy)", duplicate.name);
                    let new_id =
                        crate::ecs::systems::clip_library_systems::clip_library_register_and_activate(
                            &mut clip_library,
                            &mut app.data.ecs_assets,
                            duplicate,
                        );
                    crate::log!("Duplicated clip {} -> {}", source_id, new_id);
                }
            }

            UIEvent::ClipBrowserLoadFromFile => {
                let path = rfd::FileDialog::new()
                    .add_filter("Animation RON", &["anim.ron", "ron"])
                    .pick_file();

                if let Some(path) = path {
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
            }

            UIEvent::ClipBrowserSaveToFile(source_id) => {
                let current_name = {
                    let lib = app.data.ecs_world.resource::<ClipLibrary>();
                    lib.get(*source_id)
                        .map(|c| c.name.clone())
                        .unwrap_or_else(|| "clip".to_string())
                };

                let path = rfd::FileDialog::new()
                    .add_filter("Animation RON", &["anim.ron", "ron"])
                    .set_file_name(format!("{}.anim.ron", current_name))
                    .save_file();

                if let Some(path) = path {
                    let new_name = extract_clip_name_from_path(&path);
                    let mut clip_library = app.data.ecs_world.resource_mut::<ClipLibrary>();

                    if let Some(clip) = clip_library.get_mut(*source_id) {
                        clip.name = new_name.clone();
                        clip.source_path = Some(path.to_string_lossy().to_string());
                    }

                    use crate::ecs::systems::clip_library_systems::clip_library_save_to_file;
                    match clip_library_save_to_file(&clip_library, *source_id, &path) {
                        Ok(()) => {
                            crate::log!("Saved clip '{}' to {:?}", new_name, path);
                        }
                        Err(e) => {
                            crate::log!("Failed to save clip: {:?}", e);
                        }
                    }
                }
            }

            UIEvent::ClipBrowserExportFbx(source_id) => {
                let clip = {
                    let lib = app.data.ecs_world.resource::<ClipLibrary>();
                    lib.get(*source_id).cloned()
                };
                let skeleton = app
                    .data
                    .ecs_assets
                    .skeletons
                    .values()
                    .next()
                    .map(|sa| sa.skeleton.clone());

                if let (Some(clip), Some(skeleton)) = (clip, skeleton) {
                    let default_filename = format!("{}.fbx", clip.name);
                    let path = rfd::FileDialog::new()
                        .add_filter("FBX Binary", &["fbx"])
                        .set_file_name(&default_filename)
                        .save_file();

                    if let Some(path) = path {
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
                            crate::exporter::fbx_exporter::export_full_fbx(
                                fbx_model,
                                Some(&clip),
                                &skeleton,
                                &path,
                            )
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
                }
            }

            UIEvent::ClipBrowserDelete(source_id) => {
                let ref_count = count_source_references(*source_id, &app.data.ecs_world);
                if ref_count == 0 {
                    let mut clip_library = app.data.ecs_world.resource_mut::<ClipLibrary>();
                    clip_library.remove(*source_id);
                    crate::log!("Deleted clip (id={})", source_id);
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

fn extract_clip_name_from_path(path: &std::path::Path) -> String {
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("clip");

    filename
        .strip_suffix(".anim.ron")
        .or_else(|| filename.strip_suffix(".ron"))
        .unwrap_or(filename)
        .to_string()
}

fn count_source_references(
    source_id: crate::animation::editable::SourceClipId,
    world: &crate::ecs::world::World,
) -> usize {
    let entities = world.component_entities::<ClipSchedule>();
    let mut count = 0;
    for entity in entities {
        if let Some(schedule) = world.get_component::<ClipSchedule>(entity) {
            count += schedule
                .instances
                .iter()
                .filter(|i| i.source_id == source_id)
                .count();
        }
    }
    count
}

fn process_edit_history_events_inline(events: &[UIEvent], app: &mut App) {
    for event in events {
        match event {
            UIEvent::Undo => {
                if !app.data.ecs_world.contains_resource::<EditHistory>() {
                    return;
                }

                let mut edit_history = app.data.ecs_world.resource_mut::<EditHistory>();
                if !edit_history.can_undo() {
                    return;
                }
                let edit_history_ptr: *mut EditHistory = &mut *edit_history;
                drop(edit_history);

                let mut clip_library = app.data.ecs_world.resource_mut::<ClipLibrary>();
                let clip_library_ptr: *mut ClipLibrary = &mut *clip_library;
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
                if !app.data.ecs_world.contains_resource::<EditHistory>() {
                    return;
                }

                let mut edit_history = app.data.ecs_world.resource_mut::<EditHistory>();
                if !edit_history.can_redo() {
                    return;
                }
                let edit_history_ptr: *mut EditHistory = &mut *edit_history;
                drop(edit_history);

                let mut clip_library = app.data.ecs_world.resource_mut::<ClipLibrary>();
                let clip_library_ptr: *mut ClipLibrary = &mut *clip_library;
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
            let scene_path = std::path::PathBuf::from("assets/scenes/default.scene.ron");

            match crate::scene::save_scene(&scene_path, &app.data.ecs_world) {
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

fn process_debug_constraint_events_inline(events: &[UIEvent], app: &mut App) {
    use crate::ecs::systems::debug_constraint_systems::{
        clear_test_constraints, create_test_constraints,
    };
    use crate::ecs::systems::debug_spring_bone_systems::{
        clear_spring_bones, create_test_spring_bones,
    };

    for event in events {
        match event {
            UIEvent::CreateTestConstraints => {
                create_test_constraints(&mut app.data.ecs_world, &app.data.ecs_assets);
            }
            UIEvent::ClearTestConstraints => {
                clear_test_constraints(&mut app.data.ecs_world);
            }
            UIEvent::AddTestSpringBones => {
                create_test_spring_bones(&mut app.data.ecs_world, &app.data.ecs_assets);
            }
            UIEvent::ClearSpringBones => {
                let is_baked = app
                    .data
                    .ecs_world
                    .get_resource::<crate::ecs::resource::SpringBoneState>()
                    .map_or(false, |s| s.baked_clip_source_id.is_some());
                if is_baked {
                    handle_spring_bone_discard(app);
                }
                clear_spring_bones(&mut app.data.ecs_world);
            }
            _ => {}
        }
    }
}

fn process_constraint_edit_events_inline(events: &[UIEvent], app: &mut App) {
    use crate::ecs::systems::constraint_edit_systems::{
        handle_constraint_add, handle_constraint_remove, handle_constraint_update,
    };

    for event in events {
        match event {
            UIEvent::ConstraintAdd {
                entity,
                constraint_type_index,
            } => {
                handle_constraint_add(&mut app.data.ecs_world, *entity, *constraint_type_index);
            }
            UIEvent::ConstraintRemove {
                entity,
                constraint_id,
            } => {
                handle_constraint_remove(&mut app.data.ecs_world, *entity, *constraint_id);
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

fn process_constraint_bake_events_inline(events: &[UIEvent], app: &mut App) {
    use crate::ecs::component::ConstraintSet;
    use crate::ecs::resource::{ClipLibrary, TimelineState};
    use crate::ecs::systems::constraint_bake_systems::{
        constraint_bake_evaluate, constraint_bake_register, constraint_bake_rest_pose,
    };

    for event in events {
        let UIEvent::ConstraintBakeToKeyframes { entity, sample_fps } = event else {
            continue;
        };

        let skeleton = match app.data.ecs_assets.skeletons.values().next() {
            Some(skel_asset) => skel_asset.skeleton.clone(),
            None => {
                crate::log!("Bake failed: no skeleton found");
                continue;
            }
        };

        let constraint_set = match app.data.ecs_world.get_component::<ConstraintSet>(*entity) {
            Some(set) => set.clone(),
            None => {
                crate::log!("Bake failed: no ConstraintSet on entity");
                continue;
            }
        };

        let timeline_state = app.data.ecs_world.resource::<TimelineState>();
        let clip_id = timeline_state.current_clip_id;
        let looping = timeline_state.looping;
        drop(timeline_state);

        let mut baked = if let Some(source_id) = clip_id {
            let clip_library = app.data.ecs_world.resource::<ClipLibrary>();
            match clip_library.get(source_id) {
                Some(editable) => {
                    let anim_clip = editable.to_animation_clip();
                    let source_name = editable.name.clone();
                    drop(clip_library);

                    let mut result = constraint_bake_evaluate(
                        &anim_clip,
                        &skeleton,
                        &constraint_set,
                        *sample_fps,
                        looping,
                    );
                    result.name = format!("{}_baked", source_name);
                    result
                }
                None => {
                    drop(clip_library);
                    constraint_bake_rest_pose(&skeleton, &constraint_set)
                }
            }
        } else {
            constraint_bake_rest_pose(&skeleton, &constraint_set)
        };

        baked.name = if baked.name.is_empty() {
            "baked".to_string()
        } else {
            baked.name
        };

        let mut clip_library = app.data.ecs_world.resource_mut::<ClipLibrary>();
        let new_id = constraint_bake_register(&mut clip_library, &mut app.data.ecs_assets, baked);
        crate::log!("Baked constraints to new clip (id={})", new_id);
    }
}

fn process_spring_bone_bake_events_inline(events: &[UIEvent], app: &mut App) {
    for event in events {
        match event {
            UIEvent::SpringBoneBake => {
                handle_spring_bone_bake(app);
            }
            UIEvent::SpringBoneDiscardBake => {
                handle_spring_bone_discard(app);
            }
            UIEvent::SpringBoneSaveBake => {
                handle_spring_bone_save(app);
            }
            UIEvent::SpringBoneRebake => {
                handle_spring_bone_discard(app);
                handle_spring_bone_bake(app);
            }
            _ => {}
        }
    }
}

fn transition_to_baked_override_if_needed(app: &mut App) {
    use crate::ecs::resource::{SpringBoneMode, SpringBoneState};

    if let Some(mut state) = app.data.ecs_world.get_resource_mut::<SpringBoneState>() {
        if state.mode == SpringBoneMode::Baked {
            state.mode = SpringBoneMode::BakedOverride;
            crate::log!("Spring bone mode: Baked -> BakedOverride (manual edit detected)");
        }
    }
}

fn handle_spring_bone_bake(app: &mut App) {
    use crate::ecs::component::{ConstraintSet, SpringBoneSetup, WithSpringBone};
    use crate::ecs::resource::{ClipLibrary, TimelineState};
    use crate::ecs::resource::{SpringBoneMode, SpringBoneState};
    use crate::ecs::systems::clip_library_systems::clip_library_register_and_activate;
    use crate::ecs::systems::spring_bone_bake_systems::{
        merge_bake_into_clip, spring_bone_bake, BakeConfig,
    };

    let skeleton = match app.data.ecs_assets.skeletons.values().next() {
        Some(skel_asset) => skel_asset.skeleton.clone(),
        None => {
            crate::log!("Spring bone bake failed: no skeleton found");
            return;
        }
    };

    let spring_entity = app
        .data
        .ecs_world
        .iter_components::<WithSpringBone>()
        .next()
        .map(|(entity, _)| entity);

    let Some(entity) = spring_entity else {
        crate::log!("Spring bone bake failed: no WithSpringBone entity");
        return;
    };

    let setup = match app.data.ecs_world.get_component::<SpringBoneSetup>(entity) {
        Some(s) => s.clone(),
        None => {
            crate::log!("Spring bone bake failed: no SpringBoneSetup");
            return;
        }
    };

    let constraints = app
        .data
        .ecs_world
        .get_component::<ConstraintSet>(entity)
        .cloned();

    let timeline_state = app.data.ecs_world.resource::<TimelineState>();
    let source_id = timeline_state.current_clip_id;
    let looping = timeline_state.looping;
    drop(timeline_state);

    let clip_library = app.data.ecs_world.resource::<ClipLibrary>();
    let (anim_clip, source_editable) = match source_id.and_then(|id| clip_library.get(id)) {
        Some(editable) => (editable.to_animation_clip(), editable.clone()),
        None => {
            drop(clip_library);
            crate::log!("Spring bone bake failed: no current clip");
            return;
        }
    };
    drop(clip_library);

    let config = BakeConfig {
        start_time: 0.0,
        end_time: anim_clip.duration,
        sample_rate: 30.0,
    };

    let bake_result = spring_bone_bake(
        &config,
        &setup,
        &skeleton,
        &anim_clip,
        constraints.as_ref(),
        looping,
    );

    let mut merged = source_editable;
    merge_bake_into_clip(&mut merged, &bake_result, &skeleton);
    merged.name = format!("{}_spring_baked", merged.name);

    crate::log!(
        "[BakeDebug] bake_result: baked_bone_ids={:?}, clip_tracks={}",
        bake_result.baked_bone_ids,
        bake_result.clip.tracks.len()
    );
    crate::log!(
        "[BakeDebug] merged clip: name={}, tracks={}, duration={}",
        merged.name,
        merged.tracks.len(),
        merged.duration
    );

    let mut clip_library = app.data.ecs_world.resource_mut::<ClipLibrary>();
    let new_id =
        clip_library_register_and_activate(&mut clip_library, &mut app.data.ecs_assets, merged);
    drop(clip_library);

    let mut updated_count = 0;
    let schedule_entities = app.data.ecs_world.component_entities::<ClipSchedule>();
    crate::log!(
        "[BakeDebug] ClipSchedule entities count={}, original source_id={:?}",
        schedule_entities.len(),
        source_id
    );
    for sched_entity in &schedule_entities {
        if let Some(schedule) = app
            .data
            .ecs_world
            .get_component_mut::<ClipSchedule>(*sched_entity)
        {
            if let Some(first) = schedule.instances.first_mut() {
                crate::log!(
                    "[BakeDebug]   entity {:?}: schedule source_id={}, match={}",
                    sched_entity,
                    first.source_id,
                    Some(first.source_id) == source_id
                );
                if Some(first.source_id) == source_id {
                    first.source_id = new_id;
                    updated_count += 1;
                }
            }
        }
    }
    crate::log!(
        "[BakeDebug] updated {} ClipSchedule(s) to new source_id={}",
        updated_count,
        new_id
    );

    let baked_bone_ids = bake_result.baked_bone_ids.clone();

    let mut spring_state = app.data.ecs_world.resource_mut::<SpringBoneState>();
    spring_state.mode = SpringBoneMode::Baked;
    spring_state.baked_clip_source_id = Some(new_id);
    spring_state.baked_bone_ids = bake_result.baked_bone_ids;
    spring_state.original_clip_source_id = source_id;

    let mut timeline_state = app.data.ecs_world.resource_mut::<TimelineState>();
    timeline_state.current_clip_id = Some(new_id);
    timeline_state.baked_bone_ids = baked_bone_ids;

    crate::log!("Spring bone baked to new clip (id={})", new_id);
}

fn handle_spring_bone_discard(app: &mut App) {
    use crate::ecs::resource::ClipLibrary;
    use crate::ecs::resource::{SpringBoneMode, SpringBoneState};

    let mut spring_state = app.data.ecs_world.resource_mut::<SpringBoneState>();
    let original_id = spring_state.original_clip_source_id;
    let baked_id = spring_state.baked_clip_source_id;

    spring_state.mode = SpringBoneMode::Realtime;
    spring_state.baked_clip_source_id = None;
    spring_state.baked_bone_ids = Vec::new();
    spring_state.original_clip_source_id = None;
    spring_state.initialized = false;
    drop(spring_state);

    if let (Some(orig_id), Some(baked_source_id)) = (original_id, baked_id) {
        let schedule_entities = app.data.ecs_world.component_entities::<ClipSchedule>();
        for entity in &schedule_entities {
            if let Some(schedule) = app
                .data
                .ecs_world
                .get_component_mut::<ClipSchedule>(*entity)
            {
                if let Some(first) = schedule.instances.first_mut() {
                    if first.source_id == baked_source_id {
                        first.source_id = orig_id;
                    }
                }
            }
        }
    }

    if let Some(baked_id) = baked_id {
        let mut clip_library = app.data.ecs_world.resource_mut::<ClipLibrary>();
        if let Some(asset_id) = clip_library.source_to_asset_id.remove(&baked_id) {
            app.data.ecs_assets.animation_clips.remove(&asset_id);
        }
        clip_library.remove(baked_id);
    }

    let mut timeline_state = app.data.ecs_world.resource_mut::<TimelineState>();
    timeline_state.baked_bone_ids.clear();
    if let Some(orig) = original_id {
        timeline_state.current_clip_id = Some(orig);
    }

    crate::log!("Discarded spring bone bake, restored original clip");
}

fn handle_spring_bone_save(app: &mut App) {
    use crate::ecs::resource::ClipLibrary;
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

fn process_spring_bone_edit_events_inline(events: &[UIEvent], app: &mut App) {
    use crate::ecs::component::WithSpringBone;
    use crate::ecs::systems::spring_bone_edit_systems::*;

    let skeleton = app
        .data
        .ecs_assets
        .skeletons
        .values()
        .next()
        .map(|sa| sa.skeleton.clone());

    for event in events {
        match event {
            UIEvent::SpringChainAdd {
                entity,
                root_bone_id,
                chain_length,
            } => {
                if let Some(ref skel) = skeleton {
                    handle_spring_chain_add(
                        &mut app.data.ecs_world,
                        *entity,
                        *root_bone_id,
                        *chain_length,
                        skel,
                    );
                }
            }

            UIEvent::SpringChainRemove { entity, chain_id } => {
                handle_spring_chain_remove(&mut app.data.ecs_world, *entity, *chain_id);
            }

            UIEvent::SpringChainUpdate {
                entity,
                chain_id,
                chain,
            } => {
                handle_spring_chain_update(
                    &mut app.data.ecs_world,
                    *entity,
                    *chain_id,
                    chain.clone(),
                );
            }

            UIEvent::SpringJointUpdate {
                entity,
                chain_id,
                joint_index,
                joint,
            } => {
                handle_spring_joint_update(
                    &mut app.data.ecs_world,
                    *entity,
                    *chain_id,
                    *joint_index,
                    joint.clone(),
                );
            }

            UIEvent::SpringColliderAdd {
                entity,
                bone_id,
                shape,
            } => {
                handle_spring_collider_add(
                    &mut app.data.ecs_world,
                    *entity,
                    *bone_id,
                    shape.clone(),
                );
            }

            UIEvent::SpringColliderRemove {
                entity,
                collider_id,
            } => {
                handle_spring_collider_remove(&mut app.data.ecs_world, *entity, *collider_id);
            }

            UIEvent::SpringColliderUpdate {
                entity,
                collider_id,
                collider,
            } => {
                handle_spring_collider_update(
                    &mut app.data.ecs_world,
                    *entity,
                    *collider_id,
                    collider.clone(),
                );
            }

            UIEvent::SpringColliderGroupAdd { entity, name } => {
                handle_spring_collider_group_add(&mut app.data.ecs_world, *entity, name.clone());
            }

            UIEvent::SpringColliderGroupRemove { entity, group_id } => {
                handle_spring_collider_group_remove(&mut app.data.ecs_world, *entity, *group_id);
            }

            UIEvent::SpringColliderGroupUpdate {
                entity,
                group_id,
                group,
            } => {
                handle_spring_collider_group_update(
                    &mut app.data.ecs_world,
                    *entity,
                    *group_id,
                    group.clone(),
                );
            }

            UIEvent::SpringBoneToggleGizmo(visible) => {
                if let Some(mut gizmo) =
                    app.data
                        .ecs_world
                        .get_resource_mut::<crate::debugview::gizmo::SpringBoneGizmoData>()
                {
                    gizmo.visible = *visible;
                }
            }

            _ => {}
        }
    }
}

#[cfg(feature = "ml")]
fn process_curve_suggestion_events_inline(events: &[UIEvent], app: &mut App) {
    use crate::ecs::resource::{
        BoneNameTokenCache, BoneTopologyCache, CurveSuggestionState, InferenceActorState,
    };
    use crate::ecs::systems::{
        curve_suggestion_apply, curve_suggestion_dismiss, curve_suggestion_submit,
    };
    use crate::ml::CURVE_COPILOT_ACTOR_ID;

    for event in events {
        match event {
            UIEvent::CurveSuggestionRequest {
                bone_id,
                property_type,
            } => {
                let timeline_state = app.data.ecs_world.resource::<TimelineState>();
                let clip_id = timeline_state.current_clip_id;
                let current_time = timeline_state.current_time;
                drop(timeline_state);

                let clip_library = app.data.ecs_world.resource::<ClipLibrary>();
                let clip_info = clip_id
                    .and_then(|id| clip_library.get(id))
                    .and_then(|clip| {
                        clip.tracks
                            .get(bone_id)
                            .map(|track| (track.get_curve(*property_type).clone(), clip.duration))
                    });
                drop(clip_library);

                if let Some((curve, clip_duration)) = clip_info {
                    let topology_cache = app.data.ecs_world.resource::<BoneTopologyCache>();
                    let name_token_cache = app.data.ecs_world.resource::<BoneNameTokenCache>();
                    let mut suggestion_state =
                        app.data.ecs_world.resource_mut::<CurveSuggestionState>();
                    let mut inference_state =
                        app.data.ecs_world.resource_mut::<InferenceActorState>();
                    curve_suggestion_submit(
                        &mut suggestion_state,
                        &mut inference_state,
                        CURVE_COPILOT_ACTOR_ID,
                        &curve,
                        *property_type,
                        *bone_id,
                        clip_duration,
                        current_time,
                        &topology_cache,
                        &name_token_cache,
                    );
                }
            }

            UIEvent::CurveSuggestionAccept => {
                let suggestion = {
                    let state = app.data.ecs_world.resource::<CurveSuggestionState>();
                    state.suggestions.first().cloned()
                };

                if let Some(suggestion) = suggestion {
                    let timeline_state = app.data.ecs_world.resource::<TimelineState>();
                    let clip_id = timeline_state.current_clip_id;
                    drop(timeline_state);

                    if let Some(cid) = clip_id {
                        let mut clip_library = app.data.ecs_world.resource_mut::<ClipLibrary>();
                        if let Some(clip) = clip_library.get_mut(cid) {
                            if let Some(track) = clip.tracks.get_mut(&suggestion.bone_id) {
                                let curve = track.get_curve_mut(suggestion.property_type);
                                curve_suggestion_apply(&suggestion, curve);
                            }
                        }
                    }

                    let mut state = app.data.ecs_world.resource_mut::<CurveSuggestionState>();
                    curve_suggestion_dismiss(&mut state);
                    crate::log!("CurveCopilot: suggestion accepted");
                }
            }

            UIEvent::CurveSuggestionDismiss => {
                let mut state = app.data.ecs_world.resource_mut::<CurveSuggestionState>();
                curve_suggestion_dismiss(&mut state);
                crate::log!("CurveCopilot: suggestion dismissed");
            }

            _ => {}
        }
    }
}

#[cfg(feature = "text-to-motion")]
fn process_text_to_motion_events_inline(events: &[UIEvent], app: &mut App) {
    use crate::ecs::resource::TextToMotionState;
    use crate::ecs::systems::{text_to_motion_cancel, text_to_motion_submit};
    use crate::grpc::GrpcThreadHandle;

    const DEFAULT_ENDPOINT: &str = "http://localhost:50051";

    for event in events {
        match event {
            UIEvent::TextToMotionGenerate {
                prompt,
                duration_seconds,
            } => {
                if !app.data.ecs_world.contains_resource::<GrpcThreadHandle>() {
                    let handle = GrpcThreadHandle::spawn(DEFAULT_ENDPOINT);
                    app.data.ecs_world.insert_resource(handle);
                    crate::log!("TextToMotion: spawned gRPC thread ({})", DEFAULT_ENDPOINT);
                }

                let handle = app.data.ecs_world.get_resource::<GrpcThreadHandle>();
                let mut state = app.data.ecs_world.resource_mut::<TextToMotionState>();

                if let Some(handle) = handle {
                    text_to_motion_submit(&mut state, &*handle, prompt, *duration_seconds);
                }
            }

            UIEvent::TextToMotionApply => {
                let clip = {
                    let mut state = app.data.ecs_world.resource_mut::<TextToMotionState>();
                    state.generated_clip.take()
                };

                if let Some(clip) = clip {
                    let mut clip_library = app.data.ecs_world.resource_mut::<ClipLibrary>();
                    let new_id = crate::ecs::systems::clip_library_systems::clip_library_register_and_activate(
                        &mut clip_library,
                        &mut app.data.ecs_assets,
                        clip,
                    );
                    drop(clip_library);

                    let mut timeline = app.data.ecs_world.resource_mut::<TimelineState>();
                    timeline.current_clip_id = Some(new_id);

                    let mut state = app.data.ecs_world.resource_mut::<TextToMotionState>();
                    text_to_motion_cancel(&mut state);

                    crate::log!("TextToMotion: applied clip (id={})", new_id);
                }
            }

            UIEvent::TextToMotionCancel => {
                let mut state = app.data.ecs_world.resource_mut::<TextToMotionState>();
                text_to_motion_cancel(&mut state);
                crate::log!("TextToMotion: cancelled");
            }

            _ => {}
        }
    }
}
