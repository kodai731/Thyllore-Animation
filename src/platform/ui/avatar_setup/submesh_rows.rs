use std::sync::LazyLock;

use crate::ecs::resource::AvatarSetupState;
use crate::ecs::systems::phases::event_dispatch::avatar_setup::AvatarSetupEvent;
use crate::ecs::world::World;
use crate::platform::ui::ui_rows::{ChoiceOption, ListRows, UiRow};
use thyllore_avatar_core::garment::components::chain_gen::{
    chain_role_of_submesh, chain_root_role, ChainGenRequest, ChainGenSettings, CHAIN_GEN_UI_PARAMS,
};
use thyllore_avatar_core::garment::components::submesh_role::{
    SubmeshKey, SubmeshRole, SubmeshRoleSource, SubmeshSelector,
};
use thyllore_avatar_core::humanoid::components::role::HumanoidRole;
use thyllore_avatar_core::humanoid::components::skeleton_input::BoneInput;
use thyllore_scene_core::{FieldPath, ScalarParam, SceneFields};

const SUBMESH_ROLE_CHOICES: [SubmeshRole; 4] = [
    SubmeshRole::Skin,
    SubmeshRole::Garment,
    SubmeshRole::Hair,
    SubmeshRole::Accessory,
];

#[derive(Clone)]
pub struct SubmeshesView {
    pub submeshes: Vec<SubmeshItem>,
    pub chain_gen: ChainGenSettings,
}

#[derive(Clone)]
pub struct SubmeshItem {
    pub key: SubmeshKey,
    pub role: SubmeshRole,
    pub source: SubmeshRoleSource,
    pub chain_request: Option<ChainGenRequest>,
}

struct ChainGenPath;

impl FieldPath<SubmeshesView> for ChainGenPath {
    type Field = ChainGenSettings;

    fn get(root: &SubmeshesView) -> &ChainGenSettings {
        &root.chain_gen
    }

    fn get_mut(root: &mut SubmeshesView) -> &mut ChainGenSettings {
        &mut root.chain_gen
    }
}

static CHAIN_GEN_SCALARS: LazyLock<Vec<ScalarParam<SubmeshesView>>> = LazyLock::new(|| {
    let mut out = Vec::new();
    ChainGenSettings::collect_scalars::<SubmeshesView, ChainGenPath>("", &mut out);
    out
});

impl SubmeshesView {
    pub fn from_state(state: &AvatarSetupState) -> Self {
        let submeshes: Vec<SubmeshItem> = state
            .submesh_roles
            .iter()
            .map(|(key, entry)| {
                let chain_request = resolve_chain_request(
                    entry.role,
                    &state.mapping.by_role,
                    &state.bones,
                    key,
                    &state.chain_gen,
                );
                SubmeshItem {
                    key: key.clone(),
                    role: entry.role,
                    source: entry.source,
                    chain_request,
                }
            })
            .collect();
        Self {
            submeshes,
            chain_gen: state.chain_gen.clone(),
        }
    }
}

fn resolve_chain_request(
    role: SubmeshRole,
    mapping: &std::collections::BTreeMap<HumanoidRole, usize>,
    bones: &[BoneInput],
    key: &SubmeshKey,
    settings: &ChainGenSettings,
) -> Option<ChainGenRequest> {
    let root = chain_root_role(chain_role_of_submesh(role)?);
    let bone_index = *mapping.get(&root)?;
    bones.get(bone_index)?;
    Some(ChainGenRequest {
        target: SubmeshSelector::Submesh(key.clone()),
        root,
        settings: settings.clone(),
    })
}

const CHAIN_GENERATION_HELP: &str = "Generate Chains places bone chains along a Hair or Garment \
submesh, hanging from its root bone, and skins the submesh to them. These settings apply to the \
next Generate Chains press; hover a label for its meaning.";

fn chain_generation_group() -> UiRow<SubmeshesView> {
    let mut rows: Vec<UiRow<SubmeshesView>> = vec![UiRow::Canvas(|ui, _, _| {
        ui.text_wrapped(CHAIN_GENERATION_HELP);
    })];
    rows.extend(CHAIN_GEN_UI_PARAMS.iter().map(|param| UiRow::Scalar {
        param,
        scalars: &CHAIN_GEN_SCALARS,
    }));
    UiRow::Group {
        title: "Chain Generation".into(),
        initially_open: true,
        rows,
    }
}

pub fn rows() -> Vec<UiRow<SubmeshesView>> {
    let mut out = vec![chain_generation_group()];
    out.push(UiRow::List(Box::new(ListRows::<
        SubmeshesView,
        SubmeshItem,
    > {
        title: "Submeshes",
        items: list_items,
        rows: item_rows,
        on_change: list_on_change,
        initially_open: |_| true,
    })));
    out
}

fn list_items(_world: &World, view: &SubmeshesView) -> Vec<(String, SubmeshItem)> {
    view.submeshes
        .iter()
        .map(|item| {
            let mut label = format!("{}/{}", item.key.mesh, item.key.material);
            if item.source == SubmeshRoleSource::Guessed {
                label.push_str(" (guessed)");
            }
            (label, item.clone())
        })
        .collect()
}

fn item_rows(item: &SubmeshItem) -> Vec<UiRow<SubmeshItem>> {
    let chain_role = chain_role_of_submesh(item.role);
    let mut rows: Vec<UiRow<SubmeshItem>> = vec![
        UiRow::Action {
            label: "Select in Hierarchy",
            enabled: |_world, _view| true,
            on_click: select_hierarchy,
        },
        UiRow::Choice {
            label: "Role",
            options: role_options,
            pick: role_pick,
        },
    ];

    if chain_role.is_some() {
        rows.push(UiRow::Action {
            label: "Generate Chains",
            enabled: |_world, item| item.chain_request.is_some(),
            on_click: generate_chains,
        });
    }

    rows
}

fn select_hierarchy(item: &SubmeshItem) -> Box<dyn crate::ecs::events::UiCommand> {
    Box::new(AvatarSetupEvent::SelectSubmesh {
        key: item.key.clone(),
    })
}

fn role_options(_world: &World, item: &SubmeshItem) -> Vec<ChoiceOption> {
    SUBMESH_ROLE_CHOICES
        .iter()
        .map(|role| ChoiceOption {
            label: submesh_role_label(*role).to_string(),
            selected: *role == item.role,
        })
        .collect()
}

fn role_pick(item: &mut SubmeshItem, index: usize) {
    item.role = SUBMESH_ROLE_CHOICES[index];
}

fn list_on_change(
    _view: &SubmeshesView,
    _index: usize,
    item: SubmeshItem,
) -> Box<dyn crate::ecs::events::UiCommand> {
    Box::new(AvatarSetupEvent::SetSubmeshRole {
        key: item.key,
        role: item.role,
    })
}

fn generate_chains(item: &SubmeshItem) -> Box<dyn crate::ecs::events::UiCommand> {
    let request = item
        .chain_request
        .clone()
        .expect("chain_request should be Some when Generate Chains is shown");
    Box::new(AvatarSetupEvent::GenerateChainBones { request })
}

fn submesh_role_label(role: SubmeshRole) -> &'static str {
    match role {
        SubmeshRole::Skin => "Skin",
        SubmeshRole::Garment => "Garment",
        SubmeshRole::Hair => "Hair",
        SubmeshRole::Accessory => "Accessory",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::ui::ui_rows::visible_labels;
    use thyllore_avatar_core::garment::components::submesh_role::SubmeshRoleEntry;

    #[test]
    fn test_row_labels() {
        let rows = rows();
        assert_eq!(visible_labels(&rows), ["Submeshes"]);

        let Some(UiRow::Group {
            title, rows: group, ..
        }) = rows.first()
        else {
            panic!("the chain generation settings lead the Submeshes tab");
        };
        assert_eq!(title, "Chain Generation");
        let expected: Vec<&str> = CHAIN_GEN_UI_PARAMS.iter().map(|p| p.name).collect();
        assert_eq!(visible_labels(group), expected);
    }

    #[test]
    fn test_no_duplicate_labels() {
        let rows = rows();
        let mut seen = std::collections::HashSet::new();
        for label in visible_labels(&rows) {
            assert!(seen.insert(label), "duplicate label: {}", label);
        }
    }

    fn hair_key() -> SubmeshKey {
        SubmeshKey {
            mesh: "head".to_string(),
            material: "hair".to_string(),
        }
    }

    fn hair_state_with_head_bone() -> AvatarSetupState {
        let mut state = AvatarSetupState::default();
        state.bones.push(BoneInput {
            name: "Head".to_string(),
            parent: None,
            rest_position: [0.0, 1.6, 0.0],
        });
        state.submesh_roles.insert(
            hair_key(),
            SubmeshRoleEntry {
                role: SubmeshRole::Hair,
                source: SubmeshRoleSource::User,
            },
        );
        state
    }

    #[test]
    fn test_hair_chain_request_is_resolved_when_head_is_mapped() {
        let mut state = hair_state_with_head_bone();
        state.mapping.by_role.insert(HumanoidRole::Head, 0);

        let view = SubmeshesView::from_state(&state);
        let request = view.submeshes[0]
            .chain_request
            .as_ref()
            .expect("Head is mapped");
        assert_eq!(request.target, SubmeshSelector::Submesh(hair_key()));
        assert_eq!(request.root, HumanoidRole::Head);
        assert!(request.settings == state.chain_gen);
    }

    #[test]
    fn test_hair_chain_request_is_none_when_head_is_unmapped() {
        let state = hair_state_with_head_bone();

        let view = SubmeshesView::from_state(&state);
        assert!(view.submeshes[0].chain_request.is_none());
    }
}
