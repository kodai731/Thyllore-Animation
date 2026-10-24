use std::sync::LazyLock;

use thyllore_anim_core::spring_bone::{ColliderShape, SpringColliderDef, SpringColliderGroup};
use thyllore_scene_core::{FieldPath, ScalarParam, SceneFields};

use crate::asset::AssetStorage;
use crate::ecs::component::{SpringBoneSetup, SpringChain, WithSpringBone};
use crate::ecs::resource::gizmo::{SpringBoneGizmoData, SpringChainGizmoMode};
use crate::ecs::resource::{
    AddChainForm, AddColliderForm, AvatarSetupState, HierarchyState, SpringBoneInspectorState,
};
use crate::ecs::systems::phases::event_dispatch::spring_bone::SpringBoneEvent;
use crate::ecs::world::{Entity, World};
use thyllore_avatar_core::humanoid::components::role::HumanoidRole;

use super::chain_rows::chain_list_row;
use super::collider_rows::collider_list_row;
use super::group_rows::group_list_row;
use crate::animation::BoneId;

use super::super::bone_choice::{bone_options, collect_bone_list};
use super::super::form_window::{message_section, rows_section, FormSection};
use super::super::ui_rows::{ChoiceOption, UiRow};

struct AddChainPath;

impl FieldPath<SpringView> for AddChainPath {
    type Field = AddChainForm;

    fn get(root: &SpringView) -> &AddChainForm {
        &root.add_chain
    }

    fn get_mut(root: &mut SpringView) -> &mut AddChainForm {
        &mut root.add_chain
    }
}

static ADD_CHAIN_SCALARS: LazyLock<Vec<ScalarParam<SpringView>>> = LazyLock::new(|| {
    let mut out = Vec::new();
    AddChainForm::collect_scalars::<SpringView, AddChainPath>("", &mut out);
    out
});

static ADD_CHAIN_UI_PARAMS: LazyLock<Vec<thyllore_scene_core::UiParam>> = LazyLock::new(|| {
    let mut out = Vec::new();
    AddChainForm::collect_ui("", "", &mut out);
    out
});

#[derive(Clone)]
pub struct SpringView {
    pub entity: Entity,
    pub chains: Vec<SpringChain>,
    pub colliders: Vec<SpringColliderDef>,
    pub groups: Vec<SpringColliderGroup>,
    pub bones: std::rc::Rc<[(BoneId, String)]>,
    pub selected_bone: Option<BoneId>,
    pub default_bone: Option<BoneId>,
    pub add_chain: AddChainForm,
    pub add_collider: AddColliderForm,
    pub add_group_name: String,
    pub collider_gizmo_visible: bool,
    pub chain_gizmo_mode: SpringChainGizmoMode,
}

impl SpringView {
    pub fn chain_root_bone(&self) -> Option<BoneId> {
        self.add_chain.root_bone.or(self.default_bone)
    }

    pub fn collider_bone(&self) -> Option<BoneId> {
        self.add_collider.bone.or(self.default_bone)
    }
}

/// The bone a new chain or collider targets before the user picks one: the selected bone, else Head.
fn default_form_bone(world: &World, hierarchy_state: &HierarchyState) -> Option<BoneId> {
    hierarchy_state.selected_bone_id.or_else(|| {
        world
            .get_resource::<AvatarSetupState>()?
            .mapping
            .by_role
            .get(&HumanoidRole::Head)
            .map(|&index| index as BoneId)
    })
}

fn bone_choice_options(view: &SpringView, current: Option<BoneId>) -> Vec<ChoiceOption> {
    match current {
        Some(current) => bone_options(&view.bones, view.selected_bone, current),
        None => view
            .bones
            .iter()
            .map(|(_, name)| ChoiceOption {
                label: name.clone(),
                selected: false,
            })
            .collect(),
    }
}

pub fn spring_bone_section(
    world: &World,
    assets: &AssetStorage,
    hierarchy_state: &HierarchyState,
) -> Box<dyn FormSection> {
    let target_entity = find_spring_bone_entity(world);
    let Some(target_entity) = target_entity else {
        return message_section("Spring Bones", true, "No spring bone entity".to_string());
    };

    let setup = match world.get_component::<SpringBoneSetup>(target_entity) {
        Some(s) => s.clone(),
        None => {
            return message_section("Spring Bones", true, "No SpringBoneSetup".to_string());
        }
    };
    let bone_list = collect_bone_list(assets);

    let (collider_gizmo_visible, chain_gizmo_mode) = world
        .get_resource::<SpringBoneGizmoData>()
        .map(|g| (g.colliders_visible, g.chain_mode))
        .unwrap_or((false, SpringChainGizmoMode::None));

    let inspector_state = world.resource::<SpringBoneInspectorState>().clone();

    let view = SpringView {
        entity: target_entity,
        chains: setup.chains,
        colliders: setup.colliders,
        groups: setup.collider_groups,
        bones: bone_list.into(),
        selected_bone: hierarchy_state.selected_bone_id,
        default_bone: default_form_bone(world, hierarchy_state),
        add_chain: inspector_state.add_chain,
        add_collider: inspector_state.add_collider,
        add_group_name: inspector_state.add_collider_group_name,
        collider_gizmo_visible,
        chain_gizmo_mode,
    };

    rows_section(
        vec![UiRow::Group {
            title: "Spring Bones".into(),
            initially_open: true,
            rows: section_rows(),
        }],
        view,
        commit_spring_view,
    )
}

fn commit_spring_view(world: &World, shown: &SpringView, view: &SpringView) {
    if view.add_chain != shown.add_chain
        || view.add_collider != shown.add_collider
        || view.add_group_name != shown.add_group_name
    {
        if let Some(mut stored_state) = world.get_resource_mut::<SpringBoneInspectorState>() {
            stored_state.add_chain = view.add_chain.clone();
            stored_state.add_collider = view.add_collider.clone();
            stored_state.add_collider_group_name = view.add_group_name.clone();
        }
    }

    if view.collider_gizmo_visible != shown.collider_gizmo_visible {
        world.send_command(SpringBoneEvent::ToggleColliderGizmo(
            view.collider_gizmo_visible,
        ));
    }
    if view.chain_gizmo_mode != shown.chain_gizmo_mode {
        world.send_command(SpringBoneEvent::SetChainGizmoMode(view.chain_gizmo_mode));
    }
}

fn find_spring_bone_entity(world: &World) -> Option<Entity> {
    world
        .iter_components::<WithSpringBone>()
        .next()
        .map(|(e, _)| e)
}

pub fn section_rows() -> Vec<UiRow<SpringView>> {
    let mut rows: Vec<UiRow<SpringView>> = Vec::new();
    rows.push(chain_list_row());

    rows.push(UiRow::Choice {
        label: "Root Bone",
        options: |_, view: &SpringView| bone_choice_options(view, view.chain_root_bone()),
        pick: |view: &mut SpringView, index: usize| {
            if let Some((bone_id, _)) = view.bones.get(index) {
                view.add_chain.root_bone = Some(*bone_id);
            }
        },
    });
    rows.push(UiRow::Apply {
        label: "Use Selected Bone",
        enabled: |_, view: &SpringView| view.selected_bone.is_some(),
        apply: |view: &mut SpringView| {
            if let Some(bone_id) = view.selected_bone {
                view.add_chain.root_bone = Some(bone_id);
            }
        },
    });
    rows.extend(ADD_CHAIN_UI_PARAMS.iter().map(|param| UiRow::Scalar {
        param,
        scalars: &ADD_CHAIN_SCALARS,
    }));
    rows.push(UiRow::Action {
        label: "Add Chain",
        enabled: |_, view: &SpringView| view.chain_root_bone().is_some(),
        on_click: |view: &SpringView| {
            Box::new(SpringBoneEvent::ChainAdd {
                entity: view.entity,
                root_bone_id: view
                    .chain_root_bone()
                    .expect("enabled only with a root bone"),
                chain_length: view.add_chain.chain_length,
            }) as _
        },
    });
    rows.push(collider_list_row());
    rows.push(UiRow::Choice {
        label: "Collider Bone",
        options: |_, view: &SpringView| bone_choice_options(view, view.collider_bone()),
        pick: |view: &mut SpringView, index: usize| {
            if let Some((bone_id, _)) = view.bones.get(index) {
                view.add_collider.bone = Some(*bone_id);
            }
        },
    });
    rows.push(UiRow::Choice {
        label: "Collider Shape",
        options: |_, view: &SpringView| {
            let sphere_selected = !view.add_collider.capsule;
            vec![
                ChoiceOption {
                    label: "Sphere".to_string(),
                    selected: sphere_selected,
                },
                ChoiceOption {
                    label: "Capsule".to_string(),
                    selected: !sphere_selected,
                },
            ]
        },
        pick: |view: &mut SpringView, index: usize| {
            view.add_collider.capsule = index == 1;
        },
    });
    rows.push(UiRow::Action {
        label: "Add Collider",
        enabled: |_, view: &SpringView| view.collider_bone().is_some(),
        on_click: |view: &SpringView| {
            let shape = if view.add_collider.capsule {
                ColliderShape::default_capsule()
            } else {
                ColliderShape::default_sphere()
            };
            Box::new(SpringBoneEvent::ColliderAdd {
                entity: view.entity,
                bone_id: view
                    .collider_bone()
                    .expect("enabled only with a collider bone"),
                shape,
            }) as _
        },
    });
    rows.push(group_list_row());
    rows.push(UiRow::Text {
        label: "Group Name",
        get: |view: &SpringView| &view.add_group_name,
        set: |view: &mut SpringView, value: String| {
            view.add_group_name = value;
        },
    });
    rows.push(UiRow::Action {
        label: "Add Group",
        enabled: |_, _: &SpringView| true,
        on_click: |view: &SpringView| {
            Box::new(SpringBoneEvent::ColliderGroupAdd {
                entity: view.entity,
                name: view.add_group_name.clone(),
            }) as _
        },
    });
    rows.push(UiRow::Toggle {
        label: "Show Collider Gizmos",
        get: |view: &SpringView| view.collider_gizmo_visible,
        set: |view: &mut SpringView, value: bool| {
            view.collider_gizmo_visible = value;
        },
    });
    rows.push(UiRow::Choice {
        label: "Chain Gizmos",
        options: |_, view: &SpringView| {
            SpringChainGizmoMode::ALL
                .iter()
                .map(|mode| ChoiceOption {
                    label: mode.label().to_string(),
                    selected: view.chain_gizmo_mode == *mode,
                })
                .collect()
        },
        pick: |view: &mut SpringView, index| {
            view.chain_gizmo_mode = SpringChainGizmoMode::ALL[index];
        },
    });
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::events::UiCommandQueue;

    fn make_world() -> World {
        let mut world = World::new();
        world.insert_resource(SpringBoneInspectorState::default());
        world.insert_resource(UiCommandQueue::default());
        world
    }

    fn base_view(entity: Entity) -> SpringView {
        SpringView {
            entity,
            chains: Vec::new(),
            colliders: Vec::new(),
            groups: Vec::new(),
            bones: std::rc::Rc::new([]),
            selected_bone: None,
            default_bone: None,
            add_chain: AddChainForm::default(),
            add_collider: AddColliderForm::default(),
            add_group_name: "NewGroup".to_string(),
            collider_gizmo_visible: false,
            chain_gizmo_mode: SpringChainGizmoMode::Selected,
        }
    }

    #[test]
    fn no_change_when_shown_equals_view() {
        let mut world = make_world();
        let entity = world.entity().with_name("test").build();
        let view = base_view(entity);
        let shown = view.clone();

        commit_spring_view(&world, &shown, &view);

        let state = world.resource::<SpringBoneInspectorState>();
        assert_eq!(state.add_chain, AddChainForm::default());
        assert_eq!(state.add_collider, AddColliderForm::default());
        assert_eq!(state.add_collider_group_name, "NewGroup");

        let queue = world.resource::<UiCommandQueue>();
        assert!(queue.is_empty());
    }

    #[test]
    fn chain_length_change_is_reflected() {
        let mut world = make_world();
        let entity = world.entity().with_name("test").build();
        let shown = base_view(entity);
        let mut view = shown.clone();
        view.add_chain.chain_length = 5;

        commit_spring_view(&world, &shown, &view);

        let state = world.resource::<SpringBoneInspectorState>();
        assert_eq!(state.add_chain.chain_length, 5);
    }

    #[test]
    fn gizmo_changes_send_one_command_each() {
        let mut world = make_world();
        let entity = world.entity().with_name("test").build();
        let shown = base_view(entity);
        let mut view = shown.clone();
        view.collider_gizmo_visible = true;
        view.chain_gizmo_mode = SpringChainGizmoMode::All;

        commit_spring_view(&world, &shown, &view);

        let mut queue = world.get_resource_mut::<UiCommandQueue>().unwrap();
        let drained: Vec<_> = queue.drain().collect();
        assert_eq!(drained.len(), 2);
    }

    fn world_with_head_mapped(head: BoneId) -> World {
        let mut world = make_world();
        let mut state = AvatarSetupState::default();
        state
            .mapping
            .by_role
            .insert(HumanoidRole::Head, head as usize);
        world.insert_resource(state);
        world
    }

    #[test]
    fn form_bones_default_to_the_selected_bone() {
        let world = world_with_head_mapped(4);
        let hierarchy = HierarchyState {
            selected_bone_id: Some(9),
            ..Default::default()
        };

        assert_eq!(default_form_bone(&world, &hierarchy), Some(9));
    }

    #[test]
    fn form_bones_fall_back_to_the_mapped_head() {
        let world = world_with_head_mapped(4);

        assert_eq!(
            default_form_bone(&world, &HierarchyState::default()),
            Some(4)
        );
    }

    #[test]
    fn form_bones_have_no_default_without_selection_or_mapping() {
        let world = make_world();

        assert_eq!(default_form_bone(&world, &HierarchyState::default()), None);
    }

    #[test]
    fn a_picked_bone_wins_over_the_default() {
        let entity = make_world().entity().with_name("test").build();
        let mut view = base_view(entity);
        view.default_bone = Some(4);
        assert_eq!(view.chain_root_bone(), Some(4));
        assert_eq!(view.collider_bone(), Some(4));

        view.add_chain.root_bone = Some(2);
        view.add_collider.bone = Some(3);
        assert_eq!(view.chain_root_bone(), Some(2));
        assert_eq!(view.collider_bone(), Some(3));
    }
}
