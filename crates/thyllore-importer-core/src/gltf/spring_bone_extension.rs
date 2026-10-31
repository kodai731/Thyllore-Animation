use serde::Deserialize;

use thyllore_anim_core::spring_bone::{
    ChainOrigin, ColliderShape, JointForce, SpringBoneSetup, SpringChain, SpringColliderDef,
    SpringColliderGroup, SpringCollisionMode, SpringForceModel, SpringJointParam, VrmJointForce,
};

#[derive(Deserialize)]
struct VrmcSpringBone {
    colliders: Option<Vec<VrmcCollider>>,
    #[serde(rename = "colliderGroups")]
    collider_groups: Option<Vec<VrmcColliderGroup>>,
    springs: Option<Vec<VrmcSpring>>,
}

#[derive(Deserialize)]
struct VrmcCollider {
    node: u32,
    shape: VrmcColliderShape,
}

#[derive(Deserialize)]
struct VrmcColliderShape {
    sphere: Option<VrmcSphere>,
    capsule: Option<VrmcCapsule>,
}

#[derive(Deserialize)]
struct VrmcSphere {
    offset: Option<[f32; 3]>,
    radius: Option<f32>,
}

#[derive(Deserialize)]
struct VrmcCapsule {
    offset: Option<[f32; 3]>,
    radius: Option<f32>,
    tail: Option<[f32; 3]>,
}

#[derive(Deserialize)]
struct VrmcColliderGroup {
    name: Option<String>,
    colliders: Option<Vec<u32>>,
}

#[derive(Deserialize)]
struct VrmcSpring {
    name: Option<String>,
    joints: Vec<VrmcJoint>,
    #[serde(rename = "colliderGroups")]
    collider_groups: Option<Vec<u32>>,
    center: Option<u32>,
}

#[derive(Deserialize)]
struct VrmcJoint {
    node: u32,
    #[serde(rename = "hitRadius")]
    hit_radius: Option<f32>,
    stiffness: Option<f32>,
    #[serde(rename = "gravityPower")]
    gravity_power: Option<f32>,
    #[serde(rename = "gravityDir")]
    gravity_dir: Option<[f32; 3]>,
    #[serde(rename = "dragForce")]
    drag_force: Option<f32>,
}

pub fn parse_vrmc_spring_bone(
    extension_json: &serde_json::Value,
    resolve_bone_id: &dyn Fn(u32) -> Option<u32>,
) -> Option<SpringBoneSetup> {
    let vrmc: VrmcSpringBone = match serde_json::from_value(extension_json.clone()) {
        Ok(v) => v,
        Err(e) => {
            log!("Failed to parse VRMC_springBone: {}", e);
            return None;
        }
    };

    let colliders = convert_colliders(vrmc.colliders.as_deref().unwrap_or(&[]), resolve_bone_id);
    let collider_groups = convert_collider_groups(vrmc.collider_groups.as_deref().unwrap_or(&[]));
    let chains = convert_springs(vrmc.springs.as_deref().unwrap_or(&[]), resolve_bone_id);

    let next_collider_id = colliders.len() as u32;
    let next_group_id = collider_groups.len() as u32;
    let next_chain_id = chains.len() as u32;

    Some(SpringBoneSetup {
        chains,
        colliders,
        collider_groups,
        next_chain_id,
        next_collider_id,
        next_group_id,
        collision_mode: SpringCollisionMode::VrmCompatible,
        new_joint_force_model: SpringForceModel::Vrm,
    })
}

fn convert_colliders(
    vrmc_colliders: &[VrmcCollider],
    resolve_bone_id: &dyn Fn(u32) -> Option<u32>,
) -> Vec<SpringColliderDef> {
    let mut result = Vec::new();

    for (i, vc) in vrmc_colliders.iter().enumerate() {
        let Some(bone_id) = resolve_bone_id(vc.node) else {
            log!(
                "VRMC collider {}: node {} not found in skeleton, skipping",
                i,
                vc.node
            );
            continue;
        };

        let shape = if let Some(ref capsule) = vc.shape.capsule {
            let offset = capsule.offset.unwrap_or([0.0; 3]);
            let tail = capsule.tail.unwrap_or([0.0; 3]);
            let radius = capsule.radius.unwrap_or(0.0);
            ColliderShape::Capsule {
                radius,
                tail: cgmath::Vector3::new(
                    tail[0] - offset[0],
                    tail[1] - offset[1],
                    tail[2] - offset[2],
                ),
            }
        } else if let Some(ref sphere) = vc.shape.sphere {
            ColliderShape::Sphere {
                radius: sphere.radius.unwrap_or(0.0),
            }
        } else {
            ColliderShape::Sphere { radius: 0.0 }
        };

        let offset_arr = vc
            .shape
            .capsule
            .as_ref()
            .and_then(|c| c.offset)
            .or_else(|| vc.shape.sphere.as_ref().and_then(|s| s.offset))
            .unwrap_or([0.0; 3]);

        result.push(SpringColliderDef {
            id: i as u32,
            bone_id,
            offset: cgmath::Vector3::new(offset_arr[0], offset_arr[1], offset_arr[2]),
            shape,
        });
    }

    result
}

fn convert_collider_groups(vrmc_groups: &[VrmcColliderGroup]) -> Vec<SpringColliderGroup> {
    vrmc_groups
        .iter()
        .enumerate()
        .map(|(i, g)| SpringColliderGroup {
            id: i as u32,
            name: g.name.clone().unwrap_or_else(|| format!("group_{}", i)),
            collider_ids: g.colliders.clone().unwrap_or_default(),
        })
        .collect()
}

fn convert_springs(
    vrmc_springs: &[VrmcSpring],
    resolve_bone_id: &dyn Fn(u32) -> Option<u32>,
) -> Vec<SpringChain> {
    let mut result = Vec::new();

    for (i, vs) in vrmc_springs.iter().enumerate() {
        let joints: Vec<SpringJointParam> = vs
            .joints
            .iter()
            .take(vs.joints.len().saturating_sub(1))
            .filter_map(|vj| {
                let bone_id = resolve_bone_id(vj.node);
                if bone_id.is_none() {
                    log!(
                        "VRMC spring '{}' joint node {} not found, skipping",
                        vs.name.as_deref().unwrap_or("?"),
                        vj.node
                    );
                }
                bone_id.map(|bid| SpringJointParam {
                    bone_id: bid,
                    force: JointForce::Vrm(VrmJointForce {
                        stiffness: vj.stiffness.unwrap_or(1.0),
                        gravity_power: vj.gravity_power.unwrap_or(0.0),
                    }),
                    drag_force: vj.drag_force.unwrap_or(0.5),
                    gravity_dir: vj
                        .gravity_dir
                        .map(|d| cgmath::Vector3::new(d[0], d[1], d[2]))
                        .unwrap_or(cgmath::Vector3::new(0.0, -1.0, 0.0)),
                    hit_radius: vj.hit_radius.unwrap_or(0.0),
                })
            })
            .collect();

        if joints.is_empty() {
            log!(
                "VRMC spring '{}': no valid joints, skipping",
                vs.name.as_deref().unwrap_or("?")
            );
            continue;
        }

        let center_bone_id = vs.center.and_then(|node| resolve_bone_id(node));
        let tail_bone_id = vs.joints.last().and_then(|tail| resolve_bone_id(tail.node));

        result.push(SpringChain {
            id: i as u32,
            name: vs.name.clone().unwrap_or_else(|| format!("spring_{}", i)),
            joints,
            collider_group_ids: vs.collider_groups.clone().unwrap_or_default(),
            center_bone_id,
            tail_bone_id,
            enabled: true,
            origin: ChainOrigin::Authored,
        });
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_three_joint_spring_chain_has_two_joints() {
        let node_to_bone: [u32; 4] = [0, 1, 2, 3];
        let resolve = |node: u32| -> Option<u32> { node_to_bone.get(node as usize).copied() };

        let springs: [VrmcSpring; 1] = [VrmcSpring {
            name: Some("test".to_string()),
            joints: vec![
                VrmcJoint {
                    node: 0,
                    hit_radius: Some(0.01),
                    stiffness: Some(0.8),
                    gravity_power: Some(0.0),
                    gravity_dir: None,
                    drag_force: Some(0.5),
                },
                VrmcJoint {
                    node: 1,
                    hit_radius: Some(0.01),
                    stiffness: Some(0.6),
                    gravity_power: Some(0.0),
                    gravity_dir: None,
                    drag_force: Some(0.5),
                },
                VrmcJoint {
                    node: 2,
                    hit_radius: Some(0.01),
                    stiffness: Some(0.4),
                    gravity_power: Some(0.0),
                    gravity_dir: None,
                    drag_force: Some(0.5),
                },
            ],
            collider_groups: None,
            center: None,
        }];

        let chains = convert_springs(&springs, &resolve);

        assert_eq!(chains.len(), 1);
        assert_eq!(chains[0].joints.len(), 2);
        assert_eq!(chains[0].joints[0].bone_id, 0);
        assert_eq!(chains[0].joints[1].bone_id, 1);
        assert_eq!(chains[0].tail_bone_id, Some(2));
    }

    /// The checked-in extension sample must come through with every value intact; a regression here
    /// changes what every VRM model's spring bones do.
    #[test]
    fn checked_in_extension_sample_keeps_every_value() {
        let json: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/data/vrmc_spring_bone.json"))
                .expect("sample parses");
        let resolve = |node: u32| -> Option<u32> { Some(node + 10) };

        let setup = parse_vrmc_spring_bone(&json, &resolve).expect("extension parses");

        assert_eq!(setup.collision_mode, SpringCollisionMode::VrmCompatible);
        assert_eq!(setup.new_joint_force_model, SpringForceModel::Vrm);
        assert_eq!(
            (
                setup.next_chain_id,
                setup.next_collider_id,
                setup.next_group_id
            ),
            (1, 2, 2)
        );

        let sphere = &setup.colliders[0];
        assert_eq!(sphere.bone_id, 11);
        assert_eq!(sphere.offset, cgmath::Vector3::new(0.0, 0.05, 0.02));
        assert_eq!(sphere.shape, ColliderShape::Sphere { radius: 0.08 });
        let capsule = &setup.colliders[1];
        assert_eq!(capsule.bone_id, 12);
        assert_eq!(
            capsule.shape,
            ColliderShape::Capsule {
                radius: 0.04,
                tail: cgmath::Vector3::new(0.0, -0.25, 0.0),
            }
        );
        let group_names: Vec<&str> = setup
            .collider_groups
            .iter()
            .map(|g| g.name.as_str())
            .collect();
        assert_eq!(group_names, ["head", "body"]);
        assert_eq!(setup.collider_groups[1].collider_ids, [1]);

        let chain = &setup.chains[0];
        assert_eq!(chain.name, "hair_front");
        assert_eq!(chain.center_bone_id, Some(10));
        assert_eq!(chain.collider_group_ids, [0, 1]);
        assert_eq!(chain.tail_bone_id, Some(15));
        let joints: Vec<(u32, JointForce, f32, f32)> = chain
            .joints
            .iter()
            .map(|j| (j.bone_id, j.force, j.drag_force, j.hit_radius))
            .collect();
        assert_eq!(
            joints,
            [
                (
                    13,
                    JointForce::Vrm(VrmJointForce {
                        stiffness: 0.8,
                        gravity_power: 0.1
                    }),
                    0.4,
                    0.01
                ),
                (
                    14,
                    JointForce::Vrm(VrmJointForce {
                        stiffness: 0.6,
                        gravity_power: 0.2
                    }),
                    0.5,
                    0.015
                ),
            ]
        );
        assert!(chain
            .joints
            .iter()
            .all(|j| j.gravity_dir == cgmath::Vector3::new(0.0, -1.0, 0.0)));
    }
}
