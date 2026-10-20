use std::sync::LazyLock;

use thyllore_anim_core::spring_bone::{chains_in_scope, RampShape, CHAIN_RAMP_UI_PARAMS};
use thyllore_scene_core::{FieldPath, ScalarParam, SceneFields};

use crate::animation::Skeleton;
use crate::ecs::component::{ChainRamp, RampScope, SpringBoneSetup, SpringChainId};
use crate::ecs::systems::phases::event_dispatch::spring_bone::{
    ApplyChainRamp, LoadChainRamp, SetChainRampForm,
};
use crate::ecs::world::Entity;

use super::super::ui_rows::{ChoiceOption, ListRows, UiRow};
use super::bone_section::BoneSpringView;

#[derive(Clone)]
pub struct RampFormView {
    pub entity: Entity,
    pub chain_id: SpringChainId,
    pub targets: Vec<SpringChainId>,
    pub ramp: ChainRamp,
    pub scope: RampScope,
}

struct RampPath;

impl FieldPath<RampFormView> for RampPath {
    type Field = ChainRamp;

    fn get(root: &RampFormView) -> &ChainRamp {
        &root.ramp
    }

    fn get_mut(root: &mut RampFormView) -> &mut ChainRamp {
        &mut root.ramp
    }
}

static RAMP_SCALARS: LazyLock<Vec<ScalarParam<RampFormView>>> = LazyLock::new(|| {
    let mut out = Vec::new();
    ChainRamp::collect_scalars::<RampFormView, RampPath>("", &mut out);
    out
});

/// The chains a ramp applied from `chain_id` reaches under `scope`; without a skeleton only the chain itself.
pub fn resolve_ramp_targets(
    setup: &SpringBoneSetup,
    skeleton: Option<&Skeleton>,
    chain_id: SpringChainId,
    scope: RampScope,
) -> Vec<SpringChainId> {
    match skeleton {
        Some(skeleton) => chains_in_scope(setup, skeleton, chain_id, scope),
        None => vec![chain_id],
    }
}

pub fn ramp_list_row() -> UiRow<BoneSpringView> {
    UiRow::List(Box::new(ListRows::<BoneSpringView, RampFormView> {
        title: "Ramp",
        items: |_, view: &BoneSpringView| {
            view.joints
                .iter()
                .zip(&view.ramp_targets)
                .map(|(joint, targets)| {
                    let item = RampFormView {
                        entity: view.entity,
                        chain_id: joint.chain_id,
                        targets: targets.clone(),
                        ramp: view.ramp.clone(),
                        scope: view.ramp_scope,
                    };
                    (joint.chain_name.clone(), item)
                })
                .collect()
        },
        rows: |_| ramp_form_rows(),
        on_change: |_, _, form| {
            Box::new(SetChainRampForm {
                ramp: form.ramp,
                scope: form.scope,
            })
        },
        initially_open: |_| true,
    }))
}

fn ramp_param_rows(hidden: &[&str]) -> Vec<UiRow<RampFormView>> {
    CHAIN_RAMP_UI_PARAMS
        .iter()
        .filter(|param| !hidden.contains(&param.name))
        .map(|param| UiRow::Scalar {
            param,
            scalars: &RAMP_SCALARS,
        })
        .collect()
}

pub fn ramp_form_rows() -> Vec<UiRow<RampFormView>> {
    let mut rows = Vec::new();
    rows.push(UiRow::Heading("Root"));
    rows.extend(ramp_param_rows(&[
        "tip_stiffness",
        "tip_drag_force",
        "tip_gravity_power",
        "tip_gravity_dir",
        "tip_hit_radius",
    ]));
    rows.push(UiRow::Heading("Tip"));
    rows.extend(ramp_param_rows(&[
        "root_stiffness",
        "root_drag_force",
        "root_gravity_power",
        "root_gravity_dir",
        "root_hit_radius",
        "tip_gravity_dir",
    ]));
    rows.push(UiRow::Choice {
        label: "Shape",
        options: |_, view: &RampFormView| {
            RampShape::ALL
                .iter()
                .map(|shape| ChoiceOption {
                    label: shape.label().to_string(),
                    selected: view.ramp.shape == *shape,
                })
                .collect()
        },
        pick: |view: &mut RampFormView, index: usize| {
            if let Some(shape) = RampShape::ALL.get(index) {
                view.ramp.shape = *shape;
            }
        },
    });
    rows.push(UiRow::Choice {
        label: "Scope",
        options: |_, view: &RampFormView| {
            RampScope::ALL
                .iter()
                .map(|scope| ChoiceOption {
                    label: scope.label().to_string(),
                    selected: view.scope == *scope,
                })
                .collect()
        },
        pick: |view: &mut RampFormView, index: usize| {
            if let Some(scope) = RampScope::ALL.get(index) {
                view.scope = *scope;
            }
        },
    });
    rows.push(UiRow::Action {
        label: "Load Ramp From This Chain",
        enabled: |_, _: &RampFormView| true,
        on_click: |view: &RampFormView| {
            Box::new(LoadChainRamp {
                entity: view.entity,
                chain_id: view.chain_id,
            }) as _
        },
    });
    rows.push(UiRow::Action {
        label: "Apply Ramp",
        enabled: |_, view: &RampFormView| !view.targets.is_empty(),
        on_click: |view: &RampFormView| {
            Box::new(ApplyChainRamp {
                entity: view.entity,
                chain_ids: view.targets.clone(),
                ramp: view.ramp.clone(),
            }) as _
        },
    });
    rows
}

#[cfg(test)]
mod tests {
    use thyllore_anim_core::spring_bone::{SpringChain, SpringJointParam, SPRING_JOINT_UI_PARAMS};

    use super::*;
    use crate::animation::BoneId;
    use crate::platform::ui::ui_rows::{find_duplicate_labels, visible_labels};

    fn make_skeleton() -> Skeleton {
        let mut skeleton = Skeleton::default();
        skeleton.add_bone("Head", None);
        skeleton.add_bone("hair_l_root", Some(0));
        skeleton.add_bone("hair_r_root", Some(0));
        skeleton
    }

    fn chain(id: SpringChainId, root_bone_id: BoneId) -> SpringChain {
        SpringChain {
            id,
            joints: vec![SpringJointParam {
                bone_id: root_bone_id,
                ..SpringJointParam::default()
            }],
            ..SpringChain::default()
        }
    }

    #[test]
    fn ramp_form_rows_label_order_and_no_duplicates() {
        let rows = ramp_form_rows();
        let labels = visible_labels(&rows);
        assert_eq!(
            labels,
            [
                "root_stiffness",
                "root_drag_force",
                "root_gravity_power",
                "root_gravity_dir",
                "root_hit_radius",
                "tip_stiffness",
                "tip_drag_force",
                "tip_gravity_power",
                "tip_hit_radius",
                "Shape",
                "Scope",
                "Load Ramp From This Chain",
                "Apply Ramp",
            ]
        );
        assert!(find_duplicate_labels(&rows).is_empty());
    }

    #[test]
    fn ramp_root_block_shows_every_joint_param_and_tip_block_all_but_gravity_dir() {
        let rows = ramp_form_rows();
        let heading_at = |label: &str| {
            rows.iter()
                .position(|row| matches!(row, UiRow::Heading(heading) if *heading == label))
                .unwrap_or_else(|| panic!("ramp rows have a {label} heading"))
        };
        let shape_at = rows
            .iter()
            .position(|row| matches!(row, UiRow::Choice { label, .. } if *label == "Shape"))
            .expect("ramp rows have the Shape choice");
        let scalar_count = |rows: &[UiRow<RampFormView>]| {
            rows.iter()
                .filter(|row| matches!(row, UiRow::Scalar { .. }))
                .count()
        };
        let root_block = &rows[heading_at("Root") + 1..heading_at("Tip")];
        let tip_block = &rows[heading_at("Tip") + 1..shape_at];

        assert_eq!(scalar_count(root_block), SPRING_JOINT_UI_PARAMS.len());
        assert_eq!(scalar_count(tip_block), SPRING_JOINT_UI_PARAMS.len() - 1);
    }

    #[test]
    fn apply_ramp_is_enabled_only_with_targets() {
        let rows = ramp_form_rows();
        let Some(UiRow::Action { enabled, .. }) = rows
            .iter()
            .find(|row| matches!(row, UiRow::Action { label, .. } if *label == "Apply Ramp"))
        else {
            panic!("Apply Ramp row not found");
        };
        let world = crate::ecs::world::World::new();
        let form = |targets: Vec<SpringChainId>| RampFormView {
            entity: 0,
            chain_id: 1,
            targets,
            ramp: ChainRamp::default(),
            scope: RampScope::default(),
        };

        assert!(!enabled(&world, &form(Vec::new())));
        assert!(enabled(&world, &form(vec![1])));
    }

    #[test]
    fn resolve_ramp_targets_without_skeleton_is_the_chain_alone() {
        let setup = SpringBoneSetup {
            chains: vec![chain(1, 1), chain(2, 2)],
            ..SpringBoneSetup::default()
        };
        assert_eq!(
            resolve_ramp_targets(&setup, None, 1, RampScope::SiblingChains),
            vec![1]
        );
    }

    #[test]
    fn resolve_ramp_targets_sibling_chains_reaches_both_children_of_head() {
        let setup = SpringBoneSetup {
            chains: vec![chain(1, 1), chain(2, 2)],
            ..SpringBoneSetup::default()
        };
        let skeleton = make_skeleton();
        assert_eq!(
            resolve_ramp_targets(&setup, Some(&skeleton), 1, RampScope::SiblingChains),
            vec![1, 2]
        );
        assert_eq!(
            resolve_ramp_targets(&setup, Some(&skeleton), 1, RampScope::ThisChain),
            vec![1]
        );
    }
}
