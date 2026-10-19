use thyllore_model_core::{BoneId, Skeleton};

use crate::spring_bone::{SpringBoneSetup, SpringChain, SpringChainId, SpringJointParam};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RampShape {
    #[default]
    Linear,
    EaseIn,
    EaseOut,
}

impl RampShape {
    pub const ALL: [RampShape; 3] = [RampShape::Linear, RampShape::EaseIn, RampShape::EaseOut];

    pub fn label(&self) -> &'static str {
        match self {
            RampShape::Linear => "Linear",
            RampShape::EaseIn => "Ease In",
            RampShape::EaseOut => "Ease Out",
        }
    }

    pub fn weight(&self, t: f32) -> f32 {
        match self {
            RampShape::Linear => t,
            RampShape::EaseIn => t * t,
            RampShape::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ChainRamp {
    pub root: SpringJointParam,
    pub tip: SpringJointParam,
    pub shape: RampShape,
}

fn lerp(from: f32, to: f32, weight: f32) -> f32 {
    from + (to - from) * weight
}

pub fn apply_ramp(chain: &SpringChain, ramp: &ChainRamp) -> SpringChain {
    let n = chain.joints.len();
    let mut out = chain.clone();

    for (i, joint) in out.joints.iter_mut().enumerate() {
        let t = if n <= 1 {
            0.0
        } else {
            i as f32 / (n - 1) as f32
        };
        let w = ramp.shape.weight(t);

        joint.stiffness = lerp(ramp.root.stiffness, ramp.tip.stiffness, w);
        joint.drag_force = lerp(ramp.root.drag_force, ramp.tip.drag_force, w);
        joint.gravity_power = lerp(ramp.root.gravity_power, ramp.tip.gravity_power, w);
        joint.hit_radius = lerp(ramp.root.hit_radius, ramp.tip.hit_radius, w);
        joint.gravity_dir = ramp.root.gravity_dir;
    }

    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RampScope {
    #[default]
    ThisChain,
    SiblingChains,
    AllChains,
}

impl RampScope {
    pub const ALL: [RampScope; 3] = [
        RampScope::ThisChain,
        RampScope::SiblingChains,
        RampScope::AllChains,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            RampScope::ThisChain => "This Chain",
            RampScope::SiblingChains => "Sibling Chains",
            RampScope::AllChains => "All Chains",
        }
    }
}

fn root_parent(chain: &SpringChain, skeleton: &Skeleton) -> Option<BoneId> {
    let first_joint = chain.joints.first()?;
    skeleton
        .get_bone(first_joint.bone_id)
        .and_then(|b| b.parent_id)
}

pub fn chains_in_scope(
    setup: &SpringBoneSetup,
    skeleton: &Skeleton,
    chain_id: SpringChainId,
    scope: RampScope,
) -> Vec<SpringChainId> {
    match scope {
        RampScope::ThisChain => vec![chain_id],
        RampScope::AllChains => setup.chains.iter().map(|c| c.id).collect(),
        RampScope::SiblingChains => {
            let target_chain = setup.chains.iter().find(|c| c.id == chain_id);
            let Some(target_chain) = target_chain else {
                return Vec::new();
            };
            let Some(parent) = root_parent(target_chain, skeleton) else {
                return Vec::new();
            };
            setup
                .chains
                .iter()
                .filter(|c| root_parent(c, skeleton) == Some(parent))
                .map(|c| c.id)
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use cgmath::Vector3;

    use super::*;

    fn make_chain(bone_ids: &[u32]) -> SpringChain {
        SpringChain {
            joints: bone_ids
                .iter()
                .map(|&bone_id| SpringJointParam {
                    bone_id,
                    stiffness: 1.0,
                    drag_force: 0.5,
                    gravity_power: 1.0,
                    gravity_dir: Vector3::new(0.0, -1.0, 0.0),
                    hit_radius: 0.1,
                })
                .collect(),
            ..Default::default()
        }
    }

    fn ramp(root_stiffness: f32, tip_stiffness: f32, shape: RampShape) -> ChainRamp {
        ChainRamp {
            root: SpringJointParam {
                stiffness: root_stiffness,
                ..Default::default()
            },
            tip: SpringJointParam {
                stiffness: tip_stiffness,
                ..Default::default()
            },
            shape,
        }
    }

    #[test]
    fn single_joint_is_root_value() {
        let chain = make_chain(&[0]);
        let ramp = ramp(0.8, 0.2, RampShape::Linear);
        let result = apply_ramp(&chain, &ramp);

        assert_eq!(result.joints.len(), 1);
        assert!((result.joints[0].stiffness - 0.8).abs() < 1e-6);
    }

    #[test]
    fn four_joints_linear_interpolation() {
        let chain = make_chain(&[0, 1, 2, 3]);
        let ramp = ramp(0.8, 0.2, RampShape::Linear);
        let result = apply_ramp(&chain, &ramp);

        assert_eq!(result.joints.len(), 4);
        assert!((result.joints[0].stiffness - 0.8).abs() < 1e-6);
        assert!((result.joints[1].stiffness - 0.6).abs() < 1e-6);
        assert!((result.joints[2].stiffness - 0.4).abs() < 1e-6);
        assert!((result.joints[3].stiffness - 0.2).abs() < 1e-6);
    }

    #[test]
    fn four_joints_ease_in_second_joint_closer_to_root() {
        let chain = make_chain(&[0, 1, 2, 3]);
        let ramp_linear = ramp(0.8, 0.2, RampShape::Linear);
        let ramp_ease_in = ramp(0.8, 0.2, RampShape::EaseIn);

        let result_linear = apply_ramp(&chain, &ramp_linear);
        let result_ease_in = apply_ramp(&chain, &ramp_ease_in);
        assert!(result_ease_in.joints[1].stiffness > result_linear.joints[1].stiffness);
    }

    #[test]
    fn gravity_dir_is_root_for_all_joints() {
        let chain = make_chain(&[0, 1, 2]);
        let ramp = ChainRamp {
            root: SpringJointParam {
                gravity_dir: Vector3::new(1.0, 0.0, 0.0),
                ..Default::default()
            },
            tip: SpringJointParam {
                gravity_dir: Vector3::new(0.0, 1.0, 0.0),
                ..Default::default()
            },
            shape: RampShape::Linear,
        };
        let result = apply_ramp(&chain, &ramp);

        for joint in &result.joints {
            assert_eq!(joint.gravity_dir, Vector3::new(1.0, 0.0, 0.0));
        }
    }

    #[test]
    fn bone_ids_preserved_from_input_chain() {
        let chain = make_chain(&[5, 12, 7]);
        let ramp = ramp(0.8, 0.2, RampShape::Linear);
        let result = apply_ramp(&chain, &ramp);

        assert_eq!(result.joints[0].bone_id, 5);
        assert_eq!(result.joints[1].bone_id, 12);
        assert_eq!(result.joints[2].bone_id, 7);
    }

    fn build_head_ring_skeleton(
        chain_count: usize,
        chain_length: usize,
    ) -> (Skeleton, Vec<Vec<BoneId>>) {
        let mut skeleton = Skeleton::default();
        skeleton.add_bone("Head", None);
        let mut chain_bones: Vec<Vec<BoneId>> = Vec::new();
        for c in 0..chain_count {
            let mut prev: Option<u32> = Some(0);
            let mut bones: Vec<BoneId> = Vec::new();
            for j in 0..chain_length {
                let name = format!("chain{}_bone{}", c, j);
                let bone_id = skeleton.add_bone(&name, prev);
                bones.push(bone_id);
                prev = Some(bone_id);
            }
            chain_bones.push(bones);
        }
        (skeleton, chain_bones)
    }

    fn build_full_test_data(
        chain_count: usize,
        chain_length: usize,
    ) -> (Skeleton, SpringBoneSetup) {
        let (skeleton, chain_bones) = build_head_ring_skeleton(chain_count, chain_length);
        let mut setup = SpringBoneSetup::default();

        for (i, bones) in chain_bones.into_iter().enumerate() {
            let joints: Vec<SpringJointParam> = bones
                .iter()
                .map(|&bone_id| SpringJointParam {
                    bone_id,
                    ..Default::default()
                })
                .collect();
            setup.chains.push(SpringChain {
                id: i as u32,
                name: format!("chain{}", i),
                joints,
                ..Default::default()
            });
            setup.next_chain_id = i as u32 + 1;
        }
        (skeleton, setup)
    }

    #[test]
    fn sibling_chains_returns_all_six() {
        let (skeleton, setup) = build_full_test_data(6, 4);

        let result = chains_in_scope(&setup, &skeleton, 0, RampScope::SiblingChains);
        assert_eq!(result.len(), 6);
        for i in 0..6u32 {
            assert!(result.contains(&i), "chain {} should be in scope", i);
        }
    }

    #[test]
    fn sibling_chains_empty_for_root_chain_without_parent() {
        let mut skeleton = Skeleton::default();
        skeleton.add_bone("Hips", None);
        skeleton.add_bone("Spine", Some(0));

        let mut setup = SpringBoneSetup::default();
        setup.chains.push(SpringChain {
            id: 0,
            name: "root_chain".to_string(),
            joints: vec![SpringJointParam {
                bone_id: 0,
                ..Default::default()
            }],
            ..Default::default()
        });
        setup.next_chain_id = 1;
        setup.chains.push(SpringChain {
            id: 1,
            name: "spine_chain".to_string(),
            joints: vec![SpringJointParam {
                bone_id: 1,
                ..Default::default()
            }],
            ..Default::default()
        });
        setup.next_chain_id = 2;

        let result = chains_in_scope(&setup, &skeleton, 0, RampScope::SiblingChains);
        assert!(
            result.is_empty(),
            "root chain with no parent should return empty sibling scope"
        );
    }

    #[test]
    fn this_chain_returns_only_self() {
        let (skeleton, setup) = build_full_test_data(6, 4);

        let result = chains_in_scope(&setup, &skeleton, 3, RampScope::ThisChain);
        assert_eq!(result, vec![3]);
    }

    #[test]
    fn all_chains_returns_all() {
        let (skeleton, setup) = build_full_test_data(6, 4);

        let result = chains_in_scope(&setup, &skeleton, 2, RampScope::AllChains);
        assert_eq!(result.len(), 6);
        for i in 0..6u32 {
            assert!(result.contains(&i), "chain {} should be in scope", i);
        }
    }

    #[test]
    fn sibling_chains_excludes_different_parent() {
        let mut skeleton = Skeleton::default();
        skeleton.add_bone("Head", None);
        skeleton.add_bone("Neck", Some(0));

        for c in 0..3u32 {
            let name = format!("head_chain{}_bone0", c);
            skeleton.add_bone(&name, Some(0));
        }
        skeleton.add_bone("neck_chain0", Some(1));

        let mut setup = SpringBoneSetup::default();
        for i in 0..3u32 {
            setup.chains.push(SpringChain {
                id: i,
                name: format!("head_chain{}", i),
                joints: vec![SpringJointParam {
                    bone_id: i + 2,
                    ..Default::default()
                }],
                ..Default::default()
            });
        }
        setup.next_chain_id = 3;
        setup.chains.push(SpringChain {
            id: 3,
            name: "neck_chain".to_string(),
            joints: vec![SpringJointParam {
                bone_id: 5,
                ..Default::default()
            }],
            ..Default::default()
        });
        setup.next_chain_id = 4;

        let result = chains_in_scope(&setup, &skeleton, 0, RampScope::SiblingChains);
        assert_eq!(result.len(), 3);
        assert!(result.contains(&0));
        assert!(result.contains(&1));
        assert!(result.contains(&2));
        assert!(!result.contains(&3), "neck chain should not be in scope");
    }

    #[test]
    fn missing_chain_returns_empty() {
        let (skeleton, setup) = build_full_test_data(6, 4);

        let result = chains_in_scope(&setup, &skeleton, 99, RampScope::SiblingChains);
        assert!(result.is_empty());
    }
}
