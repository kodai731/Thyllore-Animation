use crate::{BoneId, Skeleton};

use super::chain_origin::ChainOrigin;
use super::collider_fit::{ensure_fitted_collider, FittedCapsule, FITTED_BODY_GROUP_NAME};
use super::space::SpringSpace;
use super::{SpringBoneSetup, SpringChain, SpringChainId, SpringJointParam};

fn build_chain_joints(
    root_bone_id: BoneId,
    chain_length: u32,
    skeleton: &Skeleton,
    template: &SpringJointParam,
) -> Vec<SpringJointParam> {
    let mut joints = Vec::new();
    let mut current_bone = root_bone_id;

    for _ in 0..chain_length {
        joints.push(SpringJointParam {
            bone_id: current_bone,
            ..template.clone()
        });

        let children: Vec<BoneId> = skeleton
            .bones
            .iter()
            .filter(|b| b.parent_id == Some(current_bone))
            .map(|b| b.id)
            .collect();

        if let Some(&first_child) = children.first() {
            current_bone = first_child;
        } else {
            break;
        }
    }

    joints
}

pub fn add_chain(
    setup: &mut SpringBoneSetup,
    root_bone_id: BoneId,
    chain_length: u32,
    skeleton: &Skeleton,
) -> SpringChainId {
    let chain_id = setup.next_chain_id;
    setup.next_chain_id += 1;

    let template = setup.new_joint_template();
    let joints = build_chain_joints(root_bone_id, chain_length, skeleton, &template);
    let chain = SpringChain {
        id: chain_id,
        name: format!("Chain_{}", chain_id),
        joints,
        collider_group_ids: Vec::new(),
        center_bone_id: None,
        tail_bone_id: None,
        enabled: true,
        origin: ChainOrigin::Authored,
    };
    setup.chains.push(chain);

    chain_id
}

pub fn add_generated_chains(
    setup: &mut SpringBoneSetup,
    chains: Vec<SpringChain>,
    root_collider: Option<FittedCapsule>,
    space: SpringSpace,
) {
    let fitted_body_group = match root_collider {
        Some(capsule) => Some(ensure_fitted_collider(setup, &capsule)),
        None => setup
            .collider_groups
            .iter()
            .find(|group| group.name == FITTED_BODY_GROUP_NAME)
            .map(|group| group.id),
    };
    let template = setup.new_joint_template();
    for mut chain in chains {
        chain.id = setup.next_chain_id;
        setup.next_chain_id += 1;
        chain.collider_group_ids.extend(fitted_body_group);
        for joint in &mut chain.joints {
            *joint = SpringJointParam {
                bone_id: joint.bone_id,
                hit_radius: joint.hit_radius * space.world_scale(),
                ..template.clone()
            };
        }
        setup.chains.push(chain);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spring_bone::{ChainOrigin, ChainRole, SpringForceModel, SubmeshSelector};

    fn make_linear_skeleton(bone_count: usize) -> Skeleton {
        let mut skeleton = Skeleton::new("test");
        let mut parent_id: Option<BoneId> = None;
        for i in 0..bone_count {
            let child = skeleton.add_bone(&format!("bone_{}", i), parent_id);
            parent_id = Some(child);
        }
        skeleton
    }

    #[test]
    fn add_chain_joints_follow_bone_hierarchy() {
        let skeleton = make_linear_skeleton(4);
        let mut setup = SpringBoneSetup::default();

        let chain_id = add_chain(&mut setup, 1, 3, &skeleton);

        assert_eq!(chain_id, 0);
        assert_eq!(setup.chains.len(), 1);
        let chain = &setup.chains[0];
        assert_eq!(chain.joints.len(), 3);
        assert_eq!(chain.joints[0].bone_id, 1);
        assert_eq!(chain.joints[1].bone_id, 2);
        assert_eq!(chain.joints[2].bone_id, 3);
    }

    #[test]
    fn add_chain_names_increment() {
        let skeleton = make_linear_skeleton(4);
        let mut setup = SpringBoneSetup::default();

        let id0 = add_chain(&mut setup, 1, 3, &skeleton);
        let id1 = add_chain(&mut setup, 1, 3, &skeleton);

        assert_eq!(id0, 0);
        assert_eq!(id1, 1);
        assert_eq!(setup.chains[0].name, "Chain_0");
        assert_eq!(setup.chains[1].name, "Chain_1");
    }

    #[test]
    fn add_chain_takes_the_setup_force_model_even_when_empty() {
        let skeleton = make_linear_skeleton(4);
        let mut setup = SpringBoneSetup {
            new_joint_force_model: SpringForceModel::Vrm,
            ..Default::default()
        };

        add_chain(&mut setup, 1, 3, &skeleton);

        assert!(setup.chains[0]
            .joints
            .iter()
            .all(|joint| joint.force.model() == SpringForceModel::Vrm));
    }

    #[test]
    fn add_generated_chains_scales_hit_radius_and_joins_fitted_group() {
        let space = SpringSpace::from_world_scale(0.01).expect("valid scale");

        let mut setup = SpringBoneSetup::default();
        let root_collider = FittedCapsule {
            bone_id: 0,
            head: cgmath::Vector3::new(0.0, 0.0, 0.0),
            tail: cgmath::Vector3::new(0.0, 1.0, 0.0),
            radius: 0.1,
        };

        let chains = vec![
            SpringChain {
                id: 99,
                name: "chain_a".to_string(),
                joints: vec![SpringJointParam {
                    bone_id: 1,
                    hit_radius: 1.0,
                    ..SpringJointParam::default()
                }],
                collider_group_ids: Vec::new(),
                center_bone_id: None,
                tail_bone_id: None,
                enabled: true,
                origin: ChainOrigin::Generated {
                    target: SubmeshSelector::Mesh("mesh".to_string()),
                    role: ChainRole::Hair,
                },
            },
            SpringChain {
                id: 98,
                name: "chain_b".to_string(),
                joints: vec![SpringJointParam {
                    bone_id: 2,
                    hit_radius: 2.0,
                    ..SpringJointParam::default()
                }],
                collider_group_ids: Vec::new(),
                center_bone_id: None,
                tail_bone_id: None,
                enabled: true,
                origin: ChainOrigin::Generated {
                    target: SubmeshSelector::Mesh("mesh".to_string()),
                    role: ChainRole::Hair,
                },
            },
        ];

        add_generated_chains(&mut setup, chains, Some(root_collider), space);

        assert_eq!(setup.chains.len(), 2);

        let fitted_group = setup
            .collider_groups
            .iter()
            .find(|g| g.name == FITTED_BODY_GROUP_NAME)
            .expect("Fitted Body group should exist");

        for (i, chain) in setup.chains.iter().enumerate() {
            assert_eq!(chain.id, i as SpringChainId);
            assert_eq!(chain.collider_group_ids, vec![fitted_group.id]);
            let expected_hit_radius = match i {
                0 => 1.0 * 0.01,
                1 => 2.0 * 0.01,
                _ => unreachable!(),
            };
            assert!(
                (chain.joints[0].hit_radius - expected_hit_radius).abs() < 1e-6,
                "chain {} hit_radius: expected {}, got {}",
                i,
                expected_hit_radius,
                chain.joints[0].hit_radius
            );
        }
    }
}
