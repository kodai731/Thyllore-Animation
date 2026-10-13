use super::*;

struct ClipTrackPointer {
    pos: [f32; 2],
    down: bool,
    clicked: bool,
    double_clicked: bool,
}

struct ClipTrackFrame {
    pointer: ClipTrackPointer,
    pixels_per_second: f32,
    timeline_width: f32,
    solo_preview: Option<(crate::ecs::world::Entity, SourceClipId)>,
}

struct ClipTrackRow<'a> {
    entry_idx: usize,
    entry: &'a ClipTrackEntry,
    track_origin: [f32; 2],
    is_solo_owner: bool,
}

pub(super) fn build_clip_tracks_section(
    ui: &imgui::Ui,
    world: &World,
    state: &mut TimelineState,
    interaction: &mut TimelineInteractionState,
    clip_library: &ClipLibrary,
    curve_editor_state: &mut CurveEditorState,
    snapshot: &ClipTrackSnapshot,
    timeline_width: f32,
) {
    let pixels_per_second = PIXELS_PER_SECOND * state.zoom_level;
    let pointer = read_ui_pointer(ui);
    let mouse_clicked = pointer.is_clicked(MouseButton::Left)
        && ui_pointer_begin(ui, world, UiPointerOwnerId::Timeline, PointerRegion::Window);
    let track_pointer = ClipTrackPointer {
        pos: pointer.pos,
        down: pointer.is_down(MouseButton::Left),
        clicked: mouse_clicked,
        double_clicked: mouse_clicked && pointer.is_double_clicked(MouseButton::Left),
    };

    handle_clip_drag_release(
        world,
        interaction,
        track_pointer.pos,
        track_pointer.down,
        pixels_per_second,
    );

    let solo_preview: Option<(crate::ecs::world::Entity, SourceClipId)> =
        (state.preview == ClipPreview::Solo && state.current_clip_id.is_some())
            .then(|| find_preview_owner(world))
            .flatten()
            .zip(state.current_clip_id);
    let frame = ClipTrackFrame {
        pointer: track_pointer,
        pixels_per_second,
        timeline_width,
        solo_preview,
    };

    let mut clicked_any_block = false;
    let mut total_track_rows: f32 = 0.0;

    for (entry_idx, entry) in snapshot.entries.iter().enumerate() {
        let (clicked_row_block, track_height) = build_clip_track_row(
            ui,
            world,
            state,
            interaction,
            clip_library,
            curve_editor_state,
            &frame,
            entry_idx,
            entry,
        );
        clicked_any_block |= clicked_row_block;
        total_track_rows += track_height;
    }

    if frame.pointer.clicked && !clicked_any_block {
        deselect_on_empty_track_click(ui, world, &frame, total_track_rows, snapshot.entries.len());
    }
}

fn build_clip_track_row(
    ui: &imgui::Ui,
    world: &World,
    state: &mut TimelineState,
    interaction: &mut TimelineInteractionState,
    clip_library: &ClipLibrary,
    curve_editor_state: &mut CurveEditorState,
    frame: &ClipTrackFrame,
    entry_idx: usize,
    entry: &ClipTrackEntry,
) -> (bool, f32) {
    build_group_headers(ui, world, entry);

    let cursor_pos = ui.cursor_screen_pos();
    ui.text(&truncate_label(&entry.entity_name, 15));
    ui.same_line_with_pos(TRACK_LABEL_WIDTH);

    let track_origin = [cursor_pos[0] + TRACK_LABEL_WIDTH, cursor_pos[1]];
    let draw_list = ui.get_window_draw_list();

    let solo_clip_for_row = frame
        .solo_preview
        .filter(|(owner, _)| *owner == entry.entity)
        .map(|(_, clip_id)| clip_id);
    let row = ClipTrackRow {
        entry_idx,
        entry,
        track_origin,
        is_solo_owner: solo_clip_for_row.is_some(),
    };

    let lanes = world
        .get_component::<ClipSchedule>(entry.entity)
        .map(|schedule| clip_schedule_assign_lanes(&schedule.instances))
        .unwrap_or_else(|| vec![0; entry.instances.len()]);
    let mut lane_count = lanes.iter().max().map_or(1, |max_lane| max_lane + 1);

    if row.is_solo_owner {
        lane_count += 1;
    }

    let track_height = (lane_count as f32) * CLIP_TRACK_HEIGHT;

    draw_list
        .add_rect(
            track_origin,
            [
                track_origin[0] + frame.timeline_width,
                track_origin[1] + track_height,
            ],
            [0.15, 0.15, 0.2, 1.0],
        )
        .filled(true)
        .build();

    let mut clicked_any_block = false;

    for (inst_idx, inst) in entry.instances.iter().enumerate() {
        clicked_any_block |= build_clip_instance_block(
            ui,
            world,
            state,
            interaction,
            clip_library,
            curve_editor_state,
            &draw_list,
            frame,
            &row,
            inst_idx,
            inst,
            lanes[inst_idx],
        );
    }

    if let Some(solo_source_id) = solo_clip_for_row {
        draw_solo_preview_block(
            &draw_list,
            clip_library,
            solo_source_id,
            track_origin,
            lane_count - 1,
            frame.pixels_per_second,
        );
    }

    ui.dummy([frame.timeline_width, track_height]);

    accept_clip_source_drop(ui, world, frame, entry.entity, track_origin);

    build_clip_instance_properties(ui, world, state, clip_library, entry);

    (clicked_any_block, track_height)
}

fn compute_lane_block_rect(
    track_origin: [f32; 2],
    lane: usize,
    start_offset_x: f32,
    width: f32,
) -> ([f32; 2], [f32; 2]) {
    let lane_y = track_origin[1] + (lane as f32) * CLIP_TRACK_HEIGHT;
    let block_x = track_origin[0] + start_offset_x;
    let block_min = [block_x, lane_y + 2.0];
    let block_max = [block_x + width, lane_y + CLIP_TRACK_HEIGHT - 2.0];
    (block_min, block_max)
}

fn build_clip_instance_block(
    ui: &imgui::Ui,
    world: &World,
    state: &TimelineState,
    interaction: &mut TimelineInteractionState,
    clip_library: &ClipLibrary,
    curve_editor_state: &mut CurveEditorState,
    draw_list: &imgui::DrawListMut,
    frame: &ClipTrackFrame,
    row: &ClipTrackRow,
    inst_idx: usize,
    inst: &ClipInstanceSnapshot,
    lane: usize,
) -> bool {
    let entity = row.entry.entity;
    let pixels_per_second = frame.pixels_per_second;

    let (draw_start, draw_end) = clip_block_display_times(
        interaction,
        entity,
        inst,
        frame.pointer.pos,
        frame.pointer.down,
        pixels_per_second,
    );
    let block_w = ((draw_end - draw_start) * pixels_per_second).max(CLIP_BLOCK_MIN_WIDTH);
    let (block_min, block_max) = compute_lane_block_rect(
        row.track_origin,
        lane,
        draw_start * pixels_per_second,
        block_w,
    );

    let base_color = CLIP_BLOCK_COLORS[row.entry_idx % CLIP_BLOCK_COLORS.len()];
    let mut color = compute_block_color(base_color, inst, interaction, entity);

    if row.is_solo_owner {
        color[3] *= 0.4;
    }

    let border_color = compute_border_color(inst, state, entity);

    draw_clip_block(draw_list, block_min, block_max, color, border_color, inst);

    let hit_block = is_point_in_rect(frame.pointer.pos, block_min, block_max);
    let clicked_block = hit_block && frame.pointer.clicked;

    if hit_block && frame.pointer.double_clicked {
        world.send_command(HierarchyEvent::SelectEntity(entity));
        world.send_command(TimelineEvent::SelectClip(inst.source_id));
        open_curve_editor_for_clip(
            curve_editor_state,
            clip_library,
            inst.source_id,
            row.entry.mesh_bone_id,
        );
    } else if clicked_block {
        world.send_command(HierarchyEvent::SelectEntity(entity));
        world.send_command(ClipInstanceEvent::Select {
            entity,
            instance_id: inst.instance_id,
        });
        begin_clip_drag(
            interaction,
            entity,
            inst,
            frame.pointer.pos,
            block_min,
            block_max,
            pixels_per_second,
        );
    }

    handle_clip_mute_button(ui, world, entity, inst, inst_idx, row.entry_idx);

    clicked_block
}

fn draw_solo_preview_block(
    draw_list: &imgui::DrawListMut,
    clip_library: &ClipLibrary,
    solo_source_id: SourceClipId,
    track_origin: [f32; 2],
    solo_lane: usize,
    pixels_per_second: f32,
) {
    let Some(clip) = clip_library.get(solo_source_id) else {
        return;
    };

    let block_w = (clip.duration * pixels_per_second).max(CLIP_BLOCK_MIN_WIDTH);
    let (block_min, block_max) = compute_lane_block_rect(track_origin, solo_lane, 0.0, block_w);

    draw_list
        .add_rect(block_min, block_max, [1.0, 1.0, 1.0, 1.0])
        .filled(false)
        .build();

    let label_pos = [block_min[0] + 4.0, block_min[1] + 2.0];
    draw_list.add_text(label_pos, [1.0, 1.0, 1.0, 1.0], &clip.name);
}

fn accept_clip_source_drop(
    ui: &imgui::Ui,
    world: &World,
    frame: &ClipTrackFrame,
    entity: crate::ecs::world::Entity,
    track_origin: [f32; 2],
) {
    let Some(target) = ui.drag_drop_target() else {
        return;
    };
    let accepted = target
        .accept_payload::<SourceClipId, _>("CLIP_SOURCE", imgui::DragDropTargetFlags::empty());

    if let Some(Ok(payload)) = accepted {
        let drop_x = frame.pointer.pos[0] - track_origin[0];
        let start_time = (drop_x / frame.pixels_per_second).max(0.0);
        world.send_command(ClipInstanceEvent::Add {
            entity,
            source_id: payload.data,
            start_time,
        });
    }
}

fn deselect_on_empty_track_click(
    ui: &imgui::Ui,
    world: &World,
    frame: &ClipTrackFrame,
    total_track_rows: f32,
    entry_count: usize,
) {
    let section_start_y = ui.cursor_screen_pos()[1]
        - (total_track_rows + entry_count as f32 * ui.text_line_height_with_spacing());

    if frame.pointer.pos[1] >= section_start_y {
        world.send_command(ClipInstanceEvent::Deselect);
    }
}

pub(super) fn build_group_headers(ui: &imgui::Ui, world: &World, entry: &ClipTrackEntry) {
    for group in &entry.groups {
        build_single_group_header(ui, world, entry.entity, group);
    }
}

pub(super) fn build_single_group_header(
    ui: &imgui::Ui,
    world: &World,
    entity: crate::ecs::world::Entity,
    group: &ClipGroupSnapshot,
) {
    let mute_label = if group.muted { "[M]" } else { "[ ]" };
    let header_text = format!(
        "  {} {} (w:{:.2}, {})",
        mute_label,
        group.name,
        group.weight,
        group.instance_ids.len()
    );
    ui.text_colored([0.7, 0.8, 1.0, 1.0], &header_text);

    ui.same_line();
    let mute_btn_id = format!("Mute##grp_{}", group.id);
    if ui.small_button(&mute_btn_id) {
        world.send_command(ClipInstanceEvent::GroupToggleMute {
            entity,
            group_id: group.id,
        });
    }

    ui.same_line();
    ui.set_next_item_width(60.0);
    let mut weight = group.weight;
    let slider_id = format!("##grp_w_{}", group.id);
    if imgui::Drag::new(&slider_id)
        .range(0.0, 1.0)
        .speed(0.01)
        .display_format(numeric_format("%.2f"))
        .build(ui, &mut weight)
    {
        world.send_command(ClipInstanceEvent::GroupSetWeight {
            entity,
            group_id: group.id,
            weight,
        });
    }

    ui.same_line();
    let del_btn_id = format!("X##grp_del_{}", group.id);
    if ui.small_button(&del_btn_id) {
        world.send_command(ClipInstanceEvent::GroupDelete {
            entity,
            group_id: group.id,
        });
    }
}

pub(super) fn build_clip_display_name(name: &str, source_path: Option<&str>) -> String {
    let filename = source_path
        .and_then(|p| std::path::Path::new(p).file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("");

    if filename.is_empty() {
        name.to_string()
    } else {
        format!("{} <{}>", name, filename)
    }
}

pub(super) fn open_curve_editor_for_clip(
    curve_editor_state: &mut CurveEditorState,
    clip_library: &ClipLibrary,
    source_id: SourceClipId,
    mesh_bone_id: Option<BoneId>,
) {
    curve_editor_state.is_open = true;
    curve_editor_state.needs_focus = true;

    if let Some(clip) = clip_library.get(source_id) {
        let previous_bone_exists = curve_editor_state
            .selected_bone_id()
            .is_some_and(|id| clip.tracks.contains_key(&id));

        if !previous_bone_exists {
            let target_bone = mesh_bone_id.filter(|id| clip.tracks.contains_key(id));
            if let Some(bone_id) = target_bone.or_else(|| clip.tracks.keys().min().copied()) {
                curve_editor_state.select_bone(bone_id);
            } else if clip.has_scalar_keyframes() {
                curve_editor_state.select_scalars();
                for curve in &clip.scalar_curves {
                    curve_editor_state
                        .visible_curves
                        .insert(curve.property_type);
                }
            }
        }

        curve_editor_state.view_initialized = false;
    }
}
