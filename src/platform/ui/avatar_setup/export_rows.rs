use std::sync::LazyLock;

use thyllore_anim_core::spring_bone::{CapsuleFitParams, CAPSULE_FIT_UI_PARAMS};
use thyllore_scene_core::{FieldPath, ScalarParam, SceneFields};

use crate::ecs::events::{send_dialog_request, DialogRequest};
use crate::ecs::resource::{AvatarSetupState, FbxModelCache};
use crate::ecs::systems::phases::event_dispatch::avatar_setup::AvatarSetupEvent;
use crate::platform::ui::ui_rows::UiRow;

#[derive(Clone)]
pub struct ExportView {
    pub spring_prefix: String,
    pub body_collider_fit: CapsuleFitParams,
}

impl ExportView {
    pub fn from_state(state: &AvatarSetupState) -> Self {
        Self {
            spring_prefix: state.spring_prefix.clone(),
            body_collider_fit: state.body_collider_fit.clone(),
        }
    }
}

struct BodyColliderFitPath;

impl FieldPath<ExportView> for BodyColliderFitPath {
    type Field = CapsuleFitParams;

    fn get(root: &ExportView) -> &CapsuleFitParams {
        &root.body_collider_fit
    }

    fn get_mut(root: &mut ExportView) -> &mut CapsuleFitParams {
        &mut root.body_collider_fit
    }
}

static BODY_COLLIDER_FIT_SCALARS: LazyLock<Vec<ScalarParam<ExportView>>> = LazyLock::new(|| {
    let mut out = Vec::new();
    CapsuleFitParams::collect_scalars::<ExportView, BodyColliderFitPath>("", &mut out);
    out
});

const BODY_COLLIDER_FIT_HELP: &str =
    "Fit Body Colliders replaces the \"Fitted Body\" group with a \
capsule per mapped body bone, fitted to the skin, and assigns the group to every chain.";

fn body_collider_fit_group() -> UiRow<ExportView> {
    let mut rows: Vec<UiRow<ExportView>> = vec![UiRow::Canvas(|ui, _, _| {
        ui.text_wrapped(BODY_COLLIDER_FIT_HELP);
    })];
    rows.extend(CAPSULE_FIT_UI_PARAMS.iter().map(|param| UiRow::Scalar {
        param,
        scalars: &BODY_COLLIDER_FIT_SCALARS,
    }));
    rows.push(UiRow::Action {
        label: "Fit Body Colliders",
        enabled: |_world, _view| true,
        on_click: |view| {
            Box::new(AvatarSetupEvent::FitBodyColliders {
                params: view.body_collider_fit.clone(),
            })
        },
    });
    UiRow::Group {
        title: "Body Colliders".into(),
        initially_open: true,
        rows,
    }
}

pub fn rows() -> Vec<UiRow<ExportView>> {
    vec![
        UiRow::Action {
            label: "Export for Unity",
            enabled: |_world, _view| true,
            on_click: |_view| Box::new(AvatarSetupEvent::ExportUnityAvatar),
        },
        UiRow::Canvas(|ui, _world, _view| {
            ui.text_disabled("sidecar JSON, expression .anim files, current clip .anim");
        }),
        UiRow::Canvas(|ui, world, _view| {
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
        }),
        UiRow::Text {
            label: "Bone Name Prefix",
            get: |view| view.spring_prefix.as_str(),
            set: |view, text| view.spring_prefix = text,
        },
        UiRow::Action {
            label: "Add Spring Chains",
            enabled: |_world, view| !view.spring_prefix.trim().is_empty(),
            on_click: |view| {
                Box::new(AvatarSetupEvent::AddSpringChainsByPrefix {
                    prefix: view.spring_prefix.trim().to_string(),
                })
            },
        },
        body_collider_fit_group(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::ui::ui_rows::visible_labels;

    #[test]
    fn test_row_labels() {
        let rows = rows();
        let labels = visible_labels(&rows);
        assert_eq!(
            labels,
            ["Export for Unity", "Bone Name Prefix", "Add Spring Chains"]
        );
        let Some(UiRow::Group { title, .. }) = rows.last() else {
            panic!("the export tab ends with the body collider group");
        };
        assert_eq!(title, "Body Colliders");
    }

    #[test]
    fn test_body_collider_rows_list_every_fit_param_before_the_action() {
        let UiRow::Group { rows, .. } = body_collider_fit_group() else {
            panic!("body collider fit is a group");
        };
        let labels = visible_labels(&rows);
        let expected: Vec<&str> = CAPSULE_FIT_UI_PARAMS
            .iter()
            .map(|param| param.name)
            .chain(["Fit Body Colliders"])
            .collect();
        assert_eq!(labels, expected);
    }

    #[test]
    fn test_no_duplicate_labels() {
        let rows = rows();
        let mut seen = std::collections::HashSet::new();
        for label in visible_labels(&rows) {
            assert!(seen.insert(label), "duplicate label: {}", label);
        }
    }
}
