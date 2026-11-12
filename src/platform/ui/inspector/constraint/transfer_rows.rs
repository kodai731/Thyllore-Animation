use crate::animation::ConstraintType;

use super::fields::{
    applied_to_rows, common_rows, constraint_params, remove_row, scalar_row, ConstraintView,
};
use crate::platform::ui::bone_choice::bone_options;
use crate::platform::ui::ui_rows::UiRow;

pub fn transfer_rows(view: &ConstraintView) -> Vec<UiRow<ConstraintView>> {
    let params = constraint_params();
    let offset_param = match &view.constraint {
        ConstraintType::Position(_) | ConstraintType::Scale(_) => params.offset,
        ConstraintType::Rotation(_) => params.rotation_offset,
        ConstraintType::Ik(_) | ConstraintType::Aim(_) | ConstraintType::Parent(_) => {
            unreachable!("transfer rows are declared for Position, Rotation and Scale only")
        }
    };

    let mut rows = common_rows();
    rows.extend(applied_to_rows());

    rows.push(UiRow::Choice {
        label: "Copy From",
        options: |_, view: &ConstraintView| {
            view.constraint
                .target_bone_id()
                .map_or_else(Vec::new, |bone_id| {
                    bone_options(&view.bones, view.selected_bone, bone_id)
                })
        },
        pick: |view: &mut ConstraintView, index: usize| {
            if let Some(&(bone_id, _)) = view.bones.get(index) {
                view.constraint.set_target_bone_id(bone_id);
            }
        },
    });
    rows.push(UiRow::Apply {
        label: "Use Selected for Copy From",
        enabled: |_, view: &ConstraintView| view.selected_bone.is_some(),
        apply: |view: &mut ConstraintView| {
            if let Some(bone_id) = view.selected_bone {
                view.constraint.set_target_bone_id(bone_id);
            }
        },
    });

    rows.push(scalar_row(offset_param));
    rows.push(affect_axis_row::<0>("Affect X"));
    rows.push(affect_axis_row::<1>("Affect Y"));
    rows.push(affect_axis_row::<2>("Affect Z"));
    rows.push(remove_row());
    rows
}

fn affect_axis_row<const AXIS: usize>(label: &'static str) -> UiRow<ConstraintView> {
    UiRow::Toggle {
        label,
        get: |view: &ConstraintView| view.constraint.affect_axes().is_some_and(|axes| axes[AXIS]),
        set: |view: &mut ConstraintView, value: bool| {
            if let Some(mut axes) = view.constraint.affect_axes() {
                axes[AXIS] = value;
                view.constraint.set_affect_axes(axes);
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::{PositionConstraintData, RotationConstraintData, ScaleConstraintData};
    use crate::platform::ui::inspector::constraint::fields::ConstraintFields;
    use crate::platform::ui::ui_rows::{find_duplicate_labels, visible_labels};

    fn view_of(constraint: ConstraintType) -> ConstraintView {
        ConstraintView {
            entity: 0,
            entry_id: 1,
            fields: ConstraintFields::from_constraint(&constraint),
            constraint,
            bones: Vec::new().into(),
            selected_bone: None,
        }
    }

    fn expected_labels(offset_label: &'static str) -> [&'static str; 11] {
        [
            "Enabled",
            "weight",
            "Applied To",
            "Use Selected for Applied To",
            "Copy From",
            "Use Selected for Copy From",
            offset_label,
            "Affect X",
            "Affect Y",
            "Affect Z",
            "Remove",
        ]
    }

    #[test]
    fn position_rows_label_order() {
        let view = view_of(ConstraintType::Position(PositionConstraintData::default()));
        assert_eq!(
            visible_labels(&transfer_rows(&view)),
            expected_labels("offset")
        );
    }

    #[test]
    fn rotation_rows_label_order() {
        let view = view_of(ConstraintType::Rotation(RotationConstraintData::default()));
        assert_eq!(
            visible_labels(&transfer_rows(&view)),
            expected_labels("rotation_offset")
        );
    }

    #[test]
    fn scale_rows_label_order() {
        let view = view_of(ConstraintType::Scale(ScaleConstraintData::default()));
        assert_eq!(
            visible_labels(&transfer_rows(&view)),
            expected_labels("offset")
        );
    }

    #[test]
    fn transfer_rows_have_no_duplicate_labels() {
        let constraints = [
            ConstraintType::Position(PositionConstraintData::default()),
            ConstraintType::Rotation(RotationConstraintData::default()),
            ConstraintType::Scale(ScaleConstraintData::default()),
        ];
        for constraint in constraints {
            let rows = transfer_rows(&view_of(constraint));
            assert!(find_duplicate_labels(&rows).is_empty());
        }
    }

    #[test]
    fn affect_axis_toggle_writes_one_axis() {
        let mut view = view_of(ConstraintType::Position(PositionConstraintData::default()));
        let UiRow::Toggle { set, .. } = affect_axis_row::<1>("Affect Y") else {
            panic!("affect axis row is a toggle");
        };

        set(&mut view, false);

        assert_eq!(view.constraint.affect_axes(), Some([true, false, true]));
    }
}
