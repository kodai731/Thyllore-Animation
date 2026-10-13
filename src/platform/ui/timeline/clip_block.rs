use super::*;

pub(super) fn handle_clip_mute_button(
    ui: &imgui::Ui,
    world: &World,
    entity: crate::ecs::world::Entity,
    inst: &ClipInstanceSnapshot,
    inst_idx: usize,
    entry_idx: usize,
) {
    let label = if inst.muted { "M##muted" } else { "M##unmuted" };
    let button_id = format!("{}_{}_{}", label, entry_idx, inst_idx);

    ui.same_line();
    if inst.muted {
        let _color_token = ui.push_style_color(imgui::StyleColor::Button, [0.5, 0.2, 0.2, 1.0]);
        if ui.small_button(&button_id) {
            world.send_command(ClipInstanceEvent::ToggleMute {
                entity,
                instance_id: inst.instance_id,
            });
        }
    } else if ui.small_button(&button_id) {
        world.send_command(ClipInstanceEvent::ToggleMute {
            entity,
            instance_id: inst.instance_id,
        });
    }
}

pub(super) fn draw_clip_block(
    draw_list: &imgui::DrawListMut,
    block_min: [f32; 2],
    block_max: [f32; 2],
    fill_color: [f32; 4],
    border_color: [f32; 4],
    inst: &ClipInstanceSnapshot,
) {
    draw_list
        .add_rect(block_min, block_max, fill_color)
        .filled(true)
        .build();

    draw_list
        .add_rect(block_min, block_max, border_color)
        .build();

    let text_x = block_min[0] + 4.0;
    let text_y = block_min[1] + 2.0;
    let available_width = block_max[0] - block_min[0] - 8.0;

    if available_width > 10.0 {
        let mode_char = match inst.blend_mode {
            BlendMode::Override => "O",
            BlendMode::Additive => "A",
        };
        let label = format!("{} [{} {:.2}]", inst.clip_name, mode_char, inst.weight);
        let display = truncate_label_by_width(&label, available_width);
        draw_list.add_text([text_x, text_y], [1.0, 1.0, 1.0, 1.0], &display);
    }
}

pub(super) fn compute_block_color(
    base_color: [f32; 4],
    inst: &ClipInstanceSnapshot,
    interaction: &TimelineInteractionState,
    entity: crate::ecs::world::Entity,
) -> [f32; 4] {
    let alpha = if inst.muted { 0.4 } else { base_color[3] };

    let is_dragging = interaction
        .dragging_clip
        .as_ref()
        .map(|d| d.entity == entity && d.instance_id == inst.instance_id)
        .unwrap_or(false);

    let brightness = if is_dragging { 1.3 } else { 1.0 };

    [
        (base_color[0] * brightness).min(1.0),
        (base_color[1] * brightness).min(1.0),
        (base_color[2] * brightness).min(1.0),
        alpha,
    ]
}

pub(super) fn compute_border_color(
    inst: &ClipInstanceSnapshot,
    state: &TimelineState,
    entity: crate::ecs::world::Entity,
) -> [f32; 4] {
    let is_selected = state
        .selected_clip_instance
        .map(|(e, id)| e == entity && id == inst.instance_id)
        .unwrap_or(false);

    if is_selected {
        [1.0, 1.0, 0.4, 1.0]
    } else {
        [0.6, 0.6, 0.6, 0.5]
    }
}

pub(super) fn truncate_label(name: &str, max_chars: usize) -> String {
    if name.len() > max_chars {
        format!("{}...", &name[..max_chars.saturating_sub(3)])
    } else {
        name.to_string()
    }
}

pub(super) fn truncate_label_by_width(name: &str, available_width: f32) -> String {
    let approx_char_width = 7.0;
    let max_chars = (available_width / approx_char_width) as usize;
    truncate_label(name, max_chars)
}

pub(super) fn is_point_in_rect(point: [f32; 2], rect_min: [f32; 2], rect_max: [f32; 2]) -> bool {
    point[0] >= rect_min[0]
        && point[0] <= rect_max[0]
        && point[1] >= rect_min[1]
        && point[1] <= rect_max[1]
}
