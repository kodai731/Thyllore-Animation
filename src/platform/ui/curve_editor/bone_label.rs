use thyllore_avatar_core::humanoid::components::role::HumanoidRole;

use crate::animation::BoneId;

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
}
