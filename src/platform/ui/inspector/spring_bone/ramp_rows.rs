use std::sync::LazyLock;

use thyllore_anim_core::spring_bone::{
    JointForceForm, RampShape, RampTargets, SpringJointParam, CHAIN_RAMP_UI_PARAMS,
};
use thyllore_scene_core::{FieldPath, ScalarParam, SceneFields};

use crate::ecs::component::{ChainRamp, RampScope, SpringChainId};
use crate::ecs::systems::phases::event_dispatch::spring_bone::{
    ApplyChainRamp, LoadChainRamp, SetChainRampForm,
};
use crate::ecs::world::Entity;

use super::bone_section::BoneSpringView;
use super::force_rows::force_field_row;
use crate::platform::ui::ui_rows::{ChoiceOption, ListRows, UiRow};

/// Where a ramp item starts: a joint of the selected bone (scope widens it) or the bone above chain roots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RampOrigin {
    Joint,
    ChildChains,
}

#[derive(Clone)]
pub struct RampFormView {
    pub entity: Entity,
    pub origin: RampOrigin,
    /// The chain Load reads from
    pub chain_id: SpringChainId,
    pub targets: RampTargets,
    pub ramp: ChainRamp,
    pub root_force: JointForceForm,
    pub tip_force: JointForceForm,
    pub scope: RampScope,
}

impl RampFormView {
    pub fn new(
        entity: Entity,
        origin: RampOrigin,
        chain_id: SpringChainId,
        targets: RampTargets,
        ramp: ChainRamp,
        scope: RampScope,
    ) -> Self {
        let root_force = JointForceForm::from(ramp.root.force);
        let tip_force = JointForceForm::from(ramp.tip.force);
        Self {
            entity,
            origin,
            chain_id,
            targets,
            ramp,
            root_force,
            tip_force,
            scope,
        }
    }

    /// The ramp with the forces the two forms currently select.
    pub fn ramp_param(&self) -> ChainRamp {
        ChainRamp {
            root: SpringJointParam {
                force: self.root_force.into(),
                ..self.ramp.root.clone()
            },
            tip: SpringJointParam {
                force: self.tip_force.into(),
                ..self.ramp.tip.clone()
            },
            shape: self.ramp.shape,
        }
    }
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

/// One ramp form per joint of the bone, plus one for the chains hanging under it.
fn ramp_items(_: &crate::ecs::World, view: &BoneSpringView) -> Vec<(String, RampFormView)> {
    let form_for = |origin: RampOrigin, chain_id: SpringChainId, targets: &RampTargets| {
        RampFormView::new(
            view.entity,
            origin,
            chain_id,
            targets.clone(),
            view.ramp.clone(),
            view.ramp_scope,
        )
    };
    let mut items: Vec<(String, RampFormView)> = view
        .joints
        .iter()
        .zip(&view.ramp_targets)
        .map(|(joint, targets)| {
            (
                joint.chain_name.clone(),
                form_for(RampOrigin::Joint, joint.chain_id, targets),
            )
        })
        .collect();
    if let Some((first_chain, targets)) = view.child_chains_with_first_chain() {
        items.push((
            "Chains under this bone".to_string(),
            form_for(RampOrigin::ChildChains, first_chain, targets),
        ));
    }
    items
}

pub fn ramp_list_row() -> UiRow<BoneSpringView> {
    UiRow::List(Box::new(ListRows::<BoneSpringView, RampFormView> {
        title: "Ramp",
        items: ramp_items,
        rows: ramp_form_rows,
        on_change: |_, _, form| {
            Box::new(SetChainRampForm {
                ramp: form.ramp_param(),
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

/// The force form of one ramp end followed by the end's scalar params.
fn ramp_end_rows(
    heading: &'static str,
    force: &JointForceForm,
    force_field: fn(&mut RampFormView) -> &mut JointForceForm,
    hidden: &[&str],
) -> Vec<UiRow<RampFormView>> {
    let mut rows = vec![UiRow::Heading(heading), force_field_row(force, force_field)];
    rows.extend(ramp_param_rows(hidden));
    rows
}

fn shape_choice_row() -> UiRow<RampFormView> {
    UiRow::Choice {
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
    }
}

fn scope_choice_row() -> UiRow<RampFormView> {
    UiRow::Choice {
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
    }
}

fn ramp_action_rows() -> Vec<UiRow<RampFormView>> {
    vec![
        UiRow::Canvas(|ui, _, view: &RampFormView| {
            ui.text(format!(
                "Applies to {} chains, {} bones",
                view.targets.chains.len(),
                view.targets.joint_count
            ));
        }),
        UiRow::Action {
            label: "Load Ramp From This Chain",
            enabled: |_, _: &RampFormView| true,
            on_click: |view: &RampFormView| {
                Box::new(LoadChainRamp {
                    entity: view.entity,
                    chain_id: view.chain_id,
                }) as _
            },
        },
        UiRow::Action {
            label: "Apply Ramp",
            enabled: |_, view: &RampFormView| !view.targets.chains.is_empty(),
            on_click: |view: &RampFormView| {
                Box::new(ApplyChainRamp {
                    entity: view.entity,
                    chain_ids: view.targets.chains.clone(),
                    ramp: view.ramp_param(),
                }) as _
            },
        },
    ]
}

pub fn ramp_form_rows(form: &RampFormView) -> Vec<UiRow<RampFormView>> {
    let mut rows = ramp_end_rows(
        "Root",
        &form.root_force,
        |view| &mut view.root_force,
        &["tip_drag_force", "tip_gravity_dir", "tip_hit_radius"],
    );
    rows.extend(ramp_end_rows(
        "Tip",
        &form.tip_force,
        |view| &mut view.tip_force,
        &[
            "root_drag_force",
            "root_gravity_dir",
            "root_hit_radius",
            "tip_gravity_dir",
        ],
    ));
    rows.push(shape_choice_row());
    if form.origin == RampOrigin::Joint {
        rows.push(scope_choice_row());
    }
    rows.extend(ramp_action_rows());
    rows
}

#[cfg(test)]
mod tests {
    use thyllore_anim_core::spring_bone::SPRING_JOINT_UI_PARAMS;

    use super::*;
    use crate::platform::ui::ui_rows::{find_duplicate_labels, visible_labels};

    fn form(origin: RampOrigin, chains: Vec<SpringChainId>) -> RampFormView {
        RampFormView::new(
            0,
            origin,
            0,
            RampTargets {
                joint_count: chains.len(),
                chains,
            },
            ChainRamp::default(),
            RampScope::ThisChain,
        )
    }

    #[test]
    fn ramp_form_rows_label_order_and_no_duplicates() {
        let rows = ramp_form_rows(&form(RampOrigin::Joint, Vec::new()));
        let labels = visible_labels(&rows);
        assert_eq!(
            labels,
            [
                "root_drag_force",
                "root_gravity_dir",
                "root_hit_radius",
                "tip_drag_force",
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
    fn ramp_blocks_start_with_a_force_form() {
        let rows = ramp_form_rows(&form(RampOrigin::Joint, Vec::new()));
        let field_count = rows
            .iter()
            .filter(|row| matches!(row, UiRow::Field(_)))
            .count();
        assert_eq!(field_count, 2);
    }

    #[test]
    fn child_chain_rows_have_no_scope_choice() {
        let rows = ramp_form_rows(&form(RampOrigin::ChildChains, Vec::new()));
        let labels = visible_labels(&rows);
        assert!(!labels.contains(&"Scope"));
        assert!(labels.contains(&"Apply Ramp"));
    }

    #[test]
    fn ramp_root_block_shows_every_joint_param_and_tip_block_all_but_gravity_dir() {
        let rows = ramp_form_rows(&form(RampOrigin::Joint, Vec::new()));
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
        let rows = ramp_form_rows(&form(RampOrigin::Joint, Vec::new()));
        let Some(UiRow::Action { enabled, .. }) = rows
            .iter()
            .find(|row| matches!(row, UiRow::Action { label, .. } if *label == "Apply Ramp"))
        else {
            panic!("Apply Ramp row not found");
        };
        let world = crate::ecs::world::World::new();

        assert!(!enabled(&world, &form(RampOrigin::Joint, Vec::new())));
        assert!(enabled(&world, &form(RampOrigin::Joint, vec![1])));
    }
}
