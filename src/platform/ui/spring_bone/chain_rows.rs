use std::sync::LazyLock;

use thyllore_anim_core::spring_bone::{SpringJointParam, SPRING_JOINT_UI_PARAMS};
use thyllore_scene_core::{FieldPath, ScalarParam, SceneFields};

use crate::animation::BoneId;
use crate::ecs::component::{ChainRamp, SpringChain, SpringChainId};
use crate::ecs::systems::phases::event_dispatch::spring_bone::{
    ApplyChainRamp, LoadChainRamp, SpringBoneEvent,
};
use crate::ecs::world::Entity;

use super::super::bone_choice::bone_options;
use super::super::ui_rows::{ListRows, UiRow};
use super::section::SpringView;

#[derive(Clone)]
pub struct ChainView {
    pub entity: Entity,
    pub chain: SpringChain,
    pub ramp: ChainRamp,
    pub ramp_targets: Vec<SpringChainId>,
    pub bones: std::rc::Rc<[(BoneId, String)]>,
    pub selected_bone: Option<BoneId>,
}

#[derive(Clone)]
pub struct JointView {
    pub bones: std::rc::Rc<[(BoneId, String)]>,
    pub selected_bone: Option<BoneId>,
    pub joint: SpringJointParam,
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
                .zip(&view.ramp_targets)
                .map(|(chain, targets)| {
                    let header = format!("Chain: {} (id:{})", chain.name, chain.id);
                    let item = ChainView {
                        entity: view.entity,
                        chain: chain.clone(),
                        ramp: view.ramp.clone(),
                        ramp_targets: targets.clone(),
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
                    let item = JointView {
                        bones: view.bones.clone(),
                        selected_bone: view.selected_bone,
                        joint: joint.clone(),
                    };
                    (header, item)
                })
                .collect()
        },
        rows: |_| joint_rows(),
        on_change: |chain_view, joint_index, joint_view| {
            Box::new(SpringBoneEvent::JointUpdate {
                entity: chain_view.entity,
                chain_id: chain_view.chain.id,
                joint_index,
                joint: joint_view.joint,
            }) as _
        },
        initially_open: |_| false,
    })));

    rows.push(UiRow::Action {
        label: "Load Ramp From Chain",
        enabled: |_, _: &ChainView| true,
        on_click: |view: &ChainView| {
            Box::new(LoadChainRamp {
                entity: view.entity,
                chain_id: view.chain.id,
            }) as _
        },
    });

    rows.push(UiRow::Action {
        label: "Apply Ramp",
        enabled: |_, view: &ChainView| !view.ramp_targets.is_empty(),
        on_click: |view: &ChainView| {
            Box::new(ApplyChainRamp {
                entity: view.entity,
                chain_ids: view.ramp_targets.clone(),
                ramp: view.ramp.clone(),
            }) as _
        },
    });

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

pub fn joint_rows() -> Vec<UiRow<JointView>> {
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

    rows.extend(joint_param_rows());
    rows
}

pub fn joint_param_rows() -> Vec<UiRow<JointView>> {
    SPRING_JOINT_UI_PARAMS
        .iter()
        .map(|param| UiRow::Scalar {
            param,
            scalars: &JOINT_SCALARS,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::ui::ui_rows::{find_duplicate_labels, visible_labels};

    #[test]
    fn joint_rows_label_order() {
        let rows = joint_rows();
        let labels = visible_labels(&rows);
        assert_eq!(
            labels,
            [
                "Bone",
                "Use Selected Bone",
                "stiffness",
                "drag_force",
                "gravity_power",
                "gravity_dir",
                "hit_radius",
            ]
        );
    }

    #[test]
    fn joint_param_rows_follow_the_declaration_order() {
        let rows = joint_param_rows();
        let labels = visible_labels(&rows);
        let declared: Vec<&str> = SPRING_JOINT_UI_PARAMS
            .iter()
            .map(|param| param.name)
            .collect();
        assert_eq!(labels, declared);
    }

    #[test]
    fn joint_rows_no_duplicate_labels() {
        let rows = joint_rows();
        assert!(find_duplicate_labels(&rows).is_empty());
    }

    #[test]
    fn chain_rows_label_order() {
        let rows = chain_rows();
        let labels = visible_labels(&rows);
        assert_eq!(
            labels,
            [
                "Enabled",
                "Joints",
                "Load Ramp From Chain",
                "Apply Ramp",
                "Remove Chain"
            ]
        );
    }

    #[test]
    fn chain_rows_no_duplicate_labels() {
        let rows = chain_rows();
        assert!(find_duplicate_labels(&rows).is_empty());
    }
}
