use thyllore_avatar_core::humanoid::components::role::HumanoidRole;

use crate::animation::editable::SourceClipId;
use crate::animation::BoneId;
use crate::ecs::resource::{CurveEditorState, RecipeClipSource};
use crate::ecs::systems::phases::event_dispatch::motion_recipe::MotionRecipeEvent;
use crate::ecs::world::World;

pub(super) fn format_bone_label(bone_name: &str, role: Option<HumanoidRole>) -> String {
    match role {
        Some(role) => format!("{:?} ({})", role, bone_name),
        None => bone_name.to_string(),
    }
}

pub(super) fn order_bone_ids_by_role(
    bone_ids: &[BoneId],
    roles: &[(BoneId, HumanoidRole)],
) -> Vec<BoneId> {
    let find_role = |bone_id: BoneId| {
        roles
            .iter()
            .find(|(role_bone_id, _)| *role_bone_id == bone_id)
            .map(|(_, role)| *role)
    };

    let mut with_role: Vec<(HumanoidRole, BoneId)> = Vec::new();
    let mut without_role: Vec<BoneId> = Vec::new();
    for &bone_id in bone_ids {
        match find_role(bone_id) {
            Some(role) => with_role.push((role, bone_id)),
            None => without_role.push(bone_id),
        }
    }
    with_role.sort();
    without_role.sort();

    with_role
        .into_iter()
        .map(|(_, bone_id)| bone_id)
        .chain(without_role)
        .collect()
}

pub(super) fn is_pose_key_time(time: f32, pose_times: &[f32]) -> bool {
    pose_times
        .iter()
        .any(|&pose_time| (time - pose_time).abs() < 1e-3)
}

pub(super) fn build_recipe_section(
    ui: &imgui::Ui,
    world: &World,
    editor_state: &mut CurveEditorState,
    clip: SourceClipId,
    source: &RecipeClipSource,
    role: Option<HumanoidRole>,
) {
    ui.separator();

    let file_name = source
        .path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    ui.text(format!("Recipe: {}", file_name));

    if source.detached {
        ui.text("(detached from recipe)");
        if ui.small_button("Rebake from recipe") {
            world.send_command(MotionRecipeEvent::RebakeFromRecipe { clip });
        }
    } else if let Some(role) = role {
        for (i, pose_rotations) in source.pose_rotations.iter().enumerate() {
            let Some(current_value) = pose_rotations.get(&role) else {
                continue;
            };

            ui.text(format!("t={:.2}s", source.pose_times[i]));
            ui.same_line();

            let mut value = match editor_state.recipe_edit {
                Some((edit_index, edit_value)) if edit_index == i => edit_value,
                _ => *current_value,
            };

            let changed = imgui::Drag::new(format!("##recipe_pose_{i}"))
                .speed(0.5)
                .build_array(ui, &mut value);
            if changed {
                editor_state.recipe_edit = Some((i, value));
            }

            if ui.is_item_deactivated_after_edit() {
                world.send_command(MotionRecipeEvent::SetPoseRotation {
                    clip,
                    role,
                    pose_index: i,
                    euler: value,
                });
                editor_state.recipe_edit = None;
            }
        }
    } else {
        ui.text("Select a bone with a humanoid role");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bone_label_with_role() {
        let label = format_bone_label("mixamorig:Hips", Some(HumanoidRole::Hips));
        assert_eq!(label, "Hips (mixamorig:Hips)");
    }

    #[test]
    fn test_order_bone_ids_puts_roles_first() {
        let bone_ids: Vec<BoneId> = vec![3, 1, 2];
        let roles: Vec<(BoneId, HumanoidRole)> = vec![(3, HumanoidRole::Hips)];
        let ordered = order_bone_ids_by_role(&bone_ids, &roles);
        assert_eq!(ordered, vec![3, 1, 2]);
    }

    #[test]
    fn test_is_pose_key_time_matches_within_tolerance() {
        let pose_times = [0.0, 1.0, 2.0];
        assert!(is_pose_key_time(0.0, &pose_times));
        assert!(is_pose_key_time(1.0, &pose_times));
        assert!(is_pose_key_time(2.0, &pose_times));
        assert!(is_pose_key_time(0.999, &pose_times));
        assert!(is_pose_key_time(1.0009, &pose_times));
        assert!(!is_pose_key_time(0.5, &pose_times));
        assert!(!is_pose_key_time(1.001, &pose_times));
        assert!(!is_pose_key_time(3.0, &pose_times));
    }
}
