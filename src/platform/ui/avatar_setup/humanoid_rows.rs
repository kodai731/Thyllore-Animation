use crate::ecs::events::UiCommand;
use crate::ecs::resource::AvatarSetupState;
use crate::ecs::systems::phases::event_dispatch::avatar_setup::AvatarSetupEvent;
use crate::ecs::world::World;
use crate::platform::ui::ui_rows::{ChoiceOption, NamedChoice, UiRow};
use thyllore_avatar_core::humanoid::components::mapping::UnresolvedRole;
use thyllore_avatar_core::humanoid::components::role::{HumanoidRole, REQUIRED};

const NO_BONE_LABEL: &str = "(none)";

#[derive(Clone)]
pub struct HumanoidView {
    pub bones: Vec<(usize, String)>,
    pub mapping: std::collections::BTreeMap<HumanoidRole, usize>,
    pub unresolved: Vec<UnresolvedRole>,
}

impl HumanoidView {
    pub fn from_state(state: &AvatarSetupState) -> Self {
        Self {
            bones: state
                .bones
                .iter()
                .enumerate()
                .map(|(i, b)| (i, b.name.clone()))
                .collect(),
            mapping: state.mapping.by_role.clone(),
            unresolved: state.unresolved.clone(),
        }
    }
}

pub fn rows() -> Vec<UiRow<HumanoidView>> {
    vec![
        UiRow::Action {
            label: "Save mapping",
            enabled: |_world, _view| true,
            on_click: |_view| Box::new(AvatarSetupEvent::SaveHumanoidMapping),
        },
        UiRow::Choices {
            items: make_choices,
            on_pick: pick_role,
        },
    ]
}

fn make_choices(_world: &World, view: &HumanoidView) -> Vec<NamedChoice> {
    HumanoidRole::ALL
        .iter()
        .enumerate()
        .map(|(key, role)| {
            let required_marker = if REQUIRED.contains(role) { "*" } else { "" };
            let label = format!("{}{}", role.unity_name(), required_marker);
            let emphasized = view.unresolved.iter().any(|u| u.role == *role);
            let current_bone = view.mapping.get(role).copied();
            let options: Vec<ChoiceOption> = std::iter::once(ChoiceOption {
                label: NO_BONE_LABEL.to_string(),
                selected: current_bone.is_none(),
            })
            .chain(view.bones.iter().map(|(bone_index, name)| ChoiceOption {
                label: name.clone(),
                selected: current_bone == Some(*bone_index),
            }))
            .collect();
            NamedChoice {
                label,
                key,
                emphasized,
                options,
            }
        })
        .collect()
}

fn pick_role(view: &HumanoidView, choice: &NamedChoice, index: usize) -> Box<dyn UiCommand> {
    let role = HumanoidRole::ALL[choice.key];
    let bone = if index == 0 {
        None
    } else {
        Some(view.bones[index - 1].0)
    };
    Box::new(AvatarSetupEvent::SetHumanoidRole { role, bone })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::ui::ui_rows::visible_labels;

    #[test]
    fn test_row_labels() {
        let rows = rows();
        let labels = visible_labels(&rows);
        assert_eq!(labels, ["Save mapping"]);
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
