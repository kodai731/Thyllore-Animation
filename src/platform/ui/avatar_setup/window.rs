use crate::ecs::resource::{AvatarSetupState, MaterialTextureState};
use crate::ecs::systems::phases::event_dispatch::avatar_setup::AvatarSetupEvent;
use crate::ecs::world::World;
use crate::platform::ui::form_window::{FormPlacement, FormWindow, TabsSection};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;
use thyllore_avatar_core::humanoid::components::geometry_warning::GeometryWarning;
use thyllore_avatar_core::humanoid::components::rest_pose::RestPose;

use super::export_rows::ExportView;
use super::humanoid_rows::HumanoidView;
use super::material_rows::MaterialView;
use super::stats_rows::StatsView;
use super::submesh_rows::SubmeshesView;

fn avatar_setup_form(
    world: &World,
    _assets: &crate::asset::AssetStorage,
    _graphics: &GraphicsResources,
) -> Option<FormWindow> {
    let state = world.resource::<AvatarSetupState>().clone();
    if !state.is_open {
        return None;
    }
    let material_view = MaterialView::from_state(&world.resource::<MaterialTextureState>());
    let tabs: Vec<(
        &'static str,
        Vec<Box<dyn crate::platform::ui::form_window::FormSection>>,
    )> = vec![
        (
            "Humanoid",
            vec![crate::platform::ui::form_window::rows_section(
                super::humanoid_rows::rows(),
                HumanoidView::from_state(&state),
                |_, _, _| {},
            )],
        ),
        (
            "Materials",
            vec![crate::platform::ui::form_window::rows_section(
                super::material_rows::rows(),
                material_view,
                |_, _, _| {},
            )],
        ),
        (
            "Submeshes",
            vec![crate::platform::ui::form_window::rows_section(
                super::submesh_rows::rows(),
                SubmeshesView::from_state(&state),
                commit_submeshes,
            )],
        ),
        (
            "Validation",
            vec![crate::platform::ui::form_window::rows_section(
                validation_rows(),
                ValidationView::from_state(&state),
                |_, _, _| {},
            )],
        ),
        (
            "Stats",
            vec![crate::platform::ui::form_window::rows_section(
                super::stats_rows::rows(),
                StatsView::from_state(&state),
                commit_stats,
            )],
        ),
        (
            "Export",
            vec![crate::platform::ui::form_window::rows_section(
                super::export_rows::rows(),
                ExportView::from_state(&state),
                commit_export,
            )],
        ),
    ];
    Some(FormWindow {
        title: "Avatar Setup",
        placement: FormPlacement::Floating {
            initial_size: [520.0, 600.0],
            on_close: close_avatar_setup,
        },
        sections: vec![Box::new(TabsSection {
            id: "##avatar_setup_tabs",
            tabs,
        })],
    })
}

crate::ui_window!("avatar_setup", Floating, 1, form = avatar_setup_form);

fn commit_submeshes(world: &World, shown: &SubmeshesView, view: &SubmeshesView) {
    if view.chain_gen != shown.chain_gen {
        world.send_command(AvatarSetupEvent::SetChainGenSettings {
            settings: view.chain_gen.clone(),
        });
    }
}

fn commit_stats(world: &World, shown: &StatsView, view: &StatsView) {
    if view.platform != shown.platform {
        world.send_command(AvatarSetupEvent::SetAvatarRankPlatform(view.platform));
    }
}

fn commit_export(world: &World, _shown: &ExportView, view: &ExportView) {
    if let Some(mut state) = world.get_resource_mut::<AvatarSetupState>() {
        state.body_collider_fit = view.body_collider_fit.clone();
    }
}

fn close_avatar_setup(world: &World) {
    if let Some(mut state) = world.get_resource_mut::<AvatarSetupState>() {
        state.is_open = false;
    }
}

#[derive(Clone)]
pub struct ValidationView {
    pub rest_pose: RestPose,
    pub issues: Vec<String>,
    pub warnings: Vec<String>,
    pub missing_bones: Vec<String>,
}

impl ValidationView {
    pub fn from_state(state: &AvatarSetupState) -> Self {
        Self {
            rest_pose: state.rest_pose,
            issues: state.issues.iter().map(|i| i.to_string()).collect(),
            warnings: state
                .geometry_warnings
                .iter()
                .map(format_geometry_warning)
                .collect(),
            missing_bones: state.missing_bone_names.clone(),
        }
    }
}

fn validation_rows() -> Vec<crate::platform::ui::ui_rows::UiRow<ValidationView>> {
    vec![crate::platform::ui::ui_rows::UiRow::Canvas(draw_validation)]
}

fn draw_validation(ui: &imgui::Ui, _world: &World, view: &ValidationView) {
    ui.text(format!("Rest pose: {}", format_rest_pose(view.rest_pose)));
    ui.separator();

    ui.text(format!("Issues: {}", view.issues.len()));
    for issue in &view.issues {
        ui.bullet_text(issue.clone());
    }
    ui.separator();

    ui.text(format!("Warnings: {}", view.warnings.len()));
    for warning in &view.warnings {
        ui.bullet_text(warning.clone());
    }
    ui.separator();

    ui.text(format!("Missing bones: {}", view.missing_bones.len()));
    for bone_name in &view.missing_bones {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::resource::AvatarSetupState;
    use thyllore_anim_core::spring_bone::CapsuleFitParams;
    use thyllore_avatar_core::vrchat::rank::Platform;

    #[test]
    fn test_commit_export_no_change() {
        let mut world = World::new();
        world.insert_resource(AvatarSetupState::default());
        let fit = world
            .resource::<AvatarSetupState>()
            .body_collider_fit
            .clone();

        let view = ExportView {
            body_collider_fit: fit.clone(),
        };
        commit_export(&world, &view, &view);

        assert_eq!(world.resource::<AvatarSetupState>().body_collider_fit, fit);
    }

    #[test]
    fn test_commit_export_change() {
        let mut world = World::new();
        world.insert_resource(AvatarSetupState::default());

        let shown = ExportView {
            body_collider_fit: Default::default(),
        };
        let mut changed = CapsuleFitParams::default();
        changed.min_vertex_count += 1;
        let view = ExportView {
            body_collider_fit: changed.clone(),
        };
        commit_export(&world, &shown, &view);

        assert_eq!(
            world.resource::<AvatarSetupState>().body_collider_fit,
            changed
        );
    }

    #[test]
    fn test_commit_stats_no_change() {
        let world = World::new();
        let shown = StatsView {
            platform: Platform::Pc,
            rank: None,
        };
        commit_stats(&world, &shown, &shown);
    }

    #[test]
    fn test_commit_stats_change() {
        let mut world = World::new();
        world.insert_resource(crate::ecs::events::queue::EventQueue::<
            Box<dyn crate::ecs::events::UiCommand>,
        >::default());
        let shown = StatsView {
            platform: Platform::Pc,
            rank: None,
        };
        let view = StatsView {
            platform: Platform::Quest,
            rank: None,
        };
        commit_stats(&world, &shown, &view);
    }

    #[test]
    fn test_close_avatar_setup() {
        let mut world = World::new();
        let mut state = AvatarSetupState::default();
        state.is_open = true;
        world.insert_resource(state);

        close_avatar_setup(&world);

        assert!(!world.resource::<AvatarSetupState>().is_open);
    }
}
