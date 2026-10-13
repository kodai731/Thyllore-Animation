use super::*;

pub(super) fn build_clip_instance_properties(
    ui: &imgui::Ui,
    world: &World,
    state: &TimelineState,
    clip_library: &ClipLibrary,
    entry: &ClipTrackEntry,
) {
    let Some((sel_entity, sel_id)) = state.selected_clip_instance else {
        return;
    };

    if sel_entity != entry.entity {
        return;
    }

    let Some(inst) = entry.instances.iter().find(|i| i.instance_id == sel_id) else {
        return;
    };

    ui.text("  Properties:");
    ui.same_line();

    build_clip_length_field(ui, world, clip_library, inst.source_id);
    ui.same_line();

    ui.set_next_item_width(60.0);
    let mut weight = inst.weight;
    if imgui::Drag::new("##inst_weight")
        .range(0.0, 1.0)
        .speed(0.01)
        .display_format(numeric_format("W:%.2f"))
        .build(ui, &mut weight)
    {
        world.send_command(ClipInstanceEvent::SetWeight {
            entity: entry.entity,
            instance_id: inst.instance_id,
            weight,
        });
    }

    ui.same_line();
    let blend_names = ["Override", "Additive"];
    let current_idx = match inst.blend_mode {
        BlendMode::Override => 0,
        BlendMode::Additive => 1,
    };

    ui.set_next_item_width(80.0);
    if let Some(_token) = ui.begin_combo("##blend_mode", blend_names[current_idx]) {
        for (idx, &name) in blend_names.iter().enumerate() {
            let is_selected = idx == current_idx;
            if ui.selectable_config(name).selected(is_selected).build() {
                let new_mode = match idx {
                    0 => BlendMode::Override,
                    1 => BlendMode::Additive,
                    _ => continue,
                };
                world.send_command(ClipInstanceEvent::SetBlendMode {
                    entity: entry.entity,
                    instance_id: inst.instance_id,
                    blend_mode: new_mode,
                });
            }
        }
    }

    if !entry.groups.is_empty() {
        ui.same_line();
        let current_group_name = inst
            .group_id
            .and_then(|gid| entry.groups.iter().find(|g| g.id == gid))
            .map(|g| g.name.as_str())
            .unwrap_or("No Group");

        ui.set_next_item_width(100.0);
        if let Some(_token) = ui.begin_combo("##inst_group", current_group_name) {
            if ui
                .selectable_config("No Group")
                .selected(inst.group_id.is_none())
                .build()
            {
                if let Some(gid) = inst.group_id {
                    world.send_command(ClipInstanceEvent::GroupRemoveInstance {
                        entity: entry.entity,
                        group_id: gid,
                        instance_id: inst.instance_id,
                    });
                }
            }

            for group in &entry.groups {
                let is_selected = inst.group_id.map(|gid| gid == group.id).unwrap_or(false);
                if ui
                    .selectable_config(&group.name)
                    .selected(is_selected)
                    .build()
                {
                    world.send_command(ClipInstanceEvent::GroupAddInstance {
                        entity: entry.entity,
                        group_id: group.id,
                        instance_id: inst.instance_id,
                    });
                }
            }
        }
    }
}

/// Clip length in seconds, editable even when the clip has no keyframes: the
/// authored floor `min_duration` is what makes an unkeyed clip loop longer.
pub(super) fn build_clip_length_field(
    ui: &imgui::Ui,
    world: &World,
    clip_library: &ClipLibrary,
    source_id: SourceClipId,
) {
    let Some(clip) = clip_library.get(source_id) else {
        return;
    };
    ui.set_next_item_width(80.0);
    let mut seconds = clip.duration;
    if imgui::Drag::new("##clip_length")
        .range(0.0, 3600.0)
        .speed(0.05)
        .display_format(numeric_format("Len:%.2fs"))
        .build(ui, &mut seconds)
    {
        world.send_command(ScalarCurveEvent::ClipSetMinDuration { source_id, seconds });
    }
    if ui.is_item_hovered() {
        ui.tooltip_text(
            "Clip length: keyframes extend it, this value keeps it at least this long \
             (drag to lengthen the loop of a clip without keys)",
        );
    }
}
