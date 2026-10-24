use std::sync::LazyLock;

use thyllore_scene_core::{FieldPath, ScalarParam, SceneFields, UiParam};

use crate::animation::{BoneId, ConstraintType, ParentConstraintData};
use crate::ecs::systems::phases::event_dispatch::constraint::ConstraintEvent;

use super::super::bone_choice::bone_options;
use super::super::ui_rows::{ListRows, UiRow};
use super::fields::{applied_to_rows, common_rows, remove_row, update_event, ConstraintView};

const NEW_SOURCE: (BoneId, f32) = (0, 1.0);

#[derive(Clone)]
pub struct SourceView {
    pub view: ConstraintView,
    pub source_index: usize,
    pub bone: BoneId,
    pub fields: SourceFields,
}

#[derive(Clone, Debug, Default, PartialEq, thyllore_scene_core::SceneFields)]
#[params(tag = thyllore_scene_core::SingleOwner, owner = Component)]
pub struct SourceFields {
    #[persist(ui(min = 0.0, max = 1.0))]
    pub weight: f32,
}

struct SourceFieldsPath;

impl FieldPath<SourceView> for SourceFieldsPath {
    type Field = SourceFields;

    fn get(root: &SourceView) -> &SourceFields {
        &root.fields
    }

    fn get_mut(root: &mut SourceView) -> &mut SourceFields {
        &mut root.fields
    }
}

static SOURCE_SCALARS: LazyLock<Vec<ScalarParam<SourceView>>> = LazyLock::new(|| {
    let mut out = Vec::new();
    SourceFields::collect_scalars::<SourceView, SourceFieldsPath>("", &mut out);
    out
});

static SOURCE_UI_PARAMS: LazyLock<Vec<UiParam>> = LazyLock::new(|| {
    let mut out = Vec::new();
    SourceFields::collect_ui("", "", &mut out);
    out
});

pub fn parent_rows() -> Vec<UiRow<ConstraintView>> {
    let mut rows = common_rows();
    rows.extend(applied_to_rows());

    rows.push(UiRow::Toggle {
        label: "Affect Translation",
        get: |view: &ConstraintView| parent_data(view).is_some_and(|data| data.affect_translation),
        set: |view: &mut ConstraintView, value: bool| {
            if let ConstraintType::Parent(data) = &mut view.constraint {
                data.affect_translation = value;
            }
        },
    });
    rows.push(UiRow::Toggle {
        label: "Affect Rotation",
        get: |view: &ConstraintView| parent_data(view).is_some_and(|data| data.affect_rotation),
        set: |view: &mut ConstraintView, value: bool| {
            if let ConstraintType::Parent(data) = &mut view.constraint {
                data.affect_rotation = value;
            }
        },
    });

    rows.push(source_list_row());
    rows.push(UiRow::Apply {
        label: "Add Source",
        enabled: |_, _: &ConstraintView| true,
        apply: |view: &mut ConstraintView| {
            if let ConstraintType::Parent(data) = &mut view.constraint {
                data.sources.push(NEW_SOURCE);
            }
        },
    });
    rows.push(remove_row());
    rows
}

fn parent_data(view: &ConstraintView) -> Option<&ParentConstraintData> {
    match &view.constraint {
        ConstraintType::Parent(data) => Some(data),
        ConstraintType::Ik(_)
        | ConstraintType::Aim(_)
        | ConstraintType::Position(_)
        | ConstraintType::Rotation(_)
        | ConstraintType::Scale(_) => None,
    }
}

fn source_list_row() -> UiRow<ConstraintView> {
    UiRow::List(Box::new(ListRows::<ConstraintView, SourceView> {
        title: "Sources",
        items: |_, view: &ConstraintView| source_items(view),
        rows: |_| source_rows(),
        on_change: |view, index, source| {
            Box::new(edit_sources_event(view, |sources| {
                sources[index] = (source.bone, source.fields.weight);
            })) as _
        },
        initially_open: |_| false,
    }))
}

fn source_items(view: &ConstraintView) -> Vec<(String, SourceView)> {
    let Some(data) = parent_data(view) else {
        return Vec::new();
    };
    data.sources
        .iter()
        .enumerate()
        .map(|(source_index, &(bone, weight))| {
            let bone_name = view
                .bones
                .iter()
                .find(|(bone_id, _)| *bone_id == bone)
                .map_or("(none)", |(_, name)| name.as_str());
            let item = SourceView {
                view: view.clone(),
                source_index,
                bone,
                fields: SourceFields { weight },
            };
            (format!("Source {}: {}", source_index, bone_name), item)
        })
        .collect()
}

fn source_rows() -> Vec<UiRow<SourceView>> {
    let [weight_param] = SOURCE_UI_PARAMS.as_slice() else {
        unreachable!("SourceFields declares weight");
    };
    vec![
        UiRow::Choice {
            label: "Bone",
            options: |_, source: &SourceView| {
                bone_options(&source.view.bones, source.view.selected_bone, source.bone)
            },
            pick: |source: &mut SourceView, index: usize| {
                if let Some(&(bone_id, _)) = source.view.bones.get(index) {
                    source.bone = bone_id;
                }
            },
        },
        UiRow::Scalar {
            param: weight_param,
            scalars: &SOURCE_SCALARS,
        },
        UiRow::Action {
            label: "Remove Source",
            enabled: |_, _: &SourceView| true,
            on_click: |source: &SourceView| {
                Box::new(edit_sources_event(&source.view, |sources| {
                    sources.remove(source.source_index);
                })) as _
            },
        },
    ]
}

fn edit_sources_event(
    view: &ConstraintView,
    edit: impl FnOnce(&mut Vec<(BoneId, f32)>),
) -> ConstraintEvent {
    let mut edited = view.clone();
    if let ConstraintType::Parent(data) = &mut edited.constraint {
        edit(&mut data.sources);
    }
    update_event(&edited)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::ui::constraint::fields::ConstraintFields;
    use crate::platform::ui::ui_rows::{find_duplicate_labels, visible_labels};

    fn parent_view(sources: Vec<(BoneId, f32)>) -> ConstraintView {
        let constraint = ConstraintType::Parent(ParentConstraintData {
            sources,
            ..Default::default()
        });
        ConstraintView {
            entity: 0,
            entry_id: 1,
            fields: ConstraintFields::from_constraint(&constraint),
            constraint,
            bones: vec![(3, "hand".to_string())].into(),
            selected_bone: None,
        }
    }

    fn edited_sources(event: ConstraintEvent) -> Vec<(BoneId, f32)> {
        let ConstraintEvent::Update {
            constraint: ConstraintType::Parent(data),
            ..
        } = event
        else {
            panic!("source edits send a Parent update");
        };
        data.sources
    }

    #[test]
    fn parent_rows_label_order() {
        assert_eq!(
            visible_labels(&parent_rows()),
            [
                "Enabled",
                "weight",
                "Applied To",
                "Use Selected for Applied To",
                "Affect Translation",
                "Affect Rotation",
                "Sources",
                "Add Source",
                "Remove",
            ]
        );
    }

    #[test]
    fn parent_rows_have_no_duplicate_labels() {
        assert!(find_duplicate_labels(&parent_rows()).is_empty());
    }

    #[test]
    fn source_rows_label_order() {
        assert_eq!(
            visible_labels(&source_rows()),
            ["Bone", "weight", "Remove Source"]
        );
    }

    #[test]
    fn source_items_carry_bone_and_weight() {
        let items = source_items(&parent_view(vec![(3, 0.25)]));

        let [(name, source)] = items.as_slice() else {
            panic!("one source yields one item");
        };
        assert_eq!(name, "Source 0: hand");
        assert_eq!(source.bone, 3);
        assert_eq!(source.fields.weight, 0.25);
    }

    #[test]
    fn source_edit_replaces_only_that_source() {
        let view = parent_view(vec![(3, 0.25), (3, 0.5)]);

        let event = edit_sources_event(&view, |sources| sources[1] = (3, 0.75));

        assert_eq!(edited_sources(event), [(3, 0.25), (3, 0.75)]);
    }
}
