use std::sync::LazyLock;

use thyllore_anim_core::spring_bone::{JointForceForm, SpringJointParam, SPRING_JOINT_UI_PARAMS};
use thyllore_scene_core::{FieldPath, ScalarParam, SceneFields};

use crate::animation::BoneId;
use crate::ecs::component::SpringChain;
use crate::ecs::systems::phases::event_dispatch::spring_bone::SpringBoneEvent;
use crate::ecs::world::Entity;

use super::super::bone_choice::bone_options;
use super::super::ui_rows::{ListRows, UiRow};
use super::force_rows::force_field_row;
use super::section::SpringView;

#[derive(Clone)]
pub struct ChainView {
    pub entity: Entity,
    pub chain: SpringChain,
    pub bones: std::rc::Rc<[(BoneId, String)]>,
    pub selected_bone: Option<BoneId>,
}

#[derive(Clone)]
pub struct JointView {
    pub bones: std::rc::Rc<[(BoneId, String)]>,
    pub selected_bone: Option<BoneId>,
    pub joint: SpringJointParam,
    pub force: JointForceForm,
}

impl JointView {
    pub fn new(
        bones: std::rc::Rc<[(BoneId, String)]>,
        selected_bone: Option<BoneId>,
        joint: SpringJointParam,
    ) -> Self {
        let force = JointForceForm::from(joint.force);
        Self {
            bones,
            selected_bone,
            joint,
            force,
        }
    }

    /// The joint with the force the form currently selects.
    pub fn joint_param(&self) -> SpringJointParam {
        SpringJointParam {
            force: self.force.into(),
            ..self.joint.clone()
        }
    }
}

struct JointPath;

impl FieldPath<JointView> for JointPath {
    type Field = SpringJointParam;

    fn get(root: &JointView) -> &SpringJointParam {
        &root.joint
    }

    fn get_mut(root: &mut JointView) -> &mut SpringJointParam {
        &mut root.joint
    }
}

static JOINT_SCALARS: LazyLock<Vec<ScalarParam<JointView>>> = LazyLock::new(|| {
    let mut out = Vec::new();
    SpringJointParam::collect_scalars::<JointView, JointPath>("", &mut out);
    out
});

pub fn chain_list_row() -> UiRow<SpringView> {
    UiRow::List(Box::new(ListRows::<SpringView, ChainView> {
        title: "Chains",
        items: |_, view: &SpringView| {
            view.chains
                .iter()
                .map(|chain| {
                    let header = format!("Chain: {} (id:{})", chain.name, chain.id);
                    let item = ChainView {
                        entity: view.entity,
                        chain: chain.clone(),
                        bones: view.bones.clone(),
                        selected_bone: view.selected_bone,
                    };
                    (header, item)
                })
                .collect()
        },
        rows: |_| chain_rows(),
        on_change: |_, _, chain_view| {
            Box::new(SpringBoneEvent::ChainUpdate {
                entity: chain_view.entity,
                chain_id: chain_view.chain.id,
                chain: chain_view.chain.clone(),
            }) as _
        },
        initially_open: |_| false,
    }))
}

pub fn chain_rows() -> Vec<UiRow<ChainView>> {
    let mut rows: Vec<UiRow<ChainView>> = Vec::new();

    rows.push(UiRow::Toggle {
        label: "Enabled",
        get: |view: &ChainView| view.chain.enabled,
        set: |view: &mut ChainView, value: bool| {
            view.chain.enabled = value;
        },
    });

    rows.push(UiRow::List(Box::new(ListRows::<ChainView, JointView> {
        title: "Joints",
        items: |_, view: &ChainView| {
            view.chain
                .joints
                .iter()
                .enumerate()
                .map(|(i, joint)| {
                    let header = format!("Joint {} (bone:{})", i, joint.bone_id);
                    let item =
                        JointView::new(view.bones.clone(), view.selected_bone, joint.clone());
                    (header, item)
                })
                .collect()
        },
        rows: joint_rows,
        on_change: |chain_view, joint_index, joint_view| {
            Box::new(SpringBoneEvent::JointUpdate {
                entity: chain_view.entity,
                chain_id: chain_view.chain.id,
                joint_index,
                joint: joint_view.joint_param(),
            }) as _
        },
        initially_open: |_| false,
    })));

    rows.push(UiRow::Action {
        label: "Remove Chain",
        enabled: |_, _: &ChainView| true,
        on_click: |view: &ChainView| {
            Box::new(SpringBoneEvent::ChainRemove {
                entity: view.entity,
                chain_id: view.chain.id,
            }) as _
        },
    });

    rows
}

pub fn joint_rows(view: &JointView) -> Vec<UiRow<JointView>> {
    let mut rows: Vec<UiRow<JointView>> = Vec::new();
    rows.push(UiRow::Choice {
        label: "Bone",
        options: |_, view: &JointView| {
            bone_options(&view.bones, view.selected_bone, view.joint.bone_id)
        },
        pick: |view: &mut JointView, index: usize| {
            if let Some((bone_id, _)) = view.bones.get(index) {
                view.joint.bone_id = *bone_id;
            }
        },
    });
    rows.push(UiRow::Apply {
        label: "Use Selected Bone",
        enabled: |_, view: &JointView| view.selected_bone.is_some(),
        apply: |view: &mut JointView| {
            if let Some(bone_id) = view.selected_bone {
                view.joint.bone_id = bone_id;
            }
        },
    });

    rows.extend(joint_param_rows(view));
    rows
}

/// The force block of the joint's model, then the parameters every model shares.
pub fn joint_param_rows(view: &JointView) -> Vec<UiRow<JointView>> {
    let mut rows = vec![force_field_row(&view.force, |view: &mut JointView| {
        &mut view.force
    })];
    rows.extend(SPRING_JOINT_UI_PARAMS.iter().map(|param| UiRow::Scalar {
        param,
        scalars: &JOINT_SCALARS,
    }));
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::ui::ui_rows::{find_duplicate_labels, visible_labels};

    fn view() -> JointView {
        JointView::new(
            std::rc::Rc::from(Vec::new()),
            None,
            SpringJointParam::default(),
        )
    }

    #[test]
    fn joint_rows_label_order() {
        let rows = joint_rows(&view());
        let labels = visible_labels(&rows);
        assert_eq!(
            labels,
            [
                "Bone",
                "Use Selected Bone",
                "drag_force",
                "gravity_dir",
                "hit_radius",
            ]
        );
    }

    #[test]
    fn joint_param_rows_start_with_the_force_block() {
        let rows = joint_param_rows(&view());
        assert!(matches!(rows[0], UiRow::Field(_)));
        let labels = visible_labels(&rows);
        let declared: Vec<&str> = SPRING_JOINT_UI_PARAMS
            .iter()
            .map(|param| param.name)
            .collect();
        assert_eq!(labels, declared);
    }

    #[test]
    fn joint_param_round_trips_the_force_form() {
        let mut joint_view = view();
        joint_view.force.model = thyllore_anim_core::spring_bone::SpringForceModel::Vrm;
        joint_view.force.vrm.stiffness = 2.5;
        assert_eq!(joint_view.joint_param().force.magnitudes()[0], 2.5);
    }

    #[test]
    fn joint_rows_no_duplicate_labels() {
        let rows = joint_rows(&view());
        assert!(find_duplicate_labels(&rows).is_empty());
    }

    #[test]
    fn chain_rows_label_order() {
        let rows = chain_rows();
        let labels = visible_labels(&rows);
        assert_eq!(labels, ["Enabled", "Joints", "Remove Chain"]);
    }

    #[test]
    fn chain_rows_no_duplicate_labels() {
        let rows = chain_rows();
        assert!(find_duplicate_labels(&rows).is_empty());
    }
}
