use crate::asset::AssetStorage;
use crate::ecs::events::{send_dialog_request, DialogRequest};
use crate::ecs::resource::{
    AvatarSetupState, FbxModelCache, MaterialTextureSaveState, MaterialTextureState,
};
use crate::ecs::systems::phases::event_dispatch::avatar_setup::AvatarSetupEvent;
use crate::ecs::world::World;
use crate::platform::ui::param_widgets::draw_params;
use crate::platform::ui::theme::{draw_raised_window_frame, push_raised_window_style};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;
use imgui::Condition;
use thyllore_avatar_core::garment::components::chain_gen::{
    ChainGenRequest, CHAIN_GEN_SCALAR_PARAMS, CHAIN_GEN_UI_PARAMS,
};
use thyllore_avatar_core::garment::components::submesh_role::{
    SubmeshKey, SubmeshRole, SubmeshRoleSource, SubmeshSelector,
};
use thyllore_avatar_core::humanoid::components::geometry_warning::GeometryWarning;
use thyllore_avatar_core::humanoid::components::rest_pose::RestPose;
use thyllore_avatar_core::humanoid::components::role::{HumanoidRole, REQUIRED};
use thyllore_avatar_core::vrchat::rank::{PerformanceRank, Platform};

const UNRESOLVED_COLOR: [f32; 4] = [1.0, 0.6, 0.2, 1.0];
const FAILURE_COLOR: [f32; 4] = [1.0, 0.3, 0.3, 1.0];
const NO_BONE_LABEL: &str = "(none)";
const NO_TEXTURE_LABEL: &str = "(from model)";
const SUBMESH_ROLE_CHOICES: [SubmeshRole; 4] = [
    SubmeshRole::Skin,
    SubmeshRole::Garment,
    SubmeshRole::Hair,
    SubmeshRole::Accessory,
];

fn draw_avatar_setup_window(
    ui: &imgui::Ui,
    world: &World,
    state: &mut AvatarSetupState,
    material_textures: &MaterialTextureState,
) {
    if !state.is_open {
        return;
    }

    let mut is_open = state.is_open;
    let _raised = push_raised_window_style(ui);
    ui.window("Avatar Setup")
        .size([520.0, 600.0], Condition::FirstUseEver)
        .movable(true)
        .opened(&mut is_open)
        .build(|| {
            draw_raised_window_frame(ui);
            let Some(_tab_bar) = ui.tab_bar("##avatar_setup_tabs") else {
                return;
            };
            if let Some(_tab) = ui.tab_item("Humanoid") {
                build_humanoid_tab(ui, world, state);
            }
            if let Some(_tab) = ui.tab_item("Materials") {
                build_materials_tab(ui, world, material_textures);
            }
            if let Some(_tab) = ui.tab_item("Submeshes") {
                build_submeshes_tab(ui, world, state);
            }
            if let Some(_tab) = ui.tab_item("Validation") {
                build_validation_tab(ui, state);
            }
            if let Some(_tab) = ui.tab_item("Stats") {
                build_stats_tab(ui, world, state);
            }
            if let Some(_tab) = ui.tab_item("Export") {
                build_export_tab(ui, world, state);
            }
        });
    state.is_open = is_open;
}

fn build_humanoid_tab(ui: &imgui::Ui, world: &World, state: &AvatarSetupState) {
    if ui.button("Save mapping") {
        world.send_command(AvatarSetupEvent::SaveHumanoidMapping);
    }

    let Some(_table) = ui.begin_table("##humanoid_roles", 2) else {
        return;
    };
    for role in HumanoidRole::ALL {
        ui.table_next_row();
        ui.table_next_column();
        build_role_label(ui, state, role);
        ui.table_next_column();
        build_role_bone_combo(ui, world, state, role);
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
    world: &World,
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
        world.send_command(AvatarSetupEvent::SetHumanoidRole { role, bone: None });
    }
    for (bone_index, bone) in state.bones.iter().enumerate() {
        let label = format!("{}##{}", bone.name, bone_index);
        if ui
            .selectable_config(&label)
            .selected(current_bone == Some(bone_index))
            .build()
        {
            world.send_command(AvatarSetupEvent::SetHumanoidRole {
                role,
                bone: Some(bone_index),
            });
        }
    }
}

fn build_materials_tab(ui: &imgui::Ui, world: &World, material_textures: &MaterialTextureState) {
    if ui.button("Save and reload model") {
        world.send_command(AvatarSetupEvent::SaveMaterialTextures);
    }
    match &material_textures.save_state {
        MaterialTextureSaveState::Saved => {}
        MaterialTextureSaveState::Edited => {
            ui.same_line();
            ui.text_colored(UNRESOLVED_COLOR, "Unsaved changes");
        }
        MaterialTextureSaveState::SaveFailed { reason } => {
            let _wrap = ui.push_text_wrap_pos(0.0);
            ui.text_colored(FAILURE_COLOR, reason);
        }
    }

    let Some(_table) = ui.begin_table("##material_textures", 3) else {
        return;
    };
    ui.table_setup_column("Material", imgui::TableColumnFlags::empty(), None);
    ui.table_setup_column("Base color texture", imgui::TableColumnFlags::empty(), None);
    ui.table_setup_column(
        "##material_texture_actions",
        imgui::TableColumnFlags::empty(),
        None,
    );
    ui.table_headers_row();

    for slot in &material_textures.slots {
        ui.table_next_row();
        ui.table_next_column();
        ui.text(&slot.material);

        ui.table_next_column();
        ui.text(slot.texture.as_deref().unwrap_or(NO_TEXTURE_LABEL));

        ui.table_next_column();
        if ui.button(format!("Browse##{}", slot.material)) {
            world.send_command(AvatarSetupEvent::PickMaterialTexture {
                material: slot.material.clone(),
            });
        }
        ui.same_line();
        if ui.button(format!("Clear##{}", slot.material)) {
            world.send_command(AvatarSetupEvent::ClearMaterialTexture {
                material: slot.material.clone(),
            });
        }
    }
}

fn submesh_role_label(role: SubmeshRole) -> &'static str {
    match role {
        SubmeshRole::Skin => "Skin",
        SubmeshRole::Garment => "Garment",
        SubmeshRole::Hair => "Hair",
        SubmeshRole::Accessory => "Accessory",
    }
}

fn draw_chain_gen_settings(ui: &imgui::Ui, world: &World, state: &AvatarSetupState) {
    let names: Vec<&str> = CHAIN_GEN_UI_PARAMS.iter().map(|p| p.name).collect();
    let mut settings = state.chain_gen.clone();
    draw_params(
        ui,
        &names,
        &CHAIN_GEN_UI_PARAMS,
        &CHAIN_GEN_SCALAR_PARAMS,
        &mut settings,
        |_, _| {},
    );
    if settings != state.chain_gen {
        world.send_command(AvatarSetupEvent::SetChainGenSettings { settings });
    }
}

fn build_submeshes_tab(ui: &imgui::Ui, world: &World, state: &AvatarSetupState) {
    if state.submesh_roles.is_empty() {
        ui.text_disabled("No submeshes");
        return;
    }

    ui.text("Chain generation");
    draw_chain_gen_settings(ui, world, state);
    ui.separator();

    for (index, (key, entry)) in state.submesh_roles.iter().enumerate() {
        let label = format!("{}/{}", key.mesh, key.material);
        let current_role = entry.role;
        let preview = submesh_role_label(current_role);

        ui.text(&label);
        if entry.source == SubmeshRoleSource::Guessed {
            ui.same_line();
            ui.text_disabled("(guessed)");
        }
        ui.same_line();
        ui.set_next_item_width(-1.0);
        let combo_id = format!("##submesh_role_{}", index);
        if let Some(_combo) = ui.begin_combo(&combo_id, preview) {
            for role in SUBMESH_ROLE_CHOICES {
                if ui
                    .selectable_config(submesh_role_label(role))
                    .selected(current_role == role)
                    .build()
                {
                    world.send_command(AvatarSetupEvent::SetSubmeshRole {
                        key: key.clone(),
                        role,
                    });
                }
            }
        }

        if current_role == SubmeshRole::Hair || current_role == SubmeshRole::Garment {
            let humanoid_role = match current_role {
                SubmeshRole::Hair => HumanoidRole::Head,
                SubmeshRole::Garment => HumanoidRole::Hips,
                _ => unreachable!(),
            };
            if let Some(&bone_index) = state.mapping.by_role.get(&humanoid_role) {
                let root_bone = &state.bones[bone_index as usize].name;
                ui.same_line();
                if ui.button("Generate chains##chain") {
                    world.send_command(AvatarSetupEvent::GenerateChainBones {
                        request: ChainGenRequest {
                            target: SubmeshSelector::Submesh(key.clone()),
                            root_bone: root_bone.clone(),
                            settings: state.chain_gen.clone(),
                        },
                    });
                }
            } else {
                ui.same_line();
                ui.text_disabled("Generate chains");
            }
        }
    }
}

fn build_validation_tab(ui: &imgui::Ui, state: &AvatarSetupState) {
    ui.text(format!("Rest pose: {}", format_rest_pose(state.rest_pose)));
    ui.separator();

    ui.text(format!("Issues: {}", state.issues.len()));
    for issue in &state.issues {
        ui.bullet_text(issue.to_string());
    }
    ui.separator();

    ui.text(format!("Warnings: {}", state.geometry_warnings.len()));
    for warning in &state.geometry_warnings {
        ui.bullet_text(format_geometry_warning(warning));
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

fn format_geometry_warning(warning: &GeometryWarning) -> String {
    match warning {
        GeometryWarning::Asymmetric {
            left,
            right,
            distance,
        } => format!(
            "{} and {} are asymmetric (distance {:.3})",
            left.unity_name(),
            right.unity_name(),
            distance
        ),
        GeometryWarning::NotAscending { lower, upper } => {
            format!("{} is not below {}", lower.unity_name(), upper.unity_name())
        }
        GeometryWarning::LengthRatio {
            upper,
            lower,
            ratio,
        } => format!(
            "{} / {} length ratio {:.2} is out of range",
            upper.unity_name(),
            lower.unity_name(),
            ratio
        ),
    }
}

fn build_stats_tab(ui: &imgui::Ui, world: &World, state: &AvatarSetupState) {
    for (label, platform) in [("PC", Platform::Pc), ("Quest", Platform::Quest)] {
        if ui.radio_button_bool(label, state.platform == platform) && state.platform != platform {
            world.send_command(AvatarSetupEvent::SetAvatarRankPlatform(platform));
        }
        ui.same_line();
    }
    ui.new_line();

    let Some(rank) = &state.rank else {
        ui.text("No rank available");
        return;
    };
    if let Some(_table) = ui.begin_table("##avatar_stats", 3) {
        ui.table_setup_column("Stat", imgui::TableColumnFlags::empty(), None);
        ui.table_setup_column("Value", imgui::TableColumnFlags::empty(), None);
        ui.table_setup_column("Rank", imgui::TableColumnFlags::empty(), None);
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

fn build_export_tab(ui: &imgui::Ui, world: &World, state: &mut AvatarSetupState) {
    if ui.button("Export for Unity") {
        world.send_command(AvatarSetupEvent::ExportUnityAvatar);
    }
    ui.text_disabled("sidecar JSON, expression .anim files, current clip .anim");

    let has_fbx_model = world
        .get_resource::<FbxModelCache>()
        .map_or(false, |cache| cache.fbx_model().is_some());
    if has_fbx_model {
        if ui.button("Export FBX") {
            send_dialog_request(world, DialogRequest::ExportModelFbx);
        }
        ui.text_disabled("mesh, skeleton and skin weights as loaded and edited, no clip");
    } else {
        ui.text_disabled("Export FBX (needs an FBX model)");
    }
    ui.separator();

    ui.input_text("Bone name prefix", &mut state.spring_prefix)
        .build();
    let prefix = state.spring_prefix.trim();
    if ui.button("Add spring chains") && !prefix.is_empty() {
        world.send_command(AvatarSetupEvent::AddSpringChainsByPrefix {
            prefix: prefix.to_string(),
        });
    }
}

fn build_avatar_setup_window(
    ui: &imgui::Ui,
    world: &World,
    _: &AssetStorage,
    _: &GraphicsResources,
) {
    let mut avatar_setup = world.resource_mut::<AvatarSetupState>();
    let material_textures = world.resource::<MaterialTextureState>();
    draw_avatar_setup_window(ui, world, &mut avatar_setup, &material_textures);
}

crate::ui_window!("avatar_setup", Floating, 1, build_avatar_setup_window);
