use super::clip_block::{
    compute_block_color, compute_border_color, draw_clip_block, handle_clip_mute_button,
    is_point_in_rect, truncate_label,
};
use super::clip_drag::{begin_clip_drag, clip_block_display_times, handle_clip_drag_release};
use super::clip_properties::build_clip_instance_properties;
use super::layout::{
    CLIP_BLOCK_COLORS, CLIP_BLOCK_MIN_WIDTH, CLIP_TRACK_HEIGHT, PIXELS_PER_SECOND,
    TRACK_LABEL_WIDTH,
};
use crate::animation::editable::SourceClipId;
use crate::animation::BoneId;
use crate::ecs::component::{
    ClipGroupSnapshot, ClipInstanceSnapshot, ClipSchedule, ClipTrackEntry, ClipTrackSnapshot,
};
use crate::ecs::resource::{
    ClipLibrary, ClipPreview, CurveEditorState, TimelineInteractionState, TimelineState,
    UiPointerOwnerId,
};
use crate::ecs::systems::phases::event_dispatch::clip_instance::ClipInstanceEvent;
use crate::ecs::systems::phases::event_dispatch::hierarchy::HierarchyEvent;
use crate::ecs::systems::phases::event_dispatch::timeline::TimelineEvent;
use crate::ecs::systems::{clip_schedule_assign_lanes, find_preview_owner};
use crate::ecs::world::{Entity, World};
use crate::platform::ui::numeric_format::numeric_format;
use crate::platform::ui::pointer::{read_ui_pointer, ui_pointer_begin, PointerRegion};
use imgui::MouseButton;

struct ClipTrackPointer {
    pos: [f32; 2],
    down: bool,
    clicked: bool,
    double_clicked: bool,
}

pub(super) struct ClipTrackContext<'a> {
    pub(super) ui: &'a imgui::Ui,
    pub(super) world: &'a World,
    pub(super) state: &'a mut TimelineState,
    pub(super) interaction: &'a mut TimelineInteractionState,
    pub(super) clip_library: &'a ClipLibrary,
    pub(super) curve_editor_state: &'a mut CurveEditorState,
}

struct ClipTrackFrame {
    pointer: ClipTrackPointer,
    pixels_per_second: f32,
    timeline_width: f32,
    solo_preview: Option<(Entity, SourceClipId)>,
}

struct ClipTrackRow<'a> {
    frame: &'a ClipTrackFrame,
    entry_idx: usize,
    entry: &'a ClipTrackEntry,
    track_origin: [f32; 2],
    solo_clip: Option<SourceClipId>,
}

struct ClipInstanceSlot<'a> {
    index: usize,
    instance: &'a ClipInstanceSnapshot,
    lane: usize,
}

pub(super) fn build_clip_tracks_section(
    context: &mut ClipTrackContext,
    snapshot: &ClipTrackSnapshot,
    timeline_width: f32,
) {
    let ui = context.ui;
    let world = context.world;

    let pixels_per_second = PIXELS_PER_SECOND * context.state.zoom_level;
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
        context.interaction,
        track_pointer.pos,
        track_pointer.down,
        pixels_per_second,
    );

    let solo_preview: Option<(Entity, SourceClipId)> = (context.state.preview == ClipPreview::Solo
        && context.state.current_clip_id.is_some())
    .then(|| find_preview_owner(world))
    .flatten()
    .zip(context.state.current_clip_id);
    let frame = ClipTrackFrame {
        pointer: track_pointer,
        pixels_per_second,
        timeline_width,
        solo_preview,
    };

    let section_start_y = ui.cursor_screen_pos()[1];
    let mut clicked_any_block = false;

    for (entry_idx, entry) in snapshot.entries.iter().enumerate() {
        clicked_any_block |= build_clip_track_row(context, &frame, entry_idx, entry);
    }

    let clicked_empty_track =
        frame.pointer.clicked && !clicked_any_block && frame.pointer.pos[1] >= section_start_y;
    if clicked_empty_track {
        world.send_command(ClipInstanceEvent::Deselect);
    }
}

fn build_clip_track_row(
    context: &mut ClipTrackContext,
    frame: &ClipTrackFrame,
    entry_idx: usize,
    entry: &ClipTrackEntry,
) -> bool {
    let ui = context.ui;
    let world = context.world;

    build_group_headers(ui, world, entry);

    let cursor_pos = ui.cursor_screen_pos();
    ui.text(&truncate_label(&entry.entity_name, 15));
    ui.same_line_with_pos(TRACK_LABEL_WIDTH);

    let track_origin = [cursor_pos[0] + TRACK_LABEL_WIDTH, cursor_pos[1]];
    let draw_list = ui.get_window_draw_list();

    let row = ClipTrackRow {
        frame,
        entry_idx,
        entry,
        track_origin,
        solo_clip: frame
            .solo_preview
            .filter(|(owner, _)| *owner == entry.entity)
            .map(|(_, clip_id)| clip_id),
    };

    let lanes = world
        .get_component::<ClipSchedule>(entry.entity)
        .map(|schedule| clip_schedule_assign_lanes(&schedule.instances))
        .unwrap_or_else(|| vec![0; entry.instances.len()]);
    let instance_lane_count = lanes.iter().max().map_or(1, |max_lane| max_lane + 1);
    let lane_count = instance_lane_count + usize::from(row.solo_clip.is_some());

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

    for (index, instance) in entry.instances.iter().enumerate() {
        let slot = ClipInstanceSlot {
            index,
            instance,
            lane: lanes[index],
        };
        clicked_any_block |= build_clip_instance_block(context, &draw_list, &row, &slot);
    }

    draw_solo_preview_block(&draw_list, context.clip_library, &row, instance_lane_count);

    ui.dummy([frame.timeline_width, track_height]);

    accept_clip_source_drop(ui, world, &row);

    build_clip_instance_properties(ui, world, context.state, context.clip_library, entry);

    clicked_any_block
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
    context: &mut ClipTrackContext,
    draw_list: &imgui::DrawListMut,
    row: &ClipTrackRow,
    slot: &ClipInstanceSlot,
) -> bool {
    let world = context.world;
    let frame = row.frame;
    let inst = slot.instance;
    let entity = row.entry.entity;
    let pixels_per_second = frame.pixels_per_second;

    let (draw_start, draw_end) = clip_block_display_times(
        context.interaction,
        entity,
        inst,
        frame.pointer.pos,
        frame.pointer.down,
        pixels_per_second,
    );
    let block_w = ((draw_end - draw_start) * pixels_per_second).max(CLIP_BLOCK_MIN_WIDTH);
    let (block_min, block_max) = compute_lane_block_rect(
        row.track_origin,
        slot.lane,
        draw_start * pixels_per_second,
        block_w,
    );

    let base_color = CLIP_BLOCK_COLORS[row.entry_idx % CLIP_BLOCK_COLORS.len()];
    let mut color = compute_block_color(base_color, inst, context.interaction, entity);

    if row.solo_clip.is_some() {
        color[3] *= 0.4;
    }

    let border_color = compute_border_color(inst, context.state, entity);

    draw_clip_block(draw_list, block_min, block_max, color, border_color, inst);

    let hit_block = is_point_in_rect(frame.pointer.pos, block_min, block_max);
    let clicked_block = hit_block && frame.pointer.clicked;

    if hit_block && frame.pointer.double_clicked {
        world.send_command(HierarchyEvent::SelectEntity(entity));
        world.send_command(TimelineEvent::SelectClip(inst.source_id));
        open_curve_editor_for_clip(
            context.curve_editor_state,
            context.clip_library,
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
            context.interaction,
            entity,
            inst,
            frame.pointer.pos,
            block_min,
            block_max,
            pixels_per_second,
        );
    }

    handle_clip_mute_button(context.ui, world, entity, inst, slot.index, row.entry_idx);

    clicked_block
}

fn draw_solo_preview_block(
    draw_list: &imgui::DrawListMut,
    clip_library: &ClipLibrary,
    row: &ClipTrackRow,
    solo_lane: usize,
) {
    let Some(clip) = row.solo_clip.and_then(|clip_id| clip_library.get(clip_id)) else {
        return;
    };

    let block_w = (clip.duration * row.frame.pixels_per_second).max(CLIP_BLOCK_MIN_WIDTH);
    let (block_min, block_max) = compute_lane_block_rect(row.track_origin, solo_lane, 0.0, block_w);

    draw_list
        .add_rect(block_min, block_max, [1.0, 1.0, 1.0, 1.0])
        .filled(false)
        .build();

    let label_pos = [block_min[0] + 4.0, block_min[1] + 2.0];
    draw_list.add_text(label_pos, [1.0, 1.0, 1.0, 1.0], &clip.name);
}

fn accept_clip_source_drop(ui: &imgui::Ui, world: &World, row: &ClipTrackRow) {
    let Some(target) = ui.drag_drop_target() else {
        return;
    };
    let accepted = target
        .accept_payload::<SourceClipId, _>("CLIP_SOURCE", imgui::DragDropTargetFlags::empty());

    if let Some(Ok(payload)) = accepted {
        let drop_x = row.frame.pointer.pos[0] - row.track_origin[0];
        let start_time = (drop_x / row.frame.pixels_per_second).max(0.0);
        world.send_command(ClipInstanceEvent::Add {
            entity: row.entry.entity,
            source_id: payload.data,
            start_time,
        });
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
    entity: Entity,
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
