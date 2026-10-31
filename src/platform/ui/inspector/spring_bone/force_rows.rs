use std::sync::LazyLock;

use thyllore_anim_core::spring_bone::{
    JointForceForm, RestRelativeJointForce, SpringForceModel, VrmJointForce,
    REST_RELATIVE_JOINT_FORCE_UI_PARAMS, VRM_JOINT_FORCE_UI_PARAMS,
};
use thyllore_scene_core::{FieldPath, ScalarParam, SceneFields};

use crate::platform::ui::ui_rows::{ChoiceOption, FieldRows, UiRow};

struct VrmPath;

impl FieldPath<JointForceForm> for VrmPath {
    type Field = VrmJointForce;

    fn get(root: &JointForceForm) -> &VrmJointForce {
        &root.vrm
    }

    fn get_mut(root: &mut JointForceForm) -> &mut VrmJointForce {
        &mut root.vrm
    }
}

struct RestRelativePath;

impl FieldPath<JointForceForm> for RestRelativePath {
    type Field = RestRelativeJointForce;

    fn get(root: &JointForceForm) -> &RestRelativeJointForce {
        &root.rest_relative
    }

    fn get_mut(root: &mut JointForceForm) -> &mut RestRelativeJointForce {
        &mut root.rest_relative
    }
}

static VRM_SCALARS: LazyLock<Vec<ScalarParam<JointForceForm>>> = LazyLock::new(|| {
    let mut out = Vec::new();
    VrmJointForce::collect_scalars::<JointForceForm, VrmPath>("", &mut out);
    out
});

static REST_RELATIVE_SCALARS: LazyLock<Vec<ScalarParam<JointForceForm>>> = LazyLock::new(|| {
    let mut out = Vec::new();
    RestRelativeJointForce::collect_scalars::<JointForceForm, RestRelativePath>("", &mut out);
    out
});

/// The model choice followed by the magnitudes of the chosen model.
pub fn force_form_rows(form: &JointForceForm) -> Vec<UiRow<JointForceForm>> {
    let mut rows = vec![UiRow::Choice {
        label: "Force Model",
        options: |_, form: &JointForceForm| {
            SpringForceModel::ALL
                .iter()
                .map(|model| ChoiceOption {
                    label: model.label().to_string(),
                    selected: form.model == *model,
                })
                .collect()
        },
        pick: |form: &mut JointForceForm, index: usize| {
            if let Some(model) = SpringForceModel::ALL.get(index) {
                form.model = *model;
            }
        },
    }];
    match form.model {
        SpringForceModel::Vrm => {
            rows.extend(VRM_JOINT_FORCE_UI_PARAMS.iter().map(|param| UiRow::Scalar {
                param,
                scalars: &VRM_SCALARS,
            }))
        }
        SpringForceModel::RestRelative => rows.extend(
            REST_RELATIVE_JOINT_FORCE_UI_PARAMS
                .iter()
                .map(|param| UiRow::Scalar {
                    param,
                    scalars: &REST_RELATIVE_SCALARS,
                }),
        ),
    }
    rows
}

/// The force form of a view embedded as one field block.
pub fn force_field_row<C: 'static>(
    scope: &'static str,
    form: &JointForceForm,
    field: fn(&mut C) -> &mut JointForceForm,
) -> UiRow<C> {
    UiRow::Field(Box::new(FieldRows {
        scope,
        field,
        rows: force_form_rows(form),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::ui::ui_rows::visible_labels;

    #[test]
    fn force_rows_show_the_chosen_model_only() {
        let vrm = JointForceForm {
            model: SpringForceModel::Vrm,
            ..Default::default()
        };
        assert_eq!(
            visible_labels(&force_form_rows(&vrm)),
            ["Force Model", "stiffness", "gravity_power"]
        );

        let rest_relative = JointForceForm {
            model: SpringForceModel::RestRelative,
            ..Default::default()
        };
        assert_eq!(
            visible_labels(&force_form_rows(&rest_relative)),
            ["Force Model", "bending_relaxation", "gravity_scale"]
        );
    }
}
