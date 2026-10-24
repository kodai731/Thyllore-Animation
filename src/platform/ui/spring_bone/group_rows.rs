use thyllore_anim_core::spring_bone::{SpringColliderGroup, SpringColliderId};

use crate::ecs::systems::phases::event_dispatch::spring_bone::SpringBoneEvent;
use crate::ecs::world::Entity;

use super::super::ui_rows::{ChoiceOption, ListRows, UiRow};
use super::section::SpringView;

#[derive(Clone)]
pub struct GroupView {
    pub entity: Entity,
    pub group: SpringColliderGroup,
    pub available: Vec<SpringColliderId>,
}

#[derive(Clone)]
pub struct MemberView {
    pub entity: Entity,
    pub group: SpringColliderGroup,
    pub member_index: usize,
}

pub fn group_list_row() -> UiRow<SpringView> {
    UiRow::List(Box::new(ListRows::<SpringView, GroupView> {
        title: "Collider Groups",
        items: |_, view: &SpringView| {
            let all_collider_ids: Vec<SpringColliderId> =
                view.colliders.iter().map(|c| c.id).collect();
            view.groups
                .iter()
                .map(|group| {
                    let header = format!("Group: {} (id:{})", group.name, group.id);
                    let available: Vec<SpringColliderId> = all_collider_ids
                        .iter()
                        .filter(|cid| !group.collider_ids.contains(cid))
                        .copied()
                        .collect();
                    let item = GroupView {
                        entity: view.entity,
                        group: group.clone(),
                        available,
                    };
                    (header, item)
                })
                .collect()
        },
        rows: group_rows,
        on_change: |_, _, group_view| {
            Box::new(SpringBoneEvent::ColliderGroupUpdate {
                entity: group_view.entity,
                group_id: group_view.group.id,
                group: group_view.group.clone(),
            }) as _
        },
        initially_open: |_| false,
    }))
}

pub fn group_rows(view: &GroupView) -> Vec<UiRow<GroupView>> {
    let mut rows: Vec<UiRow<GroupView>> = Vec::new();

    rows.push(UiRow::Text {
        label: "Name",
        get: |view: &GroupView| &view.group.name,
        set: |view: &mut GroupView, value: String| {
            view.group.name = value;
        },
    });

    if !view.available.is_empty() {
        rows.push(UiRow::Choice {
            label: "Add Collider",
            options: |_, view: &GroupView| {
                view.available
                    .iter()
                    .map(|cid| ChoiceOption {
                        label: format!("Collider {}", cid),
                        selected: false,
                    })
                    .collect()
            },
            pick: |view: &mut GroupView, index: usize| {
                if let Some(&cid) = view.available.get(index) {
                    view.group.collider_ids.push(cid);
                }
            },
        });
    }

    rows.push(UiRow::List(Box::new(ListRows::<GroupView, MemberView> {
        title: "Members",
        items: |_, view: &GroupView| {
            view.group
                .collider_ids
                .iter()
                .enumerate()
                .map(|(i, cid)| {
                    let header = format!("Collider {}", cid);
                    let item = MemberView {
                        entity: view.entity,
                        group: view.group.clone(),
                        member_index: i,
                    };
                    (header, item)
                })
                .collect()
        },
        rows: |_| {
            vec![UiRow::Action {
                label: "Remove",
                enabled: |_, _: &MemberView| true,
                on_click: |view: &MemberView| {
                    let mut group = view.group.clone();
                    group.collider_ids.remove(view.member_index);
                    Box::new(SpringBoneEvent::ColliderGroupUpdate {
                        entity: view.entity,
                        group_id: view.group.id,
                        group,
                    }) as _
                },
            }]
        },
        on_change: |_, _, member_view| {
            Box::new(SpringBoneEvent::ColliderGroupUpdate {
                entity: member_view.entity,
                group_id: member_view.group.id,
                group: member_view.group,
            }) as _
        },
        initially_open: |_| false,
    })));

    rows.push(UiRow::Action {
        label: "Remove Group",
        enabled: |_, _: &GroupView| true,
        on_click: |view: &GroupView| {
            Box::new(SpringBoneEvent::ColliderGroupRemove {
                entity: view.entity,
                group_id: view.group.id,
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
    fn group_rows_label_order_with_available() {
        let view = GroupView {
            entity: 0,
            group: SpringColliderGroup {
                id: 1,
                name: "Test".to_string(),
                collider_ids: vec![0],
            },
            available: vec![1, 2],
        };
        let rows = group_rows(&view);
        let labels = visible_labels(&rows);
        assert_eq!(labels, ["Name", "Add Collider", "Members", "Remove Group"]);
    }

    #[test]
    fn group_rows_label_order_without_available() {
        let view = GroupView {
            entity: 0,
            group: SpringColliderGroup {
                id: 1,
                name: "Test".to_string(),
                collider_ids: vec![0],
            },
            available: Vec::new(),
        };
        let rows = group_rows(&view);
        let labels = visible_labels(&rows);
        assert_eq!(labels, ["Name", "Members", "Remove Group"]);
    }

    #[test]
    fn group_rows_no_duplicate_labels() {
        let view = GroupView {
            entity: 0,
            group: SpringColliderGroup {
                id: 1,
                name: "Test".to_string(),
                collider_ids: vec![0],
            },
            available: vec![1, 2],
        };
        let rows = group_rows(&view);
        assert!(find_duplicate_labels(&rows).is_empty());
    }
}
