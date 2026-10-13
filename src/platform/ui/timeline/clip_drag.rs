use super::*;

pub(super) fn handle_clip_drag_release(
    world: &World,
    interaction: &mut TimelineInteractionState,
    mouse_pos: [f32; 2],
    mouse_down: bool,
    pixels_per_second: f32,
) {
    if !mouse_down {
        if let Some(drag) = interaction.dragging_clip.take() {
            let delta_x = mouse_pos[0] - drag.drag_start_x;
            let delta_time = delta_x / pixels_per_second;

            match drag.drag_type {
                ClipDragType::Move => {
                    let new_start = (drag.original_value + delta_time).max(0.0);
                    world.send_command(ClipInstanceEvent::Move {
                        entity: drag.entity,
                        instance_id: drag.instance_id,
                        new_start_time: new_start,
                    });
                }
                ClipDragType::TrimStart => {
                    let new_clip_in = (drag.original_value + delta_time).max(0.0);
                    world.send_command(ClipInstanceEvent::TrimStart {
                        entity: drag.entity,
                        instance_id: drag.instance_id,
                        new_clip_in,
                    });
                }
                ClipDragType::TrimEnd => {
                    let new_clip_out = (drag.original_value + delta_time).max(0.0);
                    world.send_command(ClipInstanceEvent::TrimEnd {
                        entity: drag.entity,
                        instance_id: drag.instance_id,
                        new_clip_out,
                    });
                }
            }
        }
    }
}

pub(super) fn begin_clip_drag(
    interaction: &mut TimelineInteractionState,
    entity: crate::ecs::world::Entity,
    inst: &ClipInstanceSnapshot,
    mouse_pos: [f32; 2],
    block_min: [f32; 2],
    block_max: [f32; 2],
    _pixels_per_second: f32,
) {
    let near_left_edge = (mouse_pos[0] - block_min[0]).abs() < CLIP_EDGE_DRAG_WIDTH;
    let near_right_edge = (mouse_pos[0] - block_max[0]).abs() < CLIP_EDGE_DRAG_WIDTH;
    let block_width = block_max[0] - block_min[0];
    let block_narrower_than_edges = block_width <= CLIP_BLOCK_MIN_WIDTH;

    let (drag_type, original_value) = if block_narrower_than_edges {
        if mouse_pos[0] >= (block_min[0] + block_max[0]) * 0.5 {
            (ClipDragType::TrimEnd, inst.clip_out)
        } else {
            (ClipDragType::Move, inst.start_time)
        }
    } else if near_left_edge {
        (ClipDragType::TrimStart, inst.clip_in)
    } else if near_right_edge {
        (ClipDragType::TrimEnd, inst.clip_out)
    } else {
        (ClipDragType::Move, inst.start_time)
    };

    interaction.dragging_clip = Some(ClipDragState {
        entity,
        instance_id: inst.instance_id,
        drag_type,
        original_value,
        drag_start_x: mouse_pos[0],
    });
}

/// Where a clip block should be drawn this frame: an externally injected
/// preview (batch debug action) wins, then a live drag previews from the
/// current mouse position, otherwise the committed instance times.
pub(super) fn clip_block_display_times(
    interaction: &TimelineInteractionState,
    entity: crate::ecs::world::Entity,
    inst: &ClipInstanceSnapshot,
    mouse_pos: [f32; 2],
    mouse_down: bool,
    pixels_per_second: f32,
) -> (f32, f32) {
    if let Some(preview) = &interaction.drag_preview {
        if preview.entity == entity && preview.instance_id == inst.instance_id {
            return (preview.start_time, preview.end_time);
        }
    }

    if mouse_down {
        if let Some(drag) = &interaction.dragging_clip {
            if drag.entity == entity && drag.instance_id == inst.instance_id {
                let delta_time = (mouse_pos[0] - drag.drag_start_x) / pixels_per_second;
                return clip_drag_preview_times(
                    &drag.drag_type,
                    drag.original_value,
                    delta_time,
                    inst.start_time,
                    inst.end_time,
                    inst.clip_in,
                    inst.clip_out,
                );
            }
        }
    }

    (inst.start_time, inst.end_time)
}
