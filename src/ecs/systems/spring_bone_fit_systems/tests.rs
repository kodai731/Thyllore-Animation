use super::*;
use crate::animation::spring_bone::{JointForce, SpringCollisionMode, VrmJointForce};
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
                force: JointForce::Vrm(VrmJointForce {
                    stiffness: 0.5,
                    gravity_power: 1.0,
                }),
                gravity_dir: Vector3::new(0.0, -1.0, 0.0),
                drag_force: 0.4,
                hit_radius: 0.0,
            }],
            collider_group_ids: vec![],
            center_bone_id: None,
            tail_bone_id: None,
            enabled: true,
            origin: Default::default(),
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
                force: JointForce::Vrm(VrmJointForce {
                    stiffness: 1.5,
                    gravity_power: 0.3,
                }),
                gravity_dir: Vector3::new(0.0, -1.0, 0.0),
                drag_force: 0.2,
                hit_radius: 0.0,
            }],
            collider_group_ids: vec![],
            center_bone_id: None,
            tail_bone_id: None,
            enabled: true,
            origin: Default::default(),
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
        space: SpringSpace::IDENTITY,
    };

    let reference = simulate_spring_tails(&config, &reference_setup, &skeleton, &clip, None, false);

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
