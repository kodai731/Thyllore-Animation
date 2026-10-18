use crate::animation::spring_bone::{compare_tail_trajectories, SpringBoneSetup, SpringTailSample};
use crate::animation::{AnimationClip, Skeleton};
use crate::ecs::component::{ConstraintSet, SpringChain};
use thyllore_math_core::{minimize_nelder_mead, NelderMeadParams};

use super::spring_bone_bake_systems::{simulate_spring_tails, BakeConfig};

#[derive(Clone, Debug, PartialEq)]
pub struct SpringChainFit {
    pub chain: String,
    pub before_rms: f32,
    pub after_rms: f32,
    pub stiffness: f32,
    pub drag_force: f32,
    pub gravity_power: f32,
}

pub struct SpringFitInputs<'a> {
    pub config: &'a BakeConfig,
    pub skeleton: &'a Skeleton,
    pub clip: &'a AnimationClip,
    pub constraints: Option<&'a ConstraintSet>,
    pub looping: bool,
    pub reference: &'a [SpringTailSample],
}

fn clamp_values(values: &[f32]) -> [f32; 3] {
    [
        values[0].max(0.0),
        values[1].clamp(0.0, 1.0),
        values[2].max(0.0),
    ]
}

fn with_chain_values(
    setup: &SpringBoneSetup,
    chain_name: &str,
    values: [f32; 3],
) -> SpringBoneSetup {
    let mut out = setup.clone();
    for chain in &mut out.chains {
        if chain.name == chain_name {
            for joint in &mut chain.joints {
                joint.stiffness = values[0];
                joint.drag_force = values[1];
                joint.gravity_power = values[2];
            }
        }
    }
    out
}

fn chain_rms(
    inputs: &SpringFitInputs,
    setup: &SpringBoneSetup,
    chain_name: &str,
    values: [f32; 3],
) -> f32 {
    let eval_setup = with_chain_values(setup, chain_name, values);
    let simulated = simulate_spring_tails(
        inputs.config,
        &eval_setup,
        inputs.skeleton,
        inputs.clip,
        inputs.constraints,
        inputs.looping,
    );
    let errors = compare_tail_trajectories(inputs.reference, &simulated);
    for e in errors {
        if e.chain == chain_name {
            return e.rms;
        }
    }
    f32::MAX
}

fn chain_start_values(chain: &SpringChain) -> Option<[f32; 3]> {
    let first = chain.joints.first()?;
    Some([first.stiffness, first.drag_force, first.gravity_power])
}

pub fn fit_spring_params(
    inputs: &SpringFitInputs,
    setup: &SpringBoneSetup,
    params: &NelderMeadParams,
) -> (SpringBoneSetup, Vec<SpringChainFit>) {
    let mut current_setup = setup.clone();
    let mut fits: Vec<SpringChainFit> = Vec::new();

    for chain in &setup.chains {
        if !chain.enabled {
            continue;
        }

        let has_reference = inputs.reference.iter().any(|s| s.chain == chain.name);
        if !has_reference {
            continue;
        }

        let start_values = match chain_start_values(chain) {
            Some(v) => v,
            None => continue,
        };

        let before_rms = chain_rms(inputs, &current_setup, &chain.name, start_values);

        let objective = |x: &[f32]| -> f32 {
            let clamped = clamp_values(x);
            chain_rms(inputs, &current_setup, &chain.name, clamped)
        };

        let result = minimize_nelder_mead(objective, &start_values, params);
        let best_clamped = clamp_values(&result.point);

        for joint in &mut current_setup
            .chains
            .iter_mut()
            .find(|c| c.name == chain.name)
            .unwrap()
            .joints
        {
            joint.stiffness = best_clamped[0];
            joint.drag_force = best_clamped[1];
            joint.gravity_power = best_clamped[2];
        }

        let after_rms = chain_rms(inputs, &current_setup, &chain.name, best_clamped);

        fits.push(SpringChainFit {
            chain: chain.name.clone(),
            before_rms,
            after_rms,
            stiffness: best_clamped[0],
            drag_force: best_clamped[1],
            gravity_power: best_clamped[2],
        });
    }

    (current_setup, fits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::spring_bone::SpringCollisionMode;
    use crate::animation::{Bone, Keyframe, TransformChannel};
    use crate::ecs::component::SpringJointParam;
    use cgmath::{Matrix4, Vector3};

    fn create_test_skeleton() -> Skeleton {
        let bones = vec![
            Bone {
                id: 0,
                name: "Root".to_string(),
                parent_id: None,
                children: vec![1],
                local_transform: Matrix4::from_translation(Vector3::new(0.0, 0.0, 0.0)),
                ..Default::default()
            },
            Bone {
                id: 1,
                name: "Spring".to_string(),
                parent_id: Some(0),
                children: vec![2],
                local_transform: Matrix4::from_translation(Vector3::new(0.0, 1.0, 0.0)),
                ..Default::default()
            },
            Bone {
                id: 2,
                name: "SpringTip".to_string(),
                parent_id: Some(1),
                children: vec![],
                local_transform: Matrix4::from_translation(Vector3::new(0.0, 1.0, 0.0)),
                ..Default::default()
            },
        ];

        let mut bone_name_to_id = std::collections::HashMap::new();
        for bone in &bones {
            bone_name_to_id.insert(bone.name.clone(), bone.id);
        }

        Skeleton {
            id: 0,
            name: "TestSkeleton".to_string(),
            bones,
            bone_name_to_id,
            root_bone_ids: vec![0],
            root_transform: Matrix4::from_scale(1.0),
        }
    }

    fn create_default_setup() -> SpringBoneSetup {
        SpringBoneSetup {
            chains: vec![SpringChain {
                id: 0,
                name: "TestChain".to_string(),
                joints: vec![SpringJointParam {
                    bone_id: 1,
                    stiffness: 0.5,
                    gravity_power: 1.0,
                    gravity_dir: Vector3::new(0.0, -1.0, 0.0),
                    drag_force: 0.4,
                    hit_radius: 0.0,
                }],
                collider_group_ids: vec![],
                center_bone_id: None,
                enabled: true,
            }],
            colliders: vec![],
            collider_groups: vec![],
            next_chain_id: 1,
            next_collider_id: 0,
            next_group_id: 0,
            collision_mode: SpringCollisionMode::VrmCompatible,
        }
    }

    fn create_test_clip() -> AnimationClip {
        let mut clip = AnimationClip::new("test");
        let mut channel = TransformChannel::default();
        channel
            .translation
            .push(Keyframe::new(0.0, Vector3::new(0.0, 0.0, 0.0)));
        channel
            .translation
            .push(Keyframe::new(0.5, Vector3::new(1.0, 0.0, 0.0)));
        channel
            .translation
            .push(Keyframe::new(1.0, Vector3::new(0.0, 0.0, 0.0)));
        clip.add_channel(0, channel);
        clip
    }

    #[test]
    fn test_fit_improves_rms() {
        let skeleton = create_test_skeleton();
        let default_setup = create_default_setup();
        let clip = create_test_clip();

        let reference_setup = SpringBoneSetup {
            chains: vec![SpringChain {
                id: 0,
                name: "TestChain".to_string(),
                joints: vec![SpringJointParam {
                    bone_id: 1,
                    stiffness: 1.5,
                    gravity_power: 0.3,
                    gravity_dir: Vector3::new(0.0, -1.0, 0.0),
                    drag_force: 0.2,
                    hit_radius: 0.0,
                }],
                collider_group_ids: vec![],
                center_bone_id: None,
                enabled: true,
            }],
            colliders: vec![],
            collider_groups: vec![],
            next_chain_id: 1,
            next_collider_id: 0,
            next_group_id: 0,
            collision_mode: SpringCollisionMode::VrmCompatible,
        };

        let config = BakeConfig {
            start_time: 0.0,
            end_time: 1.0,
            sample_rate: 30.0,
        };

        let reference =
            simulate_spring_tails(&config, &reference_setup, &skeleton, &clip, None, false);

        let opt_params = NelderMeadParams {
            initial_step: 0.2,
            max_iterations: 300,
            tolerance: 1e-7,
        };

        let inputs = SpringFitInputs {
            config: &config,
            skeleton: &skeleton,
            clip: &clip,
            constraints: None,
            looping: false,
            reference: &reference,
        };
        let (_optimized_setup, fits) = fit_spring_params(&inputs, &default_setup, &opt_params);

        assert_eq!(fits.len(), 1);
        let fit = &fits[0];
        assert_eq!(fit.chain, "TestChain");
        assert!(
            fit.after_rms < fit.before_rms * 0.1,
            "after_rms {} should be less than before_rms * 0.1 ({})",
            fit.after_rms,
            fit.before_rms * 0.1
        );
        assert!(
            fit.after_rms < 1e-3,
            "after_rms {} should be less than 1e-3",
            fit.after_rms
        );
    }
}
