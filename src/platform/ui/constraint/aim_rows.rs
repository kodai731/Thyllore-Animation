use crate::animation::{AimConstraintData, BoneId, ConstraintType};

use super::super::bone_choice::bone_options;
use super::super::ui_rows::{ChoiceOption, UiRow};
use super::fields::{common_rows, constraint_params, remove_row, scalar_row, ConstraintView};

pub fn aim_rows() -> Vec<UiRow<ConstraintView>> {
    let params = constraint_params();
    let mut rows = common_rows();

    rows.push(UiRow::Choice {
        label: "Aim Bone (Rotates)",
        options: |_, view: &ConstraintView| aim_bone_options(view, |data| data.source_bone),
        pick: |view: &mut ConstraintView, index: usize| {
            if let Some(&(bone_id, _)) = view.bones.get(index) {
                set_aim_bone(view, bone_id, |data, bone| data.source_bone = bone);
            }
        },
    });
    rows.push(UiRow::Apply {
        label: "Use Selected for Aim Bone",
        enabled: |_, view: &ConstraintView| view.selected_bone.is_some(),
        apply: |view: &mut ConstraintView| {
            if let Some(bone_id) = view.selected_bone {
                set_aim_bone(view, bone_id, |data, bone| data.source_bone = bone);
            }
        },
    });

    rows.push(UiRow::Choice {
        label: "Look At (Target)",
        options: |_, view: &ConstraintView| aim_bone_options(view, |data| data.target_bone),
        pick: |view: &mut ConstraintView, index: usize| {
            if let Some(&(bone_id, _)) = view.bones.get(index) {
                set_aim_bone(view, bone_id, |data, bone| data.target_bone = bone);
            }
        },
    });
    rows.push(UiRow::Apply {
        label: "Use Selected for Look At",
        enabled: |_, view: &ConstraintView| view.selected_bone.is_some(),
        apply: |view: &mut ConstraintView| {
            if let Some(bone_id) = view.selected_bone {
                set_aim_bone(view, bone_id, |data, bone| data.target_bone = bone);
            }
        },
    });

    rows.push(scalar_row(params.aim_axis));
    rows.push(scalar_row(params.up_axis));
    rows.push(remove_row());
    rows
}

fn aim_bone_options(
    view: &ConstraintView,
    bone_of: fn(&AimConstraintData) -> BoneId,
) -> Vec<ChoiceOption> {
    let ConstraintType::Aim(data) = &view.constraint else {
        return Vec::new();
    };
    bone_options(&view.bones, view.selected_bone, bone_of(data))
}

fn set_aim_bone(
    view: &mut ConstraintView,
    bone_id: BoneId,
    assign: fn(&mut AimConstraintData, BoneId),
) {
    if let ConstraintType::Aim(data) = &mut view.constraint {
        assign(data, bone_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::ui::ui_rows::{find_duplicate_labels, visible_labels};

    #[test]
    fn aim_rows_label_order() {
        let rows = aim_rows();
        assert_eq!(
            visible_labels(&rows),
            [
                "Enabled",
                "weight",
                "Aim Bone (Rotates)",
                "Use Selected for Aim Bone",
                "Look At (Target)",
                "Use Selected for Look At",
                "aim_axis",
                "up_axis",
                "Remove",
            ]
        );
    }

    #[test]
    fn aim_rows_no_duplicate_labels() {
        let rows = aim_rows();
        assert!(find_duplicate_labels(&rows).is_empty());
    }
}
