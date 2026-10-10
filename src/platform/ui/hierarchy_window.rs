use imgui::{Condition, Key};

use super::key_modifier::{current_key_modifier, KeyModifier};
use crate::asset::AssetStorage;
use crate::ecs::resource::gizmo::{BoneDisplayStyle, BoneGizmoData};
use crate::ecs::resource::{HierarchyDisplayMode, HierarchyState};
use crate::ecs::systems::phases::event_dispatch::camera::CameraEvent;
use crate::ecs::systems::phases::event_dispatch::hierarchy::HierarchyEvent;
use crate::ecs::systems::{
    query_bone_rows, query_hierarchy_tree, CameraMotion, TreeMove, TreeNavigation,
};
use crate::ecs::world::World;
use crate::platform::ui::theme::{
    entity_icon, search_field, tree_row, Icon, TreeRowResponse, TreeRowSpec,
};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

use crate::ecs::resource::LayoutSnapshot;

struct TreeKeyBinding {
    key: Key,
    modifier: KeyModifier,
    navigation: TreeNavigation,
}

const TREE_KEY_BINDINGS: &[TreeKeyBinding] = &[
    TreeKeyBinding {
        key: Key::DownArrow,
        modifier: KeyModifier::None,
        navigation: TreeNavigation::Move(TreeMove::Next),
    },
    TreeKeyBinding {
        key: Key::UpArrow,
        modifier: KeyModifier::None,
        navigation: TreeNavigation::Move(TreeMove::Prev),
    },
    TreeKeyBinding {
        key: Key::DownArrow,
        modifier: KeyModifier::Shift,
        navigation: TreeNavigation::Extend(TreeMove::Next),
    },
    TreeKeyBinding {
        key: Key::UpArrow,
        modifier: KeyModifier::Shift,
        navigation: TreeNavigation::Extend(TreeMove::Prev),
    },
    TreeKeyBinding {
        key: Key::Home,
        modifier: KeyModifier::None,
        navigation: TreeNavigation::Move(TreeMove::First),
    },
    TreeKeyBinding {
        key: Key::End,
        modifier: KeyModifier::None,
        navigation: TreeNavigation::Move(TreeMove::Last),
    },
    TreeKeyBinding {
        key: Key::PageDown,
        modifier: KeyModifier::None,
        navigation: TreeNavigation::Move(TreeMove::PageDown),
    },
    TreeKeyBinding {
        key: Key::PageUp,
        modifier: KeyModifier::None,
        navigation: TreeNavigation::Move(TreeMove::PageUp),
    },
    TreeKeyBinding {
        key: Key::RightArrow,
        modifier: KeyModifier::None,
        navigation: TreeNavigation::ExpandOrDescend,
    },
    TreeKeyBinding {
        key: Key::LeftArrow,
        modifier: KeyModifier::None,
        navigation: TreeNavigation::CollapseOrAscend,
    },
    TreeKeyBinding {
        key: Key::RightArrow,
        modifier: KeyModifier::Shift,
        navigation: TreeNavigation::ExpandRecursive,
    },
    TreeKeyBinding {
        key: Key::LeftArrow,
        modifier: KeyModifier::Shift,
        navigation: TreeNavigation::CollapseRecursive,
    },
    TreeKeyBinding {
        key: Key::KeypadMultiply,
        modifier: KeyModifier::None,
        navigation: TreeNavigation::ExpandSiblings,
    },
    TreeKeyBinding {
        key: Key::A,
        modifier: KeyModifier::Ctrl,
        navigation: TreeNavigation::SelectAllVisible,
    },
];

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

            handle_tree_keyboard(ui, world, state);
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
    if search_field(ui, "hierarchy_search", "Search...", &mut search_text) {
        world.send_command(HierarchyEvent::SetSearchFilter(search_text));
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RowClick {
    None,
    Select,
    ToggleSelect,
    ToggleExpand,
    Activate,
}

fn resolve_tree_response(response: TreeRowResponse, ui: &imgui::Ui) -> RowClick {
    match response {
        TreeRowResponse::None => RowClick::None,
        TreeRowResponse::Clicked => {
            let ctrl = current_key_modifier(ui) == KeyModifier::Ctrl;
            if ctrl {
                RowClick::ToggleSelect
            } else {
                RowClick::Select
            }
        }
        TreeRowResponse::ExpandToggled => RowClick::ToggleExpand,
        TreeRowResponse::DoubleClicked => RowClick::Activate,
    }
}

fn build_entity_tree(ui: &imgui::Ui, world: &World, state: &HierarchyState) {
    let entries = query_hierarchy_tree(world, state);

    for entry in entries {
        let spec = TreeRowSpec {
            id: &entry.entity.to_string(),
            label: &entry.name,
            icon: &entity_icon(entry.icon).map(Icon::glyph).unwrap_or_default(),
            depth: entry.depth,
            has_children: entry.has_children,
            expanded: entry.expanded,
            selected: entry.selected,
        };
        let follow_scroll = state.scroll_to_selected && state.selected_entity == Some(entry.entity);
        if follow_scroll {
            world.send_command(HierarchyEvent::ScrollToSelectionDone);
        }

        let response = tree_row(ui, world, &spec);
        if follow_scroll {
            ui.set_scroll_here_y_with_ratio(0.5);
        }

        match resolve_tree_response(response, ui) {
            RowClick::None => {}
            RowClick::Select => world.send_command(HierarchyEvent::SelectEntity(entry.entity)),
            RowClick::ToggleSelect => {
                world.send_command(HierarchyEvent::ToggleEntitySelection(entry.entity))
            }
            RowClick::ToggleExpand => {
                if entry.expanded {
                    world.send_command(HierarchyEvent::CollapseEntity(entry.entity));
                } else {
                    world.send_command(HierarchyEvent::ExpandEntity(entry.entity));
                }
            }
            RowClick::Activate => {
                world.send_command(HierarchyEvent::SelectEntity(entry.entity));
                world.send_command(CameraEvent::FrameSelection(CameraMotion::Eased));
            }
        }
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

    for bone_row in query_bone_rows(skeleton, state) {
        let spec = TreeRowSpec {
            id: &format!("bone_{}", bone_row.bone_id),
            label: &bone_row.name,
            icon: "[B]",
            depth: bone_row.depth,
            has_children: bone_row.has_children,
            expanded: bone_row.expanded,
            selected: bone_row.selected,
        };
        let follow_scroll = state.scroll_to_selected && bone_row.selected;
        if follow_scroll {
            world.send_command(HierarchyEvent::ScrollToSelectionDone);
        }

        let response = tree_row(ui, world, &spec);
        if follow_scroll {
            ui.set_scroll_here_y_with_ratio(0.5);
        }

        match resolve_tree_response(response, ui) {
            RowClick::None => {}
            RowClick::Select | RowClick::ToggleSelect => {
                world.send_command(HierarchyEvent::SelectBone(bone_row.bone_id))
            }
            RowClick::Activate => {
                world.send_command(HierarchyEvent::SelectBone(bone_row.bone_id));
                world.send_command(CameraEvent::FrameSelection(CameraMotion::Eased));
            }
            RowClick::ToggleExpand => {
                if bone_row.expanded {
                    world.send_command(HierarchyEvent::CollapseBone(bone_row.bone_id));
                } else {
                    world.send_command(HierarchyEvent::ExpandBone(bone_row.bone_id));
                }
            }
        }
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

fn handle_tree_keyboard(ui: &imgui::Ui, world: &World, state: &HierarchyState) {
    if !ui.is_window_focused() || ui.is_any_item_active() {
        return;
    }
    let modifier = current_key_modifier(ui);
    let page_rows =
        (ui.content_region_avail()[1] / ui.text_line_height_with_spacing()).max(1.0) as usize;

    for binding in TREE_KEY_BINDINGS {
        if binding.modifier == modifier && ui.is_key_pressed(binding.key) {
            world.send_command(HierarchyEvent::NavigateTree {
                navigation: binding.navigation,
                page_rows,
            });
            return;
        }
    }

    if modifier == KeyModifier::None {
        handle_tree_activation_keys(ui, world, state);
        handle_type_ahead(ui, world);
    }
}

fn handle_tree_activation_keys(ui: &imgui::Ui, world: &World, state: &HierarchyState) {
    let frame_pressed = ui.is_key_pressed(Key::F)
        || ui.is_key_pressed(Key::Enter)
        || ui.is_key_pressed(Key::KeypadEnter);
    if frame_pressed {
        world.send_command(CameraEvent::FrameSelection(CameraMotion::Eased));
    }

    let entities_selected =
        state.display_mode == HierarchyDisplayMode::Entities && state.selected_entity.is_some();
    if entities_selected && ui.is_key_pressed(Key::Delete) {
        world.send_command(HierarchyEvent::DeleteSelectedEntities);
    }
}

fn handle_type_ahead(ui: &imgui::Ui, world: &World) {
    let now_seconds = ui.time();
    for character in ui.io().input_queue_characters() {
        if character == '*' {
            world.send_command(HierarchyEvent::NavigateTree {
                navigation: TreeNavigation::ExpandSiblings,
                page_rows: 1,
            });
        } else if character.is_alphanumeric() || character == '_' {
            world.send_command(HierarchyEvent::NavigateTree {
                navigation: TreeNavigation::TypeAhead {
                    character,
                    now_seconds,
                },
                page_rows: 1,
            });
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
