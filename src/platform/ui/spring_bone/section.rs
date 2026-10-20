use std::sync::LazyLock;

use thyllore_anim_core::spring_bone::{
    chains_in_scope, ColliderShape, RampShape, SpringColliderDef, SpringColliderGroup,
    CHAIN_RAMP_UI_PARAMS,
};
use thyllore_scene_core::{FieldPath, ScalarParam, SceneFields};

use crate::animation::Skeleton;
use crate::asset::AssetStorage;
use crate::ecs::component::{ChainRamp, RampScope};
use crate::ecs::component::{SpringBoneSetup, SpringChain, SpringChainId, WithSpringBone};
use crate::ecs::resource::{
    AddChainForm, AddColliderForm, HierarchyState, SpringBoneInspectorState,
};
use crate::ecs::systems::phases::event_dispatch::spring_bone::SpringBoneEvent;
use crate::ecs::world::{Entity, World};

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

struct RampPath;

impl FieldPath<SpringView> for RampPath {
    type Field = ChainRamp;

    fn get(root: &SpringView) -> &ChainRamp {
        &root.ramp
    }

    fn get_mut(root: &mut SpringView) -> &mut ChainRamp {
        &mut root.ramp
    }
}

static RAMP_SCALARS: LazyLock<Vec<ScalarParam<SpringView>>> = LazyLock::new(|| {
    let mut out = Vec::new();
    ChainRamp::collect_scalars::<SpringView, RampPath>("", &mut out);
    out
});

#[derive(Clone)]
pub struct SpringView {
    pub entity: Entity,
    pub chains: Vec<SpringChain>,
    pub ramp_targets: Vec<Vec<SpringChainId>>,
    pub colliders: Vec<SpringColliderDef>,
    pub groups: Vec<SpringColliderGroup>,
    pub bones: std::rc::Rc<[(BoneId, String)]>,
    pub selected_bone: Option<BoneId>,
    pub add_chain: AddChainForm,
    pub add_collider: AddColliderForm,
    pub add_group_name: String,
    pub ramp: ChainRamp,
    pub ramp_scope: RampScope,
    pub gizmo_visible: bool,
}

fn resolve_ramp_targets(
    setup: &SpringBoneSetup,
    skeleton: Option<&Skeleton>,
    scope: RampScope,
) -> Vec<Vec<SpringChainId>> {
    match skeleton {
        Some(skeleton) => setup
            .chains
            .iter()
            .map(|chain| chains_in_scope(setup, skeleton, chain.id, scope))
            .collect(),
        None => setup.chains.iter().map(|chain| vec![chain.id]).collect(),
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

    let gizmo_visible = world
        .get_resource::<crate::ecs::resource::gizmo::SpringBoneGizmoData>()
        .map(|g| g.visible)
        .unwrap_or(false);

    let inspector_state = world.resource::<SpringBoneInspectorState>().clone();

    let ramp_targets = resolve_ramp_targets(
        &setup,
        assets.skeletons.values().next().map(|s| &s.skeleton),
        inspector_state.ramp_scope,
    );

    let view = SpringView {
        entity: target_entity,
        chains: setup.chains,
        ramp_targets,
        colliders: setup.colliders,
        groups: setup.collider_groups,
        bones: bone_list.into(),
        selected_bone: hierarchy_state.selected_bone_id,
        add_chain: inspector_state.add_chain,
        add_collider: inspector_state.add_collider,
        add_group_name: inspector_state.add_collider_group_name,
        ramp: inspector_state.ramp,
        ramp_scope: inspector_state.ramp_scope,
        gizmo_visible,
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
        || view.ramp != shown.ramp
        || view.ramp_scope != shown.ramp_scope
    {
        if let Some(mut stored_state) = world.get_resource_mut::<SpringBoneInspectorState>() {
            stored_state.add_chain = view.add_chain.clone();
            stored_state.add_collider = view.add_collider.clone();
            stored_state.add_collider_group_name = view.add_group_name.clone();
            stored_state.ramp = view.ramp.clone();
            stored_state.ramp_scope = view.ramp_scope;
        }
    }

    if view.gizmo_visible != shown.gizmo_visible {
        world.send_command(SpringBoneEvent::ToggleGizmo(view.gizmo_visible));
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

    rows.push(UiRow::Group {
        title: "Ramp".into(),
        initially_open: false,
        rows: ramp_rows(),
    });

    rows.push(UiRow::Choice {
        label: "Root Bone",
        options: |_, view: &SpringView| {
            bone_options(&view.bones, view.selected_bone, view.add_chain.root_bone)
        },
        pick: |view: &mut SpringView, index: usize| {
            if let Some((bone_id, _)) = view.bones.get(index) {
                view.add_chain.root_bone = *bone_id;
            }
        },
    });
    rows.push(UiRow::Apply {
        label: "Use Selected Bone",
        enabled: |_, view: &SpringView| view.selected_bone.is_some(),
        apply: |view: &mut SpringView| {
            if let Some(bone_id) = view.selected_bone {
                view.add_chain.root_bone = bone_id;
            }
        },
    });
    rows.extend(ADD_CHAIN_UI_PARAMS.iter().map(|param| UiRow::Scalar {
        param,
        scalars: &ADD_CHAIN_SCALARS,
    }));
    rows.push(UiRow::Action {
        label: "Add Chain",
        enabled: |_, view: &SpringView| !view.bones.is_empty(),
        on_click: |view: &SpringView| {
            Box::new(SpringBoneEvent::ChainAdd {
                entity: view.entity,
                root_bone_id: view.add_chain.root_bone,
                chain_length: view.add_chain.chain_length,
            }) as _
        },
    });
    rows.push(collider_list_row());
    rows.push(UiRow::Choice {
        label: "Collider Bone",
        options: |_, view: &SpringView| {
            bone_options(&view.bones, view.selected_bone, view.add_collider.bone)
        },
        pick: |view: &mut SpringView, index: usize| {
            if let Some((bone_id, _)) = view.bones.get(index) {
                view.add_collider.bone = *bone_id;
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
        enabled: |_, view: &SpringView| !view.bones.is_empty(),
        on_click: |view: &SpringView| {
            let shape = if view.add_collider.capsule {
                ColliderShape::default_capsule()
            } else {
                ColliderShape::default_sphere()
            };
            Box::new(SpringBoneEvent::ColliderAdd {
                entity: view.entity,
                bone_id: view.add_collider.bone,
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
        get: |view: &SpringView| view.gizmo_visible,
        set: |view: &mut SpringView, value: bool| {
            view.gizmo_visible = value;
        },
    });
    rows
}

fn ramp_param_rows(hidden: &[&str]) -> Vec<UiRow<SpringView>> {
    CHAIN_RAMP_UI_PARAMS
        .iter()
        .filter(|param| !hidden.contains(&param.name))
        .map(|param| UiRow::Scalar {
            param,
            scalars: &RAMP_SCALARS,
        })
        .collect()
}

fn ramp_rows() -> Vec<UiRow<SpringView>> {
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
        options: |_, view: &SpringView| {
            RampShape::ALL
                .iter()
                .map(|shape| ChoiceOption {
                    label: shape.label().to_string(),
                    selected: view.ramp.shape == *shape,
                })
                .collect()
        },
        pick: |view: &mut SpringView, index: usize| {
            if let Some(shape) = RampShape::ALL.get(index) {
                view.ramp.shape = *shape;
            }
        },
    });
    rows.push(UiRow::Choice {
        label: "Scope",
        options: |_, view: &SpringView| {
            RampScope::ALL
                .iter()
                .map(|scope| ChoiceOption {
                    label: scope.label().to_string(),
                    selected: view.ramp_scope == *scope,
                })
                .collect()
        },
        pick: |view: &mut SpringView, index: usize| {
            if let Some(scope) = RampScope::ALL.get(index) {
                view.ramp_scope = *scope;
            }
        },
    });
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::{BoneId, Skeleton};
    use crate::ecs::events::UiCommandQueue;
    use thyllore_anim_core::spring_bone::SpringJointParam;

    fn make_world() -> World {
        let mut world = World::new();
        world.insert_resource(SpringBoneInspectorState::default());
        world.insert_resource(UiCommandQueue::default());
        world
    }

    fn make_skeleton() -> Skeleton {
        let mut skeleton = Skeleton::default();
        skeleton.add_bone("Head", None);
        let hair_l_root = skeleton.add_bone("hair_l_root", Some(0));
        let hair_r_root = skeleton.add_bone("hair_r_root", Some(0));
        skeleton.add_bone("hair_l_1", Some(hair_l_root));
        skeleton.add_bone("hair_r_1", Some(hair_r_root));
        skeleton
    }

    fn chain(id: SpringChainId, name: &str, root_bone_id: BoneId) -> SpringChain {
        SpringChain {
            id,
            name: name.to_string(),
            joints: vec![SpringJointParam {
                bone_id: root_bone_id,
                ..SpringJointParam::default()
            }],
            ..SpringChain::default()
        }
    }

    fn base_view(entity: Entity) -> SpringView {
        SpringView {
            entity,
            chains: Vec::new(),
            ramp_targets: Vec::new(),
            colliders: Vec::new(),
            groups: Vec::new(),
            bones: std::rc::Rc::new([]),
            selected_bone: None,
            add_chain: AddChainForm::default(),
            add_collider: AddColliderForm::default(),
            add_group_name: "NewGroup".to_string(),
            ramp: ChainRamp::default(),
            ramp_scope: RampScope::default(),
            gizmo_visible: false,
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
    fn gizmo_visible_toggle_sends_command() {
        let mut world = make_world();
        let entity = world.entity().with_name("test").build();
        let shown = base_view(entity);
        let mut view = shown.clone();
        view.gizmo_visible = true;

        commit_spring_view(&world, &shown, &view);

        let mut queue = world.get_resource_mut::<UiCommandQueue>().unwrap();
        let drained: Vec<_> = queue.drain().collect();
        assert_eq!(drained.len(), 1);
    }

    #[test]
    fn ramp_group_visible_labels_no_duplicates() {
        use crate::platform::ui::ui_rows::{find_duplicate_labels, visible_labels};

        let ramp = ramp_rows();

        assert!(
            find_duplicate_labels(&ramp).is_empty(),
            "duplicate labels: {:?}",
            find_duplicate_labels(&ramp)
        );

        let labels = visible_labels(&ramp);
        assert!(labels.contains(&"Shape"));
        assert!(labels.contains(&"Scope"));
        assert!(!labels.contains(&"tip_gravity_dir"));
    }

    #[test]
    fn ramp_root_block_shows_every_joint_param_and_tip_block_all_but_gravity_dir() {
        use thyllore_anim_core::spring_bone::SPRING_JOINT_UI_PARAMS;

        let rows = ramp_rows();
        let heading_at = |label: &str| {
            rows.iter()
                .position(|row| matches!(row, UiRow::Heading(heading) if *heading == label))
                .unwrap_or_else(|| panic!("ramp rows have a {label} heading"))
        };
        let shape_at = rows
            .iter()
            .position(|row| matches!(row, UiRow::Choice { label, .. } if *label == "Shape"))
            .expect("ramp rows have the Shape choice");
        let scalar_count = |rows: &[UiRow<SpringView>]| {
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
    fn ramp_stiffness_change_updates_state() {
        let mut world = make_world();
        let entity = world.entity().with_name("test").build();
        let shown = base_view(entity);
        let mut view = shown.clone();
        view.ramp.root.stiffness = 2.0;

        commit_spring_view(&world, &shown, &view);

        let state = world.resource::<SpringBoneInspectorState>();
        assert_eq!(state.ramp.root.stiffness, 2.0);

        let queue = world.resource::<UiCommandQueue>();
        assert!(queue.is_empty());
    }

    #[test]
    fn resolve_ramp_targets_none_skeleton_returns_each_chain_alone() {
        let setup = SpringBoneSetup {
            chains: vec![chain(1, "a", 0), chain(2, "b", 0), chain(3, "c", 0)],
            ..SpringBoneSetup::default()
        };
        let targets = resolve_ramp_targets(&setup, None, RampScope::SiblingChains);
        assert_eq!(targets, vec![vec![1], vec![2], vec![3]]);
    }

    #[test]
    fn resolve_ramp_targets_sibling_chains_returns_both_children() {
        let skeleton = make_skeleton();
        let setup = SpringBoneSetup {
            chains: vec![chain(1, "hair_l", 1), chain(2, "hair_r", 2)],
            ..SpringBoneSetup::default()
        };
        let targets = resolve_ramp_targets(&setup, Some(&skeleton), RampScope::SiblingChains);
        assert_eq!(targets[0], vec![1, 2]);
        assert_eq!(targets[1], vec![1, 2]);
    }
}
