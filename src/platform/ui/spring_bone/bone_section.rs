use std::rc::Rc;

use thyllore_anim_core::spring_bone::{
    ColliderShape, SpringChainId, SpringColliderDef, SpringJointParam,
};

use crate::animation::BoneId;
use crate::asset::AssetStorage;
use crate::ecs::component::{ChainRamp, RampEnd, RampScope, SpringBoneSetup, WithSpringBone};
use crate::ecs::resource::SpringBoneInspectorState;
use crate::ecs::systems::phases::event_dispatch::spring_bone::{SetChainRampEnd, SpringBoneEvent};
use crate::ecs::world::{Entity, World};

use super::super::bone_choice::collect_bone_list;
use super::super::form_window::{message_section, rows_section, FormSection};
use super::super::ui_rows::{ListRows, UiRow};
use super::chain_rows::{joint_param_rows, JointView};
use super::collider_rows::{collider_rows, ColliderFields, ColliderView};
use super::ramp_rows::{ramp_list_row, resolve_ramp_targets};

#[derive(Clone, Debug)]
pub struct BoneJoint {
    pub chain_id: SpringChainId,
    pub chain_name: String,
    pub joint_index: usize,
    pub joint: SpringJointParam,
}

#[derive(Clone)]
pub struct BoneSpringView {
    pub entity: Entity,
    pub bone_id: BoneId,
    pub joints: Vec<BoneJoint>,
    /// Per joint: the chains a ramp applied from that joint's chain reaches under the scope
    pub ramp_targets: Vec<Vec<SpringChainId>>,
    pub ramp: ChainRamp,
    pub ramp_scope: RampScope,
    pub colliders: Vec<SpringColliderDef>,
    pub bones: Rc<[(BoneId, String)]>,
    pub chain_length: u32,
}

pub fn bone_spring_section(
    world: &World,
    assets: &AssetStorage,
    bone_id: BoneId,
) -> Box<dyn FormSection> {
    let Some(entity) = world
        .iter_components::<WithSpringBone>()
        .next()
        .map(|(entity, _)| entity)
    else {
        return message_section("Spring", true, "No spring bone entity".to_string());
    };
    let Some(setup) = world.get_component::<SpringBoneSetup>(entity) else {
        return message_section("Spring", true, "No SpringBoneSetup".to_string());
    };
    let inspector_state = world.resource::<SpringBoneInspectorState>().clone();
    let skeleton = assets.skeletons.values().next().map(|s| &s.skeleton);
    let joints = joints_on_bone(&setup, bone_id);
    let ramp_targets = joints
        .iter()
        .map(|joint| {
            resolve_ramp_targets(&setup, skeleton, joint.chain_id, inspector_state.ramp_scope)
        })
        .collect();

    let view = BoneSpringView {
        entity,
        bone_id,
        joints,
        ramp_targets,
        ramp: inspector_state.ramp,
        ramp_scope: inspector_state.ramp_scope,
        colliders: colliders_on_bone(&setup, bone_id),
        bones: collect_bone_list(assets).into(),
        chain_length: inspector_state.add_chain.chain_length,
    };

    rows_section(
        vec![UiRow::Group {
            title: "Spring".into(),
            initially_open: true,
            rows: bone_spring_rows(&view),
        }],
        view,
        |_, _, _| {},
    )
}

pub fn joints_on_bone(setup: &SpringBoneSetup, bone_id: BoneId) -> Vec<BoneJoint> {
    setup
        .chains
        .iter()
        .flat_map(|chain| {
            chain
                .joints
                .iter()
                .enumerate()
                .filter(move |(_, joint)| joint.bone_id == bone_id)
                .map(move |(joint_index, joint)| BoneJoint {
                    chain_id: chain.id,
                    chain_name: chain.name.clone(),
                    joint_index,
                    joint: joint.clone(),
                })
        })
        .collect()
}

fn colliders_on_bone(setup: &SpringBoneSetup, bone_id: BoneId) -> Vec<SpringColliderDef> {
    setup
        .colliders
        .iter()
        .filter(|collider| collider.bone_id == bone_id)
        .cloned()
        .collect()
}

pub fn bone_spring_rows(view: &BoneSpringView) -> Vec<UiRow<BoneSpringView>> {
    let mut rows = Vec::new();

    if view.joints.is_empty() {
        rows.push(UiRow::Action {
            label: "Add Chain From This Bone",
            enabled: |_, _| true,
            on_click: |view: &BoneSpringView| {
                Box::new(SpringBoneEvent::ChainAdd {
                    entity: view.entity,
                    root_bone_id: view.bone_id,
                    chain_length: view.chain_length,
                })
            },
        });
        rows.push(UiRow::Tooltip(
            "Chain length comes from the Add Chain form in the entity's Spring Bones section",
        ));
    } else {
        rows.push(joint_list_row());
        rows.push(ramp_list_row());
    }

    if view.colliders.is_empty() {
        rows.push(UiRow::Action {
            label: "Add Collider On This Bone",
            enabled: |_, _| true,
            on_click: |view: &BoneSpringView| {
                Box::new(SpringBoneEvent::ColliderAdd {
                    entity: view.entity,
                    bone_id: view.bone_id,
                    shape: ColliderShape::default_sphere(),
                })
            },
        });
    } else {
        rows.push(collider_list_row());
    }

    rows
}

fn bone_joint_rows() -> Vec<UiRow<JointView>> {
    let mut rows = joint_param_rows();
    rows.push(UiRow::Action {
        label: "Use As Ramp Root",
        enabled: |_, _: &JointView| true,
        on_click: |view: &JointView| {
            Box::new(SetChainRampEnd {
                end: RampEnd::Root,
                joint: view.joint.clone(),
            }) as _
        },
    });
    rows.push(UiRow::Action {
        label: "Use As Ramp Tip",
        enabled: |_, _: &JointView| true,
        on_click: |view: &JointView| {
            Box::new(SetChainRampEnd {
                end: RampEnd::Tip,
                joint: view.joint.clone(),
            }) as _
        },
    });
    rows
}

fn joint_list_row() -> UiRow<BoneSpringView> {
    UiRow::List(Box::new(ListRows::<BoneSpringView, JointView> {
        title: "Joints",
        items: |_, view: &BoneSpringView| {
            view.joints
                .iter()
                .map(|joint| {
                    let header = format!("{} / Joint {}", joint.chain_name, joint.joint_index);
                    let item = JointView {
                        bones: view.bones.clone(),
                        selected_bone: Some(view.bone_id),
                        joint: joint.joint.clone(),
                    };
                    (header, item)
                })
                .collect()
        },
        rows: |_| bone_joint_rows(),
        on_change: |view, index, joint_view| {
            let source = &view.joints[index];
            Box::new(SpringBoneEvent::JointUpdate {
                entity: view.entity,
                chain_id: source.chain_id,
                joint_index: source.joint_index,
                joint: joint_view.joint,
            })
        },
        initially_open: |_| true,
    }))
}

fn collider_list_row() -> UiRow<BoneSpringView> {
    UiRow::List(Box::new(ListRows::<BoneSpringView, ColliderView> {
        title: "Colliders",
        items: |_, view: &BoneSpringView| {
            view.colliders
                .iter()
                .map(|collider| {
                    let shape_name = match &collider.shape {
                        ColliderShape::Sphere { .. } => "Sphere",
                        ColliderShape::Capsule { .. } => "Capsule",
                    };
                    let header = format!("{} (id:{})", shape_name, collider.id);
                    let item = ColliderView {
                        entity: view.entity,
                        bones: view.bones.clone(),
                        selected_bone: Some(view.bone_id),
                        collider: collider.clone(),
                        fields: ColliderFields::from_collider(collider),
                    };
                    (header, item)
                })
                .collect()
        },
        rows: collider_rows,
        on_change: |_, _, collider_view| {
            let mut collider = collider_view.collider.clone();
            collider_view.fields.apply_to(&mut collider);
            Box::new(SpringBoneEvent::ColliderUpdate {
                entity: collider_view.entity,
                collider_id: collider.id,
                collider,
            })
        },
        initially_open: |_| true,
    }))
}

#[cfg(test)]
mod tests {
    use thyllore_anim_core::spring_bone::SpringChain;

    use super::*;
    use crate::platform::ui::ui_rows::{find_duplicate_labels, visible_labels};

    fn chain(id: SpringChainId, name: &str, bones: &[BoneId]) -> SpringChain {
        SpringChain {
            id,
            name: name.to_string(),
            joints: bones
                .iter()
                .map(|&bone_id| SpringJointParam {
                    bone_id,
                    ..SpringJointParam::default()
                })
                .collect(),
            ..SpringChain::default()
        }
    }

    fn setup_with_two_chains() -> SpringBoneSetup {
        SpringBoneSetup {
            chains: vec![
                chain(1, "hair_l", &[10, 11, 12]),
                chain(2, "hair_r", &[20, 11]),
            ],
            ..SpringBoneSetup::default()
        }
    }

    fn view(joints: Vec<BoneJoint>, colliders: Vec<SpringColliderDef>) -> BoneSpringView {
        let ramp_targets = joints.iter().map(|joint| vec![joint.chain_id]).collect();
        BoneSpringView {
            entity: 0,
            bone_id: 11,
            joints,
            ramp_targets,
            ramp: ChainRamp::default(),
            ramp_scope: RampScope::default(),
            colliders,
            bones: Rc::new([]),
            chain_length: 3,
        }
    }

    #[test]
    fn joints_on_bone_finds_every_chain_holding_the_bone() {
        let joints = joints_on_bone(&setup_with_two_chains(), 11);
        let found: Vec<(SpringChainId, usize)> = joints
            .iter()
            .map(|joint| (joint.chain_id, joint.joint_index))
            .collect();
        assert_eq!(found, [(1, 1), (2, 1)]);
        assert_eq!(joints[0].chain_name, "hair_l");
    }

    #[test]
    fn joints_on_bone_is_empty_for_a_bone_outside_every_chain() {
        assert!(joints_on_bone(&setup_with_two_chains(), 99).is_empty());
    }

    #[test]
    fn bone_without_spring_data_offers_add_actions() {
        let rows = bone_spring_rows(&view(Vec::new(), Vec::new()));
        assert_eq!(
            visible_labels(&rows),
            ["Add Chain From This Bone", "Add Collider On This Bone"]
        );
        assert!(find_duplicate_labels(&rows).is_empty());
    }

    #[test]
    fn bone_with_joints_lists_them_instead_of_the_add_action() {
        let joints = joints_on_bone(&setup_with_two_chains(), 11);
        let rows = bone_spring_rows(&view(joints, Vec::new()));
        assert_eq!(
            visible_labels(&rows),
            ["Joints", "Ramp", "Add Collider On This Bone"]
        );
    }

    #[test]
    fn bone_joint_rows_label_order() {
        let rows = bone_joint_rows();
        let labels = visible_labels(&rows);
        assert_eq!(
            labels,
            [
                "stiffness",
                "drag_force",
                "gravity_power",
                "gravity_dir",
                "hit_radius",
                "Use As Ramp Root",
                "Use As Ramp Tip",
            ]
        );
        assert!(find_duplicate_labels(&rows).is_empty());
    }
}
