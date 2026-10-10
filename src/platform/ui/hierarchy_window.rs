use imgui::Condition;

use crate::animation::{BoneId, Skeleton};
use crate::asset::AssetStorage;
use crate::ecs::resource::gizmo::{BoneDisplayStyle, BoneGizmoData};
use crate::ecs::resource::{HierarchyDisplayMode, HierarchyState};
use crate::ecs::systems::phases::event_dispatch::hierarchy::HierarchyEvent;
use crate::ecs::systems::{hierarchy_is_bone_expanded, query_hierarchy_tree};
use crate::ecs::world::World;
use crate::platform::ui::pointer::is_last_item_double_clicked;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

use crate::ecs::resource::LayoutSnapshot;

fn draw_hierarchy_window(
    ui: &imgui::Ui,
    world: &World,
    state: &HierarchyState,
    assets: &AssetStorage,
    layout: &LayoutSnapshot,
) {
    ui.window("Hierarchy")
        .position([0.0, 0.0], Condition::Always)
        .size(
            [layout.hierarchy_width, layout.hierarchy_height],
            Condition::Always,
        )
        .resizable(false)
        .movable(false)
        .collapsible(false)
        .bring_to_front_on_focus(false)
        .build(|| {
            build_mode_tabs(ui, world, state);
            build_search_bar(ui, world, state);
            ui.separator();

            match state.display_mode {
                HierarchyDisplayMode::Entities => {
                    build_entity_tree(ui, world, state);
                }
                HierarchyDisplayMode::Bones => {
                    build_bone_tree(ui, world, state, assets);
                }
            }
        });
}

fn build_mode_tabs(ui: &imgui::Ui, world: &World, state: &HierarchyState) {
    let tab_width = 80.0;

    let entities_selected = state.display_mode == HierarchyDisplayMode::Entities;
    ui.set_next_item_width(tab_width);
    if ui
        .selectable_config("Entities")
        .selected(entities_selected)
        .size([tab_width, 0.0])
        .build()
    {
        world.send_command(HierarchyEvent::SetHierarchyDisplayMode(
            HierarchyDisplayMode::Entities,
        ));
    }

    ui.same_line();

    let bones_selected = state.display_mode == HierarchyDisplayMode::Bones;
    ui.set_next_item_width(tab_width);
    if ui
        .selectable_config("Bones")
        .selected(bones_selected)
        .size([tab_width, 0.0])
        .build()
    {
        world.send_command(HierarchyEvent::SetHierarchyDisplayMode(
            HierarchyDisplayMode::Bones,
        ));
    }

    ui.separator();
}

fn build_search_bar(ui: &imgui::Ui, world: &World, state: &HierarchyState) {
    let mut search_text = state.search_filter.clone();
    ui.set_next_item_width(-1.0);
    if ui
        .input_text("##search", &mut search_text)
        .hint("Search...")
        .build()
    {
        world.send_command(HierarchyEvent::SetSearchFilter(search_text));
    }
}

fn build_entity_tree(ui: &imgui::Ui, world: &World, state: &HierarchyState) {
    let entries = query_hierarchy_tree(world, state);

    for entry in entries {
        let indent = entry.depth as f32 * 16.0;
        let cursor_pos = ui.cursor_pos();
        ui.set_cursor_pos([cursor_pos[0] + indent, cursor_pos[1]]);

        let expand_button_width = 16.0;

        if entry.has_children {
            let expand_symbol = if entry.expanded { "v" } else { ">" };
            if ui.small_button(&format!("{}##{}", expand_symbol, entry.entity)) {
                if entry.expanded {
                    world.send_command(HierarchyEvent::CollapseEntity(entry.entity));
                } else {
                    world.send_command(HierarchyEvent::ExpandEntity(entry.entity));
                }
            }
            ui.same_line();
        } else {
            let cursor = ui.cursor_pos();
            ui.set_cursor_pos([cursor[0] + expand_button_width, cursor[1]]);
        }

        let icon_label = format!("[{}]", entry.icon_char);
        ui.text(&icon_label);
        ui.same_line();

        let label = format!("{}##{}", entry.name, entry.entity);
        let selected = entry.selected;

        if ui.selectable_config(&label).selected(selected).build() {
            if ui.io().key_ctrl {
                world.send_command(HierarchyEvent::ToggleEntitySelection(entry.entity));
            } else {
                world.send_command(HierarchyEvent::SelectEntity(entry.entity));
            }
        }

        if is_last_item_double_clicked(ui) {
            world.send_command(HierarchyEvent::FocusOnEntity(entry.entity));
        }
    }

    if ui.is_key_pressed(imgui::Key::Delete) && state.selected_entity.is_some() {
        world.send_command(HierarchyEvent::DeleteSelectedEntities);
    }
}

fn build_bone_tree(ui: &imgui::Ui, world: &World, state: &HierarchyState, assets: &AssetStorage) {
    if let Some(bone_gizmo) = world.get_resource::<BoneGizmoData>() {
        build_bone_display_panel(ui, world, &bone_gizmo);
        ui.separator();
    }

    let skeleton = match assets.skeletons.values().next() {
        Some(skel_asset) => &skel_asset.skeleton,
        None => {
            ui.text("No skeleton loaded");
            return;
        }
    };

    if skeleton.bones.is_empty() {
        ui.text("Skeleton has no bones");
        return;
    }

    ui.text(&format!(
        "Skeleton: {} ({} bones)",
        skeleton.name,
        skeleton.bones.len()
    ));
    ui.separator();

    for &root_id in &skeleton.root_bone_ids {
        build_bone_entry_recursive(ui, world, state, skeleton, root_id, 0);
    }
}

fn build_bone_display_panel(ui: &imgui::Ui, world: &World, bone_gizmo: &BoneGizmoData) {
    ui.text("Bone Display");

    let styles = [
        (BoneDisplayStyle::Stick, "Stick"),
        (BoneDisplayStyle::Octahedral, "Octa"),
        (BoneDisplayStyle::Box, "Box"),
        (BoneDisplayStyle::Sphere, "Sphere"),
    ];

    for (i, (style, label)) in styles.iter().enumerate() {
        if i > 0 {
            ui.same_line();
        }
        if ui.radio_button_bool(label, bone_gizmo.display_style == *style) {
            world.send_command(HierarchyEvent::SetBoneDisplayStyle(*style));
        }
    }

    let mut in_front = bone_gizmo.in_front;
    if ui.checkbox("In Front", &mut in_front) {
        world.send_command(HierarchyEvent::SetBoneInFront(in_front));
    }

    let mut dist_scaling = bone_gizmo.distance_scaling_enabled;
    if ui.checkbox("Distance Scaling", &mut dist_scaling) {
        world.send_command(HierarchyEvent::SetBoneDistanceScaling(dist_scaling));
    }

    if bone_gizmo.distance_scaling_enabled {
        let mut factor = bone_gizmo.distance_scaling_factor;
        ui.set_next_item_width(-1.0);
        if imgui::Slider::new(ui, "Factor", 0.01f32, 0.1f32)
            .display_format("%.3f")
            .build(&mut factor)
        {
            world.send_command(HierarchyEvent::SetBoneDistanceScaleFactor(factor));
        }
    }
}

fn build_bone_entry_recursive(
    ui: &imgui::Ui,
    world: &World,
    state: &HierarchyState,
    skeleton: &Skeleton,
    bone_id: BoneId,
    depth: usize,
) {
    let bone = match skeleton.get_bone(bone_id) {
        Some(b) => b,
        None => return,
    };

    let has_children = !bone.children.is_empty();
    let expanded = hierarchy_is_bone_expanded(state, bone_id);
    let selected = state.selected_bone_id == Some(bone_id);

    let indent = depth as f32 * 16.0;
    let cursor_pos = ui.cursor_pos();
    ui.set_cursor_pos([cursor_pos[0] + indent, cursor_pos[1]]);

    let expand_button_width = 16.0;

    if has_children {
        let expand_symbol = if expanded { "v" } else { ">" };
        if ui.small_button(&format!("{}##bone_{}", expand_symbol, bone_id)) {
            if expanded {
                world.send_command(HierarchyEvent::CollapseBone(bone_id));
            } else {
                world.send_command(HierarchyEvent::ExpandBone(bone_id));
            }
        }
        ui.same_line();
    } else {
        let cursor = ui.cursor_pos();
        ui.set_cursor_pos([cursor[0] + expand_button_width, cursor[1]]);
    }

    ui.text("[B]");
    ui.same_line();

    let label = format!("{}##bone_{}", bone.name, bone_id);
    if ui.selectable_config(&label).selected(selected).build() {
        world.send_command(HierarchyEvent::SelectBone(bone_id));
    }

    if expanded && has_children {
        let children: Vec<BoneId> = bone.children.clone();
        for child_id in children {
            build_bone_entry_recursive(ui, world, state, skeleton, child_id, depth + 1);
        }
    }
}

fn build_hierarchy_window(
    ui: &imgui::Ui,
    world: &World,
    assets: &AssetStorage,
    _: &GraphicsResources,
) {
    let hierarchy_state = world.resource::<HierarchyState>();
    let layout = world.resource::<LayoutSnapshot>();
    draw_hierarchy_window(ui, world, &hierarchy_state, assets, &layout);
}

crate::ui_window!("hierarchy", Side, 1, build_hierarchy_window);
