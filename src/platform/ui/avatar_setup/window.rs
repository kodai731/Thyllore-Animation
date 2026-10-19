use crate::ecs::resource::{AvatarSetupState, MaterialTextureState};
use crate::ecs::systems::phases::event_dispatch::avatar_setup::AvatarSetupEvent;
use crate::ecs::world::World;
use crate::platform::ui::theme::{draw_raised_window_frame, push_raised_window_style};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;
use imgui::Condition;
use thyllore_avatar_core::humanoid::components::geometry_warning::GeometryWarning;
use thyllore_avatar_core::humanoid::components::rest_pose::RestPose;

use super::export_rows::ExportView;
use super::humanoid_rows::HumanoidView;
use super::material_rows::MaterialView;
use super::stats_rows::StatsView;
use super::submesh_rows::SubmeshesView;

fn draw_avatar_setup_window(ui: &imgui::Ui, world: &World, state: &mut AvatarSetupState) {
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
                let mut view = HumanoidView::from_state(state);
                crate::platform::ui::draw_rows(
                    ui,
                    world,
                    &super::humanoid_rows::rows(),
                    &mut view,
                    |_, _| {},
                );
            }
            if let Some(_tab) = ui.tab_item("Materials") {
                let material_textures = world.resource::<MaterialTextureState>();
                let mut view = MaterialView::from_state(&material_textures);
                crate::platform::ui::draw_rows(
                    ui,
                    world,
                    &super::material_rows::rows(),
                    &mut view,
                    |_, _| {},
                );
            }
            if let Some(_tab) = ui.tab_item("Submeshes") {
                let mut view = SubmeshesView::from_state(state);
                crate::platform::ui::draw_rows(
                    ui,
                    world,
                    &super::submesh_rows::rows(),
                    &mut view,
                    |_, _| {},
                );
                if view.chain_gen != state.chain_gen {
                    world.send_command(AvatarSetupEvent::SetChainGenSettings {
                        settings: view.chain_gen,
                    });
                }
            }
            if let Some(_tab) = ui.tab_item("Validation") {
                build_validation_tab(ui, state);
            }
            if let Some(_tab) = ui.tab_item("Stats") {
                let mut view = StatsView::from_state(state);
                crate::platform::ui::draw_rows(
                    ui,
                    world,
                    &super::stats_rows::rows(),
                    &mut view,
                    |_, _| {},
                );
                if view.platform != state.platform {
                    world.send_command(AvatarSetupEvent::SetAvatarRankPlatform(view.platform));
                }
            }
            if let Some(_tab) = ui.tab_item("Export") {
                let mut view = ExportView::from_state(state);
                crate::platform::ui::draw_rows(
                    ui,
                    world,
                    &super::export_rows::rows(),
                    &mut view,
                    |_, _| {},
                );
                state.spring_prefix = view.spring_prefix;
            }
        });
    state.is_open = is_open;
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

fn build_avatar_setup_window(
    ui: &imgui::Ui,
    world: &World,
    _assets: &crate::asset::AssetStorage,
    _graphics: &GraphicsResources,
) {
    let mut avatar_setup = world.resource::<AvatarSetupState>().clone();
    draw_avatar_setup_window(ui, world, &mut avatar_setup);
    write_back_avatar_setup_state(world, avatar_setup);
}

fn write_back_avatar_setup_state(world: &World, src: AvatarSetupState) {
    if let Some(mut state) = world.get_resource_mut::<AvatarSetupState>() {
        state.is_open = src.is_open;
        state.spring_prefix = src.spring_prefix;
    }
}

crate::ui_window!("avatar_setup", Floating, 1, build_avatar_setup_window);
