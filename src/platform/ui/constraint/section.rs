use std::rc::Rc;
use std::sync::LazyLock;

use thyllore_scene_core::{FieldPath, ScalarParam, SceneFields};

use crate::animation::{BoneId, ConstraintType};
use crate::asset::AssetStorage;
use crate::ecs::component::{ConstraintEntry, ConstraintSet};
use crate::ecs::resource::{BakeForm, ConstraintEditorState, HierarchyState};
use crate::ecs::systems::phases::event_dispatch::constraint::ConstraintEvent;
use crate::ecs::world::{Animator, Entity, World};

use super::super::bone_choice::collect_bone_list;
use super::super::form_window::{message_section, rows_section, FormSection};
use super::super::ui_rows::{ChoiceOption, ListRows, UiRow};
use super::fields::{update_event, ConstraintFields, ConstraintView};

const CONSTRAINT_TYPE_NAMES: &[&str] = &["IK", "Aim", "Parent", "Position", "Rotation", "Scale"];

#[derive(Clone)]
pub struct ConstraintSectionView {
    pub entity: Entity,
    pub entries: Vec<ConstraintEntry>,
    pub bones: Rc<[(BoneId, String)]>,
    pub selected_bone: Option<BoneId>,
    pub selected_bone_name: Option<String>,
    pub add_type_index: usize,
    pub bake: BakeForm,
}

pub fn constraint_section(
    world: &World,
    assets: &AssetStorage,
    hierarchy_state: &HierarchyState,
) -> Box<dyn FormSection> {
    let target_entity = find_animated_entity(world);
    let Some(target_entity) = target_entity else {
        return message_section("Constraints", true, "No animated entity".to_string());
    };

    let bone_list: Rc<[(BoneId, String)]> = collect_bone_list(assets).into();

    let constraint_set = world.get_component::<ConstraintSet>(target_entity);
    let entries = match constraint_set {
        Some(set) => set.constraints.clone(),
        None => Vec::new(),
    };

    if entries.is_empty() {
        return message_section("Constraints", true, "No constraints".to_string());
    }

    let editor_state: Option<ConstraintEditorState> = world
        .get_resource::<ConstraintEditorState>()
        .map(|r| r.clone());
    let (add_type_index, bake) = match editor_state {
        Some(s) => (s.add_type_index, s.bake),
        None => (3, BakeForm::default()),
    };

    let selected_bone_name = hierarchy_state.selected_bone_id.and_then(|id| {
        bone_list
            .iter()
            .find(|(bid, _)| *bid == id)
            .map(|(_, name)| name.clone())
    });

    let view = ConstraintSectionView {
        entity: target_entity,
        entries,
        bones: bone_list,
        selected_bone: hierarchy_state.selected_bone_id,
        selected_bone_name,
        add_type_index,
        bake,
    };

    rows_section(
        vec![UiRow::Group {
            title: "Constraints".into(),
            initially_open: true,
            rows: section_rows(),
        }],
        view,
        commit_constraint_view,
    )
}

fn commit_constraint_view(
    world: &World,
    shown: &ConstraintSectionView,
    view: &ConstraintSectionView,
) {
    if view.add_type_index != shown.add_type_index || view.bake != shown.bake {
        if let Some(mut editor_state) = world.get_resource_mut::<ConstraintEditorState>() {
            editor_state.add_type_index = view.add_type_index;
            editor_state.bake = view.bake.clone();
        }
    }
}

fn find_animated_entity(world: &World) -> Option<Entity> {
    world.component_entities::<Animator>().into_iter().next()
}

pub fn section_rows() -> Vec<UiRow<ConstraintSectionView>> {
    let [sample_fps_param] = BAKE_UI_PARAMS.as_slice() else {
        unreachable!("BakeForm declares sample_fps");
    };
    vec![
        UiRow::Canvas(|ui, _, view: &ConstraintSectionView| {
            if let Some(name) = &view.selected_bone_name {
                ui.text(format!("Selected: {name}"));
            }
        }),
        UiRow::Choice {
            label: "Constraint Type",
            options: |_, view: &ConstraintSectionView| {
                CONSTRAINT_TYPE_NAMES
                    .iter()
                    .enumerate()
                    .map(|(i, name)| ChoiceOption {
                        label: name.to_string(),
                        selected: i == view.add_type_index,
                    })
                    .collect()
            },
            pick: |view: &mut ConstraintSectionView, index: usize| {
                view.add_type_index = index;
            },
        },
        UiRow::Action {
            label: "Add Constraint",
            enabled: |_, _: &ConstraintSectionView| true,
            on_click: |view: &ConstraintSectionView| {
                Box::new(ConstraintEvent::Add {
                    entity: view.entity,
                    constraint_type_index: view.add_type_index as u8,
                }) as _
            },
        },
        UiRow::List(Box::new(
            ListRows::<ConstraintSectionView, ConstraintView> {
                title: "Entries",
                items: |_, view: &ConstraintSectionView| constraint_items(view),
                rows: constraint_rows,
                on_change: |_, _, item| Box::new(update_event(&item)) as _,
                initially_open: |_| false,
            },
        )),
        UiRow::Scalar {
            param: sample_fps_param,
            scalars: &BAKE_SCALARS,
        },
        UiRow::Action {
            label: "Bake Constraints",
            enabled: |_, _: &ConstraintSectionView| true,
            on_click: |view: &ConstraintSectionView| {
                Box::new(ConstraintEvent::BakeToKeyframes {
                    entity: view.entity,
                    sample_fps: view.bake.sample_fps,
                }) as _
            },
        },
    ]
}

fn constraint_items(view: &ConstraintSectionView) -> Vec<(String, ConstraintView)> {
    view.entries
        .iter()
        .map(|entry| {
            let type_name = constraint_type_name(&entry.constraint);
            let name = format!("[{}] {} (id:{})", entry.priority, type_name, entry.id);
            let item = ConstraintView {
                entity: view.entity,
                entry_id: entry.id,
                constraint: entry.constraint.clone(),
                bones: view.bones.clone(),
                selected_bone: view.selected_bone,
                fields: ConstraintFields::from_constraint(&entry.constraint),
            };
            (name, item)
        })
        .collect()
}

fn constraint_rows(view: &ConstraintView) -> Vec<UiRow<ConstraintView>> {
    match &view.constraint {
        ConstraintType::Ik(_) => super::ik_rows(),
        ConstraintType::Aim(_) => super::aim_rows(),
        ConstraintType::Parent(_) => super::parent_rows(),
        ConstraintType::Position(_) | ConstraintType::Rotation(_) | ConstraintType::Scale(_) => {
            super::transfer_rows(view)
        }
    }
}

fn constraint_type_name(ct: &ConstraintType) -> &'static str {
    match ct {
        ConstraintType::Ik(_) => "IK",
        ConstraintType::Aim(_) => "Aim",
        ConstraintType::Parent(_) => "Parent",
        ConstraintType::Position(_) => "Position",
        ConstraintType::Rotation(_) => "Rotation",
        ConstraintType::Scale(_) => "Scale",
    }
}

struct BakePath;

impl FieldPath<ConstraintSectionView> for BakePath {
    type Field = BakeForm;

    fn get(root: &ConstraintSectionView) -> &BakeForm {
        &root.bake
    }

    fn get_mut(root: &mut ConstraintSectionView) -> &mut BakeForm {
        &mut root.bake
    }
}

static BAKE_SCALARS: LazyLock<Vec<ScalarParam<ConstraintSectionView>>> = LazyLock::new(|| {
    let mut out = Vec::new();
    BakeForm::collect_scalars::<ConstraintSectionView, BakePath>("", &mut out);
    out
});

static BAKE_UI_PARAMS: LazyLock<Vec<thyllore_scene_core::UiParam>> = LazyLock::new(|| {
    let mut out = Vec::new();
    BakeForm::collect_ui("", "", &mut out);
    out
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::ui::ui_rows::{find_duplicate_labels, visible_labels};

    #[test]
    fn section_rows_label_order() {
        let rows = section_rows();
        assert_eq!(
            visible_labels(&rows),
            [
                "Constraint Type",
                "Add Constraint",
                "Entries",
                "sample_fps",
                "Bake Constraints",
            ]
        );
    }

    #[test]
    fn section_rows_no_duplicate_labels() {
        let rows = section_rows();
        assert!(find_duplicate_labels(&rows).is_empty());
    }

    fn make_world() -> World {
        let mut world = World::new();
        world.insert_resource(ConstraintEditorState::default());
        world
    }

    fn base_view(entity: Entity) -> ConstraintSectionView {
        ConstraintSectionView {
            entity,
            entries: Vec::new(),
            bones: std::rc::Rc::new([]),
            selected_bone: None,
            selected_bone_name: None,
            add_type_index: 3,
            bake: BakeForm::default(),
        }
    }

    #[test]
    fn no_change_when_shown_equals_view() {
        let mut world = make_world();
        let entity = world.entity().with_name("test").build();
        let view = base_view(entity);
        let shown = view.clone();

        commit_constraint_view(&world, &shown, &view);

        let state = world.resource::<ConstraintEditorState>();
        assert_eq!(state.add_type_index, 3);
        assert_eq!(state.bake.sample_fps, 30.0);
    }

    #[test]
    fn bake_sample_fps_change_is_reflected() {
        let mut world = make_world();
        let entity = world.entity().with_name("test").build();
        let shown = base_view(entity);
        let mut view = shown.clone();
        view.bake.sample_fps = 60.0;

        commit_constraint_view(&world, &shown, &view);

        let state = world.resource::<ConstraintEditorState>();
        assert_eq!(state.bake.sample_fps, 60.0);
    }
}
