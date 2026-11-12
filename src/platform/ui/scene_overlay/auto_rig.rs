#![cfg(feature = "auto-rig")]

use super::toolbar::TOOLBAR_GROUP_GAP;
use crate::ecs::component::GlbSource;
use crate::ecs::resource::{AutoRigState, AutoRigStatus, HierarchyState};
use crate::ecs::systems::phases::event_dispatch::ml::auto_rig::AutoRigEvent;
use crate::ecs::world::Parent;
use crate::ecs::World;
use crate::platform::ui::text_to_animation_dialog::TextToAnimationDialogState;
use crate::platform::ui::text_to_mesh_dialog::TextToMeshDialogState;

const AUTO_RIG_SAMPLE_POINTS: u32 = 65536;
const RIG_ERROR_COLOR: [f32; 4] = [1.0, 0.3, 0.3, 1.0];

pub(super) fn build_auto_rig_buttons(ui: &imgui::Ui, ecs_world: &World) {
    ui.same_line_with_spacing(0.0, TOOLBAR_GROUP_GAP);
    if ui.button("Generate Mesh") {
        ecs_world.resource_mut::<TextToMeshDialogState>().open = true;
    }

    ui.same_line();
    if ui.button("Generate Animation") {
        ecs_world.resource_mut::<TextToAnimationDialogState>().open = true;
    }

    ui.same_line();
    build_auto_rig_status(ui, ecs_world);
}

fn build_auto_rig_status(ui: &imgui::Ui, ecs_world: &World) {
    let status = ecs_world.resource::<AutoRigState>().status.clone();

    match status {
        AutoRigStatus::Idle => build_generate_button(ui, ecs_world),
        AutoRigStatus::WaitingForServer => {
            build_cancellable_progress(ui, ecs_world, "Rigging: waiting for server...")
        }
        AutoRigStatus::Rigging => {
            build_cancellable_progress(ui, ecs_world, "Rigging: processing...")
        }
        AutoRigStatus::Previewing => build_preview_actions(ui, ecs_world),
        AutoRigStatus::Error => build_error_notice(ui, ecs_world),
    }
}

fn build_generate_button(ui: &imgui::Ui, ecs_world: &World) {
    if !selection_has_glb_source(ecs_world) {
        return;
    }

    if ui.button("Auto Rig") {
        ecs_world.send_command(AutoRigEvent::AutoRigGenerate {
            num_sample_points: AUTO_RIG_SAMPLE_POINTS,
        });
    }
}

fn selection_has_glb_source(ecs_world: &World) -> bool {
    let Some(entity) = ecs_world.resource::<HierarchyState>().selected_entity else {
        return false;
    };
    if ecs_world.get_component::<GlbSource>(entity).is_some() {
        return true;
    }

    ecs_world
        .get_component::<Parent>(entity)
        .is_some_and(|parent| ecs_world.get_component::<GlbSource>(parent.0).is_some())
}

fn build_cancellable_progress(ui: &imgui::Ui, ecs_world: &World, message: &str) {
    ui.text(message);
    ui.same_line();
    if ui.button("Cancel##rig") {
        ecs_world.send_command(AutoRigEvent::AutoRigDiscard);
    }
}

fn build_preview_actions(ui: &imgui::Ui, ecs_world: &World) {
    let preview_summary = {
        let auto_rig_state = ecs_world.resource::<AutoRigState>();
        auto_rig_state.generation_time_ms.map(|generation_time_ms| {
            format!(
                "Preview: {} joints, {} bones ({:.1}s)",
                auto_rig_state.joint_count.unwrap_or(0),
                auto_rig_state.bone_count.unwrap_or(0),
                generation_time_ms / 1000.0
            )
        })
    };

    if let Some(summary) = preview_summary {
        ui.text(summary);
        ui.same_line();
    }
    if ui.button("Apply Rig") {
        ecs_world.send_command(AutoRigEvent::AutoRigApply);
    }
    ui.same_line();
    if ui.button("Discard##rig") {
        ecs_world.send_command(AutoRigEvent::AutoRigDiscard);
    }
}

fn build_error_notice(ui: &imgui::Ui, ecs_world: &World) {
    let error_message = ecs_world.resource::<AutoRigState>().error_message.clone();

    if let Some(message) = error_message {
        ui.text_colored(RIG_ERROR_COLOR, format!("Rig error: {}", message));
        ui.same_line();
    }
    if ui.button("Dismiss##rig") {
        ecs_world.send_command(AutoRigEvent::AutoRigDiscard);
    }
}
