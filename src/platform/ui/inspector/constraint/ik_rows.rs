use crate::animation::{BoneId, ConstraintType, IkConstraintData};

use super::fields::{common_rows, constraint_params, remove_row, scalar_row, ConstraintView};
use crate::platform::ui::bone_choice::bone_options;
use crate::platform::ui::ui_rows::{ChoiceOption, UiRow};

pub fn ik_rows() -> Vec<UiRow<ConstraintView>> {
    let params = constraint_params();
    let mut rows = common_rows();

    rows.push(UiRow::Choice {
        label: "Effector (End Bone)",
        options: |_, view: &ConstraintView| ik_bone_options(view, |data| data.effector_bone),
        pick: |view: &mut ConstraintView, index: usize| {
            if let Some(&(bone_id, _)) = view.bones.get(index) {
                set_ik_bone(view, bone_id, |data, bone| data.effector_bone = bone);
            }
        },
    });
    rows.push(UiRow::Apply {
        label: "Use Selected for Effector",
        enabled: |_, view: &ConstraintView| view.selected_bone.is_some(),
        apply: |view: &mut ConstraintView| {
            if let Some(bone_id) = view.selected_bone {
                set_ik_bone(view, bone_id, |data, bone| data.effector_bone = bone);
            }
        },
    });

    rows.push(UiRow::Choice {
        label: "Goal (Target Position)",
        options: |_, view: &ConstraintView| ik_bone_options(view, |data| data.target_bone),
        pick: |view: &mut ConstraintView, index: usize| {
            if let Some(&(bone_id, _)) = view.bones.get(index) {
                set_ik_bone(view, bone_id, |data, bone| data.target_bone = bone);
            }
        },
    });
    rows.push(UiRow::Apply {
        label: "Use Selected for Goal",
        enabled: |_, view: &ConstraintView| view.selected_bone.is_some(),
        apply: |view: &mut ConstraintView| {
            if let Some(bone_id) = view.selected_bone {
                set_ik_bone(view, bone_id, |data, bone| data.target_bone = bone);
            }
        },
    });

    rows.push(scalar_row(params.chain_length));
    rows.push(scalar_row(params.pole_vector));
    rows.push(scalar_row(params.twist));
    rows.push(remove_row());
    rows
}

fn ik_bone_options(
    view: &ConstraintView,
    bone_of: fn(&IkConstraintData) -> BoneId,
) -> Vec<ChoiceOption> {
    let ConstraintType::Ik(data) = &view.constraint else {
        return Vec::new();
    };
    bone_options(&view.bones, view.selected_bone, bone_of(data))
}

fn set_ik_bone(
    view: &mut ConstraintView,
    bone_id: BoneId,
    assign: fn(&mut IkConstraintData, BoneId),
) {
    if let ConstraintType::Ik(data) = &mut view.constraint {
        assign(data, bone_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::ui::ui_rows::{find_duplicate_labels, visible_labels};

    #[test]
    fn ik_rows_label_order() {
        let rows = ik_rows();
        assert_eq!(
            visible_labels(&rows),
            [
                "Enabled",
                "weight",
                "Effector (End Bone)",
                "Use Selected for Effector",
                "Goal (Target Position)",
                "Use Selected for Goal",
                "chain_length",
                "pole_vector",
                "twist",
                "Remove",
            ]
        );
    }

    #[test]
    fn ik_rows_no_duplicate_labels() {
        let rows = ik_rows();
        assert!(find_duplicate_labels(&rows).is_empty());
    }
}
