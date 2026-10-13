use super::*;

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
    let mouse_pos = pointer.pos;
    let mouse_down = pointer.is_down(MouseButton::Left);
    let mouse_clicked = pointer.is_clicked(MouseButton::Left)
        && ui_pointer_begin(ui, world, UiPointerOwnerId::Timeline, PointerRegion::Window);
    let mouse_double_clicked = mouse_clicked && pointer.is_double_clicked(MouseButton::Left);

    handle_clip_drag_release(world, interaction, mouse_pos, mouse_down, pixels_per_second);

    let mut clicked_any_block = false;
    let solo_preview: Option<(crate::ecs::world::Entity, SourceClipId)> =
        (state.preview == ClipPreview::Solo && state.current_clip_id.is_some())
            .then(|| find_preview_owner(world))
            .flatten()
            .zip(state.current_clip_id);
    let mut total_track_rows: f32 = 0.0;

    for (entry_idx, entry) in snapshot.entries.iter().enumerate() {
        build_group_headers(ui, world, entry);

        let cursor_pos = ui.cursor_screen_pos();
        ui.text(&truncate_label(&entry.entity_name, 15));
        ui.same_line_with_pos(TRACK_LABEL_WIDTH);

        let track_origin = [cursor_pos[0] + TRACK_LABEL_WIDTH, cursor_pos[1]];
        let draw_list = ui.get_window_draw_list();

        let solo_clip_for_row = solo_preview
            .filter(|(owner, _)| *owner == entry.entity)
            .map(|(_, clip_id)| clip_id);
        let is_solo_owner = solo_clip_for_row.is_some();

        let lanes = world
            .get_component::<ClipSchedule>(entry.entity)
            .map(|schedule| clip_schedule_assign_lanes(&schedule.instances))
            .unwrap_or_else(|| vec![0; entry.instances.len()]);
        let mut lane_count = lanes.iter().max().map_or(1, |max_lane| max_lane + 1);

        if is_solo_owner {
            lane_count += 1;
        }

        let track_height = (lane_count as f32) * CLIP_TRACK_HEIGHT;

        draw_list
            .add_rect(
                track_origin,
                [
                    track_origin[0] + timeline_width,
                    track_origin[1] + track_height,
                ],
                [0.15, 0.15, 0.2, 1.0],
            )
            .filled(true)
            .build();

        for (inst_idx, inst) in entry.instances.iter().enumerate() {
            let lane = lanes[inst_idx];
            let lane_y = track_origin[1] + (lane as f32) * CLIP_TRACK_HEIGHT;

            let (draw_start, draw_end) = clip_block_display_times(
                interaction,
                entry.entity,
                inst,
                mouse_pos,
                mouse_down,
                pixels_per_second,
            );
            let block_x = track_origin[0] + draw_start * pixels_per_second;
            let block_w = ((draw_end - draw_start) * pixels_per_second).max(CLIP_BLOCK_MIN_WIDTH);
            let block_min = [block_x, lane_y + 2.0];
            let block_max = [block_x + block_w, lane_y + CLIP_TRACK_HEIGHT - 2.0];

            let base_color = CLIP_BLOCK_COLORS[entry_idx % CLIP_BLOCK_COLORS.len()];
            let mut color = compute_block_color(base_color, inst, interaction, entry.entity);

            if is_solo_owner {
                color[3] *= 0.4;
            }

            let border_color = compute_border_color(inst, state, entry.entity);

            draw_clip_block(&draw_list, block_min, block_max, color, border_color, inst);

            let hit_block = is_point_in_rect(mouse_pos, block_min, block_max);

            if mouse_double_clicked && hit_block {
                clicked_any_block = true;
                world.send_command(HierarchyEvent::SelectEntity(entry.entity));
                world.send_command(TimelineEvent::SelectClip(inst.source_id));
                open_curve_editor_for_clip(
                    curve_editor_state,
                    clip_library,
                    inst.source_id,
                    entry.mesh_bone_id,
                );
            } else if mouse_clicked && hit_block {
                clicked_any_block = true;
                world.send_command(HierarchyEvent::SelectEntity(entry.entity));
                world.send_command(ClipInstanceEvent::Select {
                    entity: entry.entity,
                    instance_id: inst.instance_id,
                });
                begin_clip_drag(
                    interaction,
                    entry.entity,
                    inst,
                    mouse_pos,
                    block_min,
                    block_max,
                    pixels_per_second,
                );
            }

            handle_clip_mute_button(ui, world, entry.entity, inst, inst_idx, entry_idx);
        }

        if let Some(solo_source_id) = solo_clip_for_row {
            let solo_lane_y = track_origin[1] + (lane_count as f32 - 1.0) * CLIP_TRACK_HEIGHT;

            if let Some(clip) = clip_library.get(solo_source_id) {
                let clip_duration = clip.duration;
                let block_x = track_origin[0];
                let block_w = (clip_duration * pixels_per_second).max(CLIP_BLOCK_MIN_WIDTH);
                let block_min = [block_x, solo_lane_y + 2.0];
                let block_max = [block_x + block_w, solo_lane_y + CLIP_TRACK_HEIGHT - 2.0];

                draw_list
                    .add_rect(block_min, block_max, [1.0, 1.0, 1.0, 1.0])
                    .filled(false)
                    .build();

                let label_pos = [block_min[0] + 4.0, block_min[1] + 2.0];
                draw_list.add_text(label_pos, [1.0, 1.0, 1.0, 1.0], &clip.name);
            }
        }

        ui.dummy([timeline_width, track_height]);
        total_track_rows += track_height;

        if let Some(target) = ui.drag_drop_target() {
            let accepted = target.accept_payload::<SourceClipId, _>(
                "CLIP_SOURCE",
                imgui::DragDropTargetFlags::empty(),
            );
            if let Some(Ok(payload)) = accepted {
                let source_id = payload.data;
                let drop_x = mouse_pos[0] - track_origin[0];
                let start_time = (drop_x / pixels_per_second).max(0.0);
                world.send_command(ClipInstanceEvent::Add {
                    entity: entry.entity,
                    source_id,
                    start_time,
                });
            }
        }

        build_clip_instance_properties(ui, world, state, clip_library, entry);
    }

    if mouse_clicked && !clicked_any_block {
        let section_start_y = ui.cursor_screen_pos()[1]
            - (total_track_rows
                + snapshot.entries.len() as f32 * ui.text_line_height_with_spacing());

        if mouse_pos[1] >= section_start_y {
            world.send_command(ClipInstanceEvent::Deselect);
        }
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
