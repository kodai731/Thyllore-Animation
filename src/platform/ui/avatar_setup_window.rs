use imgui::Condition;
use thyllore_avatar_core::humanoid::components::mapping_issues::MappingIssue;
use thyllore_avatar_core::humanoid::components::rest_pose::RestPose;
use thyllore_avatar_core::humanoid::components::role::{HumanoidRole, REQUIRED};
use thyllore_avatar_core::vrchat::rank::{PerformanceRank, Platform};

use crate::ecs::events::{UIEvent, UIEventQueue};
use crate::ecs::resource::{AvatarSetupState, MaterialTextureSaveState, MaterialTextureState};

const UNRESOLVED_COLOR: [f32; 4] = [1.0, 0.6, 0.2, 1.0];
const NO_BONE_LABEL: &str = "(none)";
const NO_TEXTURE_LABEL: &str = "(from model)";

pub fn build_avatar_setup_window(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    state: &mut AvatarSetupState,
    material_textures: &MaterialTextureState,
) {
    if !state.is_open {
        return;
    }

    let mut is_open = state.is_open;
    ui.window("Avatar Setup")
        .size([520.0, 600.0], Condition::FirstUseEver)
        .movable(true)
        .opened(&mut is_open)
        .build(|| {
            let Some(_tab_bar) = ui.tab_bar("##avatar_setup_tabs") else {
                return;
            };
            if let Some(_tab) = ui.tab_item("Humanoid") {
                build_humanoid_tab(ui, ui_events, state);
            }
            if let Some(_tab) = ui.tab_item("Materials") {
                build_materials_tab(ui, ui_events, material_textures);
            }
            if let Some(_tab) = ui.tab_item("Validation") {
                build_validation_tab(ui, state);
            }
            if let Some(_tab) = ui.tab_item("Stats") {
                build_stats_tab(ui, ui_events, state);
            }
            if let Some(_tab) = ui.tab_item("Export") {
                build_export_tab(ui, ui_events, state);
            }
        });
    state.is_open = is_open;
}

fn build_humanoid_tab(ui: &imgui::Ui, ui_events: &mut UIEventQueue, state: &AvatarSetupState) {
    if ui.button("Save mapping") {
        ui_events.send(UIEvent::SaveHumanoidMapping);
    }

    let Some(_table) = ui.begin_table("##humanoid_roles", 2) else {
        return;
    };
    for role in HumanoidRole::ALL {
        ui.table_next_row();
        ui.table_next_column();
        build_role_label(ui, state, role);
        ui.table_next_column();
        build_role_bone_combo(ui, ui_events, state, role);
    }
}

fn build_role_label(ui: &imgui::Ui, state: &AvatarSetupState, role: HumanoidRole) {
    let required_marker = if REQUIRED.contains(&role) { "*" } else { "" };
    let label = format!("{}{}", role.unity_name(), required_marker);
    let is_unresolved = state
        .unresolved
        .iter()
        .any(|unresolved| unresolved.role == role);
    if is_unresolved {
        ui.text_colored(UNRESOLVED_COLOR, label);
    } else {
        ui.text(label);
    }
}

fn build_role_bone_combo(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    state: &AvatarSetupState,
    role: HumanoidRole,
) {
    let current_bone = state.mapping.by_role.get(&role).copied();
    let preview = current_bone
        .and_then(|bone_index| state.bones.get(bone_index))
        .map_or(NO_BONE_LABEL, |bone| bone.name.as_str());

    ui.set_next_item_width(-1.0);
    let combo_id = format!("##bone_{}", role.unity_name());
    let Some(_combo) = ui.begin_combo(&combo_id, preview) else {
        return;
    };
    if ui
        .selectable_config(NO_BONE_LABEL)
        .selected(current_bone.is_none())
        .build()
    {
        ui_events.send(UIEvent::SetHumanoidRole { role, bone: None });
    }
    for (bone_index, bone) in state.bones.iter().enumerate() {
        let label = format!("{}##{}", bone.name, bone_index);
        if ui
            .selectable_config(&label)
            .selected(current_bone == Some(bone_index))
            .build()
        {
            ui_events.send(UIEvent::SetHumanoidRole {
                role,
                bone: Some(bone_index),
            });
        }
    }
}

fn build_materials_tab(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    material_textures: &MaterialTextureState,
) {
    if ui.button("Save and reload model") {
        ui_events.send(UIEvent::SaveMaterialTextures);
    }
    if material_textures.save_state == MaterialTextureSaveState::Edited {
        ui.same_line();
        ui.text_colored(UNRESOLVED_COLOR, "Unsaved changes");
    }

    let Some(_table) = ui.begin_table("##material_textures", 3) else {
        return;
    };
    ui.table_setup_column("Material");
    ui.table_setup_column("Base color texture");
    ui.table_setup_column("##material_texture_actions");
    ui.table_headers_row();

    for slot in &material_textures.slots {
        ui.table_next_row();
        ui.table_next_column();
        ui.text(&slot.material);

        ui.table_next_column();
        ui.text(slot.texture.as_deref().unwrap_or(NO_TEXTURE_LABEL));

        ui.table_next_column();
        if ui.button(format!("Browse##{}", slot.material)) {
            ui_events.send(UIEvent::PickMaterialTexture {
                material: slot.material.clone(),
            });
        }
        ui.same_line();
        if ui.button(format!("Clear##{}", slot.material)) {
            ui_events.send(UIEvent::ClearMaterialTexture {
                material: slot.material.clone(),
            });
        }
    }
}

fn build_validation_tab(ui: &imgui::Ui, state: &AvatarSetupState) {
    ui.text(format!("Rest pose: {}", format_rest_pose(state.rest_pose)));
    ui.separator();

    ui.text(format!("Issues: {}", state.issues.len()));
    for issue in &state.issues {
        ui.bullet_text(format_mapping_issue(issue));
    }
    ui.separator();

    ui.text(format!("Missing bones: {}", state.missing_bone_names.len()));
    for bone_name in &state.missing_bone_names {
        ui.bullet_text(bone_name);
    }
}

fn format_rest_pose(rest_pose: RestPose) -> &'static str {
    match rest_pose {
        RestPose::TPose => "T-pose",
        RestPose::APose => "A-pose",
        RestPose::Unknown => "Unknown",
    }
}

fn format_mapping_issue(issue: &MappingIssue) -> String {
    match issue {
        MappingIssue::MissingRequired(role) => {
            format!("Missing required role {}", role.unity_name())
        }
        MappingIssue::HierarchyOrder {
            child,
            expected_ancestor,
        } => format!(
            "{} is not under {}",
            child.unity_name(),
            expected_ancestor.unity_name()
        ),
    }
}

fn build_stats_tab(ui: &imgui::Ui, ui_events: &mut UIEventQueue, state: &AvatarSetupState) {
    for (label, platform) in [("PC", Platform::Pc), ("Quest", Platform::Quest)] {
        if ui.radio_button_bool(label, state.platform == platform) && state.platform != platform {
            ui_events.send(UIEvent::SetAvatarRankPlatform(platform));
        }
        ui.same_line();
    }
    ui.new_line();

    let Some(rank) = &state.rank else {
        ui.text("No rank available");
        return;
    };
    if let Some(_table) = ui.begin_table("##avatar_stats", 3) {
        ui.table_setup_column("Stat");
        ui.table_setup_column("Value");
        ui.table_setup_column("Rank");
        ui.table_headers_row();
        for item in &rank.items {
            ui.table_next_row();
            ui.table_next_column();
            ui.text(item.stat);
            ui.table_next_column();
            ui.text(item.value.to_string());
            ui.table_next_column();
            ui.text_colored(rank_color(item.rank), format!("{:?}", item.rank));
        }
    }

    ui.separator();
    ui.text("Overall:");
    ui.same_line();
    ui.text_colored(rank_color(rank.overall), format!("{:?}", rank.overall));
}

fn rank_color(rank: PerformanceRank) -> [f32; 4] {
    match rank {
        PerformanceRank::Excellent => [0.3, 0.9, 1.0, 1.0],
        PerformanceRank::Good => [0.3, 1.0, 0.3, 1.0],
        PerformanceRank::Medium => [1.0, 1.0, 0.3, 1.0],
        PerformanceRank::Poor => [1.0, 0.6, 0.2, 1.0],
        PerformanceRank::VeryPoor => [1.0, 0.3, 0.3, 1.0],
    }
}

fn build_export_tab(ui: &imgui::Ui, ui_events: &mut UIEventQueue, state: &mut AvatarSetupState) {
    if ui.button("Write sidecar JSON") {
        ui_events.send(UIEvent::ExportAvatarSidecar);
    }
    if ui.button("Write expression .anim files") {
        ui_events.send(UIEvent::ExportExpressionAnims);
    }
    if ui.button("Write current clip .anim") {
        ui_events.send(UIEvent::ExportMorphTrackAnim);
    }
    ui.separator();

    ui.input_text("Bone name prefix", &mut state.spring_prefix)
        .build();
    let prefix = state.spring_prefix.trim();
    if ui.button("Add spring chains") && !prefix.is_empty() {
        ui_events.send(UIEvent::AddSpringChainsByPrefix {
            prefix: prefix.to_string(),
        });
    }
}
