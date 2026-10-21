use thyllore_anim_core::spring_bone::{SpringColliderGroup, SpringColliderGroupId};

use crate::ecs::component::{RampScope, SpringChainId};
use crate::ecs::systems::phases::event_dispatch::spring_bone::{
    ApplyChainColliderGroups, LoadChainColliderGroups, SetChainColliderGroupForm,
};
use crate::ecs::world::Entity;

use super::super::ui_rows::{ChoiceOption, ListRows, UiRow};
use super::bone_section::BoneSpringView;
use super::ramp_rows::{RampOrigin, RampTargets};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupChoice {
    pub id: SpringColliderGroupId,
    pub name: String,
}

impl GroupChoice {
    pub fn from_groups(groups: &[SpringColliderGroup]) -> Vec<GroupChoice> {
        groups
            .iter()
            .map(|group| GroupChoice {
                id: group.id,
                name: group.name.clone(),
            })
            .collect()
    }
}

#[derive(Clone)]
pub struct ColliderGroupFormView {
    pub entity: Entity,
    pub origin: RampOrigin,
    /// The chain Load reads from
    pub chain_id: SpringChainId,
    pub targets: RampTargets,
    pub groups: Vec<GroupChoice>,
    pub assigned: Vec<SpringColliderGroupId>,
    pub scope: RampScope,
}

#[derive(Clone)]
pub struct GroupAssignView {
    pub group_id: SpringColliderGroupId,
    pub assigned: bool,
}

pub fn collider_group_list_row() -> UiRow<BoneSpringView> {
    UiRow::List(Box::new(
        ListRows::<BoneSpringView, ColliderGroupFormView> {
            title: "Collider Groups",
            items: |_, view: &BoneSpringView| {
                let mut items: Vec<(String, ColliderGroupFormView)> = view
                    .joints
                    .iter()
                    .zip(&view.collider_group_targets)
                    .map(|(joint, targets)| {
                        let item = ColliderGroupFormView {
                            entity: view.entity,
                            origin: RampOrigin::Joint,
                            chain_id: joint.chain_id,
                            targets: targets.clone(),
                            groups: view.collider_groups.clone(),
                            assigned: view.chain_collider_groups.clone(),
                            scope: view.chain_collider_scope,
                        };
                        (joint.chain_name.clone(), item)
                    })
                    .collect();
                if let Some((first_chain, targets)) = view
                    .child_chains
                    .as_ref()
                    .and_then(|targets| Some((*targets.chains.first()?, targets)))
                {
                    items.push((
                        "Chains under this bone".to_string(),
                        ColliderGroupFormView {
                            entity: view.entity,
                            origin: RampOrigin::ChildChains,
                            chain_id: first_chain,
                            targets: targets.clone(),
                            groups: view.collider_groups.clone(),
                            assigned: view.chain_collider_groups.clone(),
                            scope: view.chain_collider_scope,
                        },
                    ));
                }
                items
            },
            rows: |form| collider_group_form_rows(form.origin),
            on_change: |_, _, form| {
                Box::new(SetChainColliderGroupForm {
                    group_ids: form.assigned,
                    scope: form.scope,
                })
            },
            initially_open: |_| true,
        },
    ))
}

fn toggled(
    assigned: &[SpringColliderGroupId],
    item: &GroupAssignView,
) -> Vec<SpringColliderGroupId> {
    let mut out: Vec<SpringColliderGroupId> = assigned
        .iter()
        .copied()
        .filter(|id| *id != item.group_id)
        .collect();
    if item.assigned {
        out.push(item.group_id);
    }
    out
}

fn group_assign_list_row() -> UiRow<ColliderGroupFormView> {
    UiRow::List(Box::new(
        ListRows::<ColliderGroupFormView, GroupAssignView> {
            title: "Groups",
            items: |_, form: &ColliderGroupFormView| {
                form.groups
                    .iter()
                    .map(|group| {
                        let item = GroupAssignView {
                            group_id: group.id,
                            assigned: form.assigned.contains(&group.id),
                        };
                        (group.name.clone(), item)
                    })
                    .collect()
            },
            rows: |_| {
                vec![UiRow::Toggle {
                    label: "Assigned",
                    get: |item: &GroupAssignView| item.assigned,
                    set: |item: &mut GroupAssignView, value| item.assigned = value,
                }]
            },
            on_change: |form, _, item| {
                Box::new(SetChainColliderGroupForm {
                    group_ids: toggled(&form.assigned, &item),
                    scope: form.scope,
                })
            },
            initially_open: |_| true,
        },
    ))
}

pub fn collider_group_form_rows(origin: RampOrigin) -> Vec<UiRow<ColliderGroupFormView>> {
    let mut rows = vec![group_assign_list_row()];
    match origin {
        RampOrigin::Joint => rows.push(UiRow::Choice {
            label: "Scope",
            options: |_, view: &ColliderGroupFormView| {
                RampScope::ALL
                    .iter()
                    .map(|scope| ChoiceOption {
                        label: scope.label().to_string(),
                        selected: view.scope == *scope,
                    })
                    .collect()
            },
            pick: |view: &mut ColliderGroupFormView, index: usize| {
                if let Some(scope) = RampScope::ALL.get(index) {
                    view.scope = *scope;
                }
            },
        }),
        RampOrigin::ChildChains => {}
    }
    rows.push(UiRow::Canvas(|ui, _, view: &ColliderGroupFormView| {
        ui.text(format!(
            "Applies to {} chains, {} bones",
            view.targets.chains.len(),
            view.targets.joint_count
        ));
    }));
    rows.push(UiRow::Action {
        label: "Load Groups From This Chain",
        enabled: |_, _: &ColliderGroupFormView| true,
        on_click: |view: &ColliderGroupFormView| {
            Box::new(LoadChainColliderGroups {
                entity: view.entity,
                chain_id: view.chain_id,
            }) as _
        },
    });
    rows.push(UiRow::Action {
        label: "Apply Collider Groups",
        enabled: |_, view: &ColliderGroupFormView| !view.targets.chains.is_empty(),
        on_click: |view: &ColliderGroupFormView| {
            Box::new(ApplyChainColliderGroups {
                entity: view.entity,
                chain_ids: view.targets.chains.clone(),
                group_ids: view.assigned.clone(),
            }) as _
        },
    });
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::ui::ui_rows::{find_duplicate_labels, visible_labels};

    #[test]
    fn form_rows_label_order_and_no_duplicates() {
        let rows = collider_group_form_rows(RampOrigin::Joint);
        assert_eq!(
            visible_labels(&rows),
            [
                "Groups",
                "Scope",
                "Load Groups From This Chain",
                "Apply Collider Groups",
            ]
        );
        assert!(find_duplicate_labels(&rows).is_empty());
    }

    #[test]
    fn child_chain_rows_have_no_scope_choice() {
        let rows = collider_group_form_rows(RampOrigin::ChildChains);
        let labels = visible_labels(&rows);
        assert!(!labels.contains(&"Scope"));
        assert!(labels.contains(&"Apply Collider Groups"));
    }

    #[test]
    fn toggled_adds_once_and_removes() {
        let on = GroupAssignView {
            group_id: 7,
            assigned: true,
        };
        let off = GroupAssignView {
            group_id: 7,
            assigned: false,
        };
        assert_eq!(toggled(&[3], &on), vec![3, 7]);
        assert_eq!(toggled(&[3, 7], &on), vec![3, 7]);
        assert_eq!(toggled(&[3, 7], &off), vec![3]);
    }

    #[test]
    fn group_choices_keep_declaration_order() {
        let groups = vec![
            SpringColliderGroup {
                id: 2,
                name: "Fitted Body".to_string(),
                collider_ids: Vec::new(),
            },
            SpringColliderGroup {
                id: 1,
                name: "Hands".to_string(),
                collider_ids: Vec::new(),
            },
        ];
        let choices = GroupChoice::from_groups(&groups);
        assert_eq!(choices[0].id, 2);
        assert_eq!(choices[1].name, "Hands");
    }
}
