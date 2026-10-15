use serde::{Deserialize, Serialize};

use crate::humanoid::components::skeleton_input::BoneInput;
use thyllore_anim_core::spring_bone::{
    ColliderShape, SpringBoneSetup, SpringChain, SpringColliderId, SpringCollisionMode,
    FITTED_BODY_GROUP_NAME,
};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SidecarCollisionMode {
    #[default]
    VrmCompatible,
    SegmentExtended,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SidecarColliderShape {
    Sphere,
    Capsule,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SidecarColliderOrigin {
    Authored,
    Fitted,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SidecarSpring {
    pub collision_mode: SidecarCollisionMode,
    pub colliders: Vec<SidecarCollider>,
    pub collider_groups: Vec<SidecarColliderGroup>,
    pub chains: Vec<SidecarSpringChain>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SidecarCollider {
    pub name: String,
    pub bone: String,
    pub offset: [f32; 3],
    pub shape: SidecarColliderShape,
    pub radius: f32,
    pub tail: Option<[f32; 3]>,
    pub origin: SidecarColliderOrigin,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SidecarColliderGroup {
    pub name: String,
    pub colliders: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SidecarSpringChain {
    pub name: String,
    pub center: Option<String>,
    pub collider_groups: Vec<String>,
    pub joints: Vec<SidecarSpringJoint>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SidecarSpringJoint {
    pub bone: String,
    pub stiffness: f32,
    pub drag_force: f32,
    pub gravity_power: f32,
    pub gravity_dir: [f32; 3],
    pub hit_radius: f32,
}

pub fn build_sidecar_spring(setup: &SpringBoneSetup, bones: &[BoneInput]) -> SidecarSpring {
    let collision_mode = to_sidecar_collision_mode(setup.collision_mode);
    let colliders_with_id = build_sidecar_colliders(setup, bones);
    let collider_groups = build_sidecar_collider_groups(setup, &colliders_with_id);
    let chains = build_sidecar_chains(setup, bones);

    SidecarSpring {
        collision_mode,
        colliders: colliders_with_id.into_iter().map(|(_, c)| c).collect(),
        collider_groups,
        chains,
    }
}

fn to_sidecar_collision_mode(mode: SpringCollisionMode) -> SidecarCollisionMode {
    match mode {
        SpringCollisionMode::VrmCompatible => SidecarCollisionMode::VrmCompatible,
        SpringCollisionMode::SegmentExtended => SidecarCollisionMode::SegmentExtended,
    }
}

fn build_sidecar_colliders(
    setup: &SpringBoneSetup,
    bones: &[BoneInput],
) -> Vec<(SpringColliderId, SidecarCollider)> {
    let fitted_group_ids: Vec<_> = setup
        .collider_groups
        .iter()
        .filter(|g| g.name == FITTED_BODY_GROUP_NAME)
        .flat_map(|g| g.collider_ids.iter().copied())
        .collect();

    setup
        .colliders
        .iter()
        .filter_map(|c| {
            let bone_name = bones.get(c.bone_id as usize).map(|b| &b.name)?;
            let is_fitted = fitted_group_ids.contains(&c.id);
            let name = if is_fitted {
                format!("{FITTED_BODY_GROUP_NAME}/{bone_name}")
            } else {
                format!("{bone_name}#{}", c.id)
            };
            let (shape, radius, tail) = match &c.shape {
                ColliderShape::Sphere { radius } => (SidecarColliderShape::Sphere, *radius, None),
                ColliderShape::Capsule { radius, tail } => (
                    SidecarColliderShape::Capsule,
                    *radius,
                    Some([tail.x, tail.y, tail.z]),
                ),
            };
            let origin = if is_fitted {
                SidecarColliderOrigin::Fitted
            } else {
                SidecarColliderOrigin::Authored
            };
            Some((
                c.id,
                SidecarCollider {
                    name,
                    bone: bone_name.clone(),
                    offset: [c.offset.x, c.offset.y, c.offset.z],
                    shape,
                    radius,
                    tail,
                    origin,
                },
            ))
        })
        .collect()
}

fn build_sidecar_collider_groups(
    setup: &SpringBoneSetup,
    colliders: &[(SpringColliderId, SidecarCollider)],
) -> Vec<SidecarColliderGroup> {
    setup
        .collider_groups
        .iter()
        .map(|g| SidecarColliderGroup {
            name: g.name.clone(),
            colliders: g
                .collider_ids
                .iter()
                .filter_map(|id| {
                    colliders
                        .iter()
                        .find(|(cid, _)| *cid == *id)
                        .map(|(_, c)| c.name.clone())
                })
                .collect(),
        })
        .collect()
}

fn build_sidecar_chains(setup: &SpringBoneSetup, bones: &[BoneInput]) -> Vec<SidecarSpringChain> {
    setup
        .chains
        .iter()
        .map(|chain| {
            let mut joints: Vec<SidecarSpringJoint> = chain
                .joints
                .iter()
                .filter_map(|joint| {
                    let bone_name = bones.get(joint.bone_id as usize).map(|b| &b.name)?;
                    Some(SidecarSpringJoint {
                        bone: bone_name.clone(),
                        stiffness: joint.stiffness,
                        drag_force: joint.drag_force,
                        gravity_power: joint.gravity_power,
                        gravity_dir: [
                            joint.gravity_dir.x,
                            joint.gravity_dir.y,
                            joint.gravity_dir.z,
                        ],
                        hit_radius: joint.hit_radius,
                    })
                })
                .collect();

            if let Some(tail) = joints
                .last()
                .and_then(|last| build_tail_joint(chain, last, bones))
            {
                joints.push(tail);
            }

            let center = chain
                .center_bone_id
                .and_then(|bid| bones.get(bid as usize).map(|b| b.name.clone()));

            let collider_groups = chain
                .collider_group_ids
                .iter()
                .filter_map(|gid| {
                    setup
                        .collider_groups
                        .iter()
                        .find(|g| g.id == *gid)
                        .map(|g| g.name.clone())
                })
                .collect();

            SidecarSpringChain {
                name: chain.name.clone(),
                center,
                collider_groups,
                joints,
            }
        })
        .collect()
}

fn find_first_child<'a>(bones: &'a [BoneInput], parent: usize) -> Option<&'a BoneInput> {
    bones
        .iter()
        .find(|b| b.parent.map(|p| p == parent).unwrap_or(false))
}

fn build_tail_joint(
    chain: &SpringChain,
    last: &SidecarSpringJoint,
    bones: &[BoneInput],
) -> Option<SidecarSpringJoint> {
    let chain_last = chain.joints.last()?;
    let last_bone_id = chain_last.bone_id as usize;
    if bones.get(last_bone_id).is_none() {
        return None;
    }
    let child = find_first_child(bones, last_bone_id)?;
    Some(SidecarSpringJoint {
        bone: child.name.clone(),
        ..last.clone()
    })
}

#[cfg(test)]
mod tests {
    use cgmath::Vector3;
    use serde_json::json;

    use super::*;
    use thyllore_anim_core::spring_bone::{
        SpringChain, SpringColliderDef, SpringColliderGroup, SpringJointParam,
    };

    fn make_bones(names: &[&str]) -> Vec<BoneInput> {
        names
            .iter()
            .enumerate()
            .map(|(i, name)| BoneInput {
                name: name.to_string(),
                parent: if i == 0 { None } else { Some(i - 1) },
                rest_position: [0.0, 0.0, 0.0],
            })
            .collect()
    }

    #[test]
    fn test_full_mapping() {
        let bones = make_bones(&["Hips", "Spine", "Tail1", "Tail2"]);

        let mut setup = SpringBoneSetup::default();
        setup.collision_mode = SpringCollisionMode::VrmCompatible;

        setup.colliders.push(SpringColliderDef {
            id: 0,
            bone_id: 0,
            offset: Vector3::new(0.0, 0.1, 0.0),
            shape: ColliderShape::Sphere { radius: 0.05 },
        });

        setup.colliders.push(SpringColliderDef {
            id: 1,
            bone_id: 1,
            offset: Vector3::new(0.0, -0.2, 0.0),
            shape: ColliderShape::Capsule {
                radius: 0.03,
                tail: Vector3::new(0.0, 0.1, 0.0),
            },
        });

        setup.collider_groups.push(SpringColliderGroup {
            id: 0,
            name: "Body".to_string(),
            collider_ids: vec![0, 1],
        });

        setup.chains.push(SpringChain {
            id: 0,
            name: "Tail".to_string(),
            joints: vec![
                SpringJointParam {
                    bone_id: 2,
                    stiffness: 0.8,
                    drag_force: 0.5,
                    gravity_power: 1.0,
                    gravity_dir: Vector3::new(0.0, -1.0, 0.0),
                    hit_radius: 0.02,
                },
                SpringJointParam {
                    bone_id: 3,
                    stiffness: 0.6,
                    drag_force: 0.4,
                    gravity_power: 0.5,
                    gravity_dir: Vector3::new(0.0, -1.0, 0.0),
                    hit_radius: 0.01,
                },
            ],
            collider_group_ids: vec![0],
            center_bone_id: Some(0),
            enabled: true,
        });

        setup.next_chain_id = 1;
        setup.next_collider_id = 2;
        setup.next_group_id = 1;

        let sidecar = build_sidecar_spring(&setup, &bones);

        assert_eq!(sidecar.collision_mode, SidecarCollisionMode::VrmCompatible);

        assert_eq!(sidecar.colliders.len(), 2);
        assert_eq!(sidecar.colliders[0].name, "Hips#0");
        assert_eq!(sidecar.colliders[0].bone, "Hips");
        assert_eq!(sidecar.colliders[0].offset, [0.0, 0.1, 0.0]);
        assert_eq!(sidecar.colliders[0].shape, SidecarColliderShape::Sphere);
        assert_eq!(sidecar.colliders[0].radius, 0.05);
        assert_eq!(sidecar.colliders[0].tail, None);
        assert_eq!(sidecar.colliders[0].origin, SidecarColliderOrigin::Authored);

        assert_eq!(sidecar.colliders[1].name, "Spine#1");
        assert_eq!(sidecar.colliders[1].bone, "Spine");
        assert_eq!(sidecar.colliders[1].offset, [0.0, -0.2, 0.0]);
        assert_eq!(sidecar.colliders[1].shape, SidecarColliderShape::Capsule);
        assert_eq!(sidecar.colliders[1].radius, 0.03);
        assert_eq!(sidecar.colliders[1].tail, Some([0.0, 0.1, 0.0]));
        assert_eq!(sidecar.colliders[1].origin, SidecarColliderOrigin::Authored);

        assert_eq!(sidecar.collider_groups.len(), 1);
        assert_eq!(sidecar.collider_groups[0].name, "Body");
        assert_eq!(
            sidecar.collider_groups[0].colliders,
            vec!["Hips#0", "Spine#1"]
        );

        assert_eq!(sidecar.chains.len(), 1);
        let chain = &sidecar.chains[0];
        assert_eq!(chain.name, "Tail");
        assert_eq!(chain.center, Some("Hips".to_string()));
        assert_eq!(chain.collider_groups, vec!["Body"]);
        assert_eq!(chain.joints.len(), 2);

        let j0 = &chain.joints[0];
        assert_eq!(j0.bone, "Tail1");
        assert_eq!(j0.stiffness, 0.8);
        assert_eq!(j0.drag_force, 0.5);
        assert_eq!(j0.gravity_power, 1.0);
        assert_eq!(j0.gravity_dir, [0.0, -1.0, 0.0]);
        assert_eq!(j0.hit_radius, 0.02);

        let j1 = &chain.joints[1];
        assert_eq!(j1.bone, "Tail2");
        assert_eq!(j1.stiffness, 0.6);
        assert_eq!(j1.drag_force, 0.4);
        assert_eq!(j1.gravity_power, 0.5);
        assert_eq!(j1.gravity_dir, [0.0, -1.0, 0.0]);
        assert_eq!(j1.hit_radius, 0.01);
    }

    #[test]
    fn test_fitted_body_origin() {
        let bones = make_bones(&["Hips", "Spine"]);

        let mut setup = SpringBoneSetup::default();

        setup.colliders.push(SpringColliderDef {
            id: 0,
            bone_id: 0,
            offset: Vector3::new(0.0, 0.0, 0.0),
            shape: ColliderShape::Sphere { radius: 0.1 },
        });

        setup.collider_groups.push(SpringColliderGroup {
            id: 0,
            name: FITTED_BODY_GROUP_NAME.to_string(),
            collider_ids: vec![0],
        });

        let sidecar = build_sidecar_spring(&setup, &bones);

        assert_eq!(sidecar.colliders.len(), 1);
        let c = &sidecar.colliders[0];
        assert_eq!(c.origin, SidecarColliderOrigin::Fitted);
        assert_eq!(c.name, format!("{FITTED_BODY_GROUP_NAME}/Hips"));
    }

    #[test]
    fn test_serde_format() {
        let bones = make_bones(&["Hips", "Spine"]);

        let mut setup = SpringBoneSetup::default();
        setup.collision_mode = SpringCollisionMode::VrmCompatible;

        setup.colliders.push(SpringColliderDef {
            id: 0,
            bone_id: 0,
            offset: Vector3::new(0.0, 0.0, 0.0),
            shape: ColliderShape::Capsule {
                radius: 0.05,
                tail: Vector3::new(0.0, 0.1, 0.0),
            },
        });

        let sidecar = build_sidecar_spring(&setup, &bones);
        let value = serde_json::to_value(&sidecar).unwrap();

        assert_eq!(value["collision_mode"], json!("vrm_compatible"));
        assert_eq!(value["colliders"][0]["shape"], json!("capsule"));
    }

    #[test]
    fn test_out_of_range_bone_id_filtered() {
        let bones = make_bones(&["Hips", "Spine"]);

        let mut setup = SpringBoneSetup::default();

        setup.colliders.push(SpringColliderDef {
            id: 0,
            bone_id: 99,
            offset: Vector3::new(0.0, 0.0, 0.0),
            shape: ColliderShape::Sphere { radius: 0.1 },
        });

        setup.colliders.push(SpringColliderDef {
            id: 1,
            bone_id: 1,
            offset: Vector3::new(0.0, 0.0, 0.0),
            shape: ColliderShape::Sphere { radius: 0.1 },
        });

        setup.collider_groups.push(SpringColliderGroup {
            id: 0,
            name: "Test".to_string(),
            collider_ids: vec![0, 1],
        });

        let sidecar = build_sidecar_spring(&setup, &bones);

        assert_eq!(sidecar.colliders.len(), 1);
        assert_eq!(sidecar.colliders[0].name, "Spine#1");

        assert_eq!(sidecar.collider_groups.len(), 1);
        assert_eq!(sidecar.collider_groups[0].colliders, vec!["Spine#1"]);
    }

    #[test]
    fn test_tail_joint_appended_when_child_exists() {
        let bones = make_bones(&["Hips", "Spine", "Tail1", "Tail2", "TailTip"]);

        let mut setup = SpringBoneSetup::default();

        setup.chains.push(SpringChain {
            id: 0,
            name: "Tail".to_string(),
            joints: vec![
                SpringJointParam {
                    bone_id: 1,
                    stiffness: 0.8,
                    drag_force: 0.5,
                    gravity_power: 1.0,
                    gravity_dir: Vector3::new(0.0, -1.0, 0.0),
                    hit_radius: 0.02,
                },
                SpringJointParam {
                    bone_id: 2,
                    stiffness: 0.6,
                    drag_force: 0.4,
                    gravity_power: 0.5,
                    gravity_dir: Vector3::new(0.0, -1.0, 0.0),
                    hit_radius: 0.01,
                },
            ],
            collider_group_ids: vec![],
            center_bone_id: None,
            enabled: true,
        });

        let sidecar = build_sidecar_spring(&setup, &bones);

        assert_eq!(sidecar.chains.len(), 1);
        let chain = &sidecar.chains[0];
        assert_eq!(chain.joints.len(), 3);
        assert_eq!(chain.joints[0].bone, "Spine");
        assert_eq!(chain.joints[1].bone, "Tail1");
        assert_eq!(chain.joints[2].bone, "Tail2");
    }

    #[test]
    fn test_no_tail_joint_when_no_child() {
        let bones = make_bones(&["Hips", "Spine", "Tail1"]);

        let mut setup = SpringBoneSetup::default();

        setup.chains.push(SpringChain {
            id: 0,
            name: "Tail".to_string(),
            joints: vec![
                SpringJointParam {
                    bone_id: 1,
                    stiffness: 0.8,
                    drag_force: 0.5,
                    gravity_power: 1.0,
                    gravity_dir: Vector3::new(0.0, -1.0, 0.0),
                    hit_radius: 0.02,
                },
                SpringJointParam {
                    bone_id: 2,
                    stiffness: 0.6,
                    drag_force: 0.4,
                    gravity_power: 0.5,
                    gravity_dir: Vector3::new(0.0, -1.0, 0.0),
                    hit_radius: 0.01,
                },
            ],
            collider_group_ids: vec![],
            center_bone_id: None,
            enabled: true,
        });

        let sidecar = build_sidecar_spring(&setup, &bones);

        assert_eq!(sidecar.chains.len(), 1);
        let chain = &sidecar.chains[0];
        assert_eq!(chain.joints.len(), 2);
        assert_eq!(chain.joints[0].bone, "Spine");
        assert_eq!(chain.joints[1].bone, "Tail1");
    }
}
