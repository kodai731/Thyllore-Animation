use super::*;
use crate::avatar_setup_bridge::BoneSpec;
use thyllore_anim_core::spring_bone::{ColliderShape, FITTED_BODY_GROUP_NAME};

fn fit_bone(name: &str, parent: Option<usize>, head: [f32; 3]) -> FitBone {
    let mut bind_matrix = [[0.0; 4]; 4];
    for (i, row) in bind_matrix.iter_mut().enumerate() {
        row[i] = 1.0;
        row[3] = head[i.min(2)];
    }
    bind_matrix[3] = [0.0, 0.0, 0.0, 1.0];
    FitBone {
        bone: BoneSpec {
            name: name.to_string(),
            parent,
            rest_position: head,
        },
        bind_matrix,
    }
}

fn bones() -> Vec<FitBone> {
    vec![
        fit_bone("Hips", None, [0.0, 1.0, 0.0]),
        fit_bone("Head", Some(0), [0.0, 1.5, 0.0]),
        fit_bone("hair_hair_back_00_00", Some(1), [0.0, 1.6, -0.1]),
        fit_bone("hair_hair_back_00_01", Some(2), [0.0, 1.4, -0.1]),
    ]
}

fn request<'a>(
    chains: &'a [GeneratedChainIn],
    head_capsule: Option<CapsuleOut>,
) -> RegisterRequest<'a> {
    RegisterRequest {
        root_name: "Head",
        target: "Hair_back/Hair",
        role: "hair",
        chains,
        head_capsule,
    }
}

fn generated() -> Vec<GeneratedChainIn> {
    vec![GeneratedChainIn {
        name: "hair_hair_back_00".to_string(),
        joints: vec![
            GeneratedJointIn {
                name: "hair_hair_back_00_00".to_string(),
                hit_radius: 0.012,
            },
            GeneratedJointIn {
                name: "hair_hair_back_00_01".to_string(),
                hit_radius: 0.008,
            },
        ],
    }]
}

fn capsule() -> CapsuleOut {
    CapsuleOut {
        bottom: [0.0, 1.55, 0.0],
        top: [0.0, 1.7, 0.0],
        radius: 0.09,
    }
}

#[test]
fn head_capsule_becomes_a_fitted_body_collider_on_the_root_in_bone_space() {
    let chains = generated();
    let spring = register_generated_chains(
        &SidecarSpring::default(),
        &bones(),
        &[],
        &request(&chains, Some(capsule())),
    )
    .unwrap();

    assert_eq!(spring.colliders.len(), 1);
    let collider = &spring.colliders[0];
    assert_eq!(collider.bone, "Head");
    let bone_inputs = to_bone_inputs(&bones());
    let setup = spring_setup_from_sidecar(&spring, &bone_inputs);
    match setup.colliders[0].shape {
        ColliderShape::Capsule { radius, tail } => {
            assert!((radius - 0.09).abs() < 1e-6);
            assert!((tail.y - 0.2).abs() < 1e-5);
        }
        ColliderShape::Sphere { .. } => panic!("the head collider is a capsule"),
    }
    assert!((setup.colliders[0].offset.y - 0.05).abs() < 1e-5);

    assert_eq!(spring.collider_groups.len(), 1);
    assert_eq!(spring.collider_groups[0].name, FITTED_BODY_GROUP_NAME);
    assert_eq!(spring.chains.len(), 1);
    assert_eq!(spring.chains[0].collider_groups, [FITTED_BODY_GROUP_NAME]);
    let radii: Vec<f32> = spring.chains[0]
        .joints
        .iter()
        .map(|j| j.hit_radius)
        .collect();
    assert_eq!(radii, [0.012, 0.008]);
}

#[test]
fn regenerating_the_same_target_replaces_its_chains_and_root_collider() {
    let chains = generated();
    let first = register_generated_chains(
        &SidecarSpring::default(),
        &bones(),
        &[],
        &request(&chains, Some(capsule())),
    )
    .unwrap();
    let second =
        register_generated_chains(&first, &bones(), &[], &request(&chains, Some(capsule())))
            .unwrap();

    assert_eq!(second.chains.len(), 1);
    assert_eq!(second.colliders.len(), 1);
    assert_eq!(second.collider_groups[0].colliders.len(), 1);
}

#[test]
fn without_a_capsule_or_skin_the_chains_join_an_existing_fitted_group_only() {
    let chains = generated();
    let spring = register_generated_chains(
        &SidecarSpring::default(),
        &bones(),
        &[],
        &request(&chains, None),
    )
    .unwrap();

    assert!(spring.colliders.is_empty());
    assert!(spring.collider_groups.is_empty());
    assert!(spring.chains[0].collider_groups.is_empty());
}

#[test]
fn unknown_joint_bone_is_an_error() {
    let chains = vec![GeneratedChainIn {
        name: "x".to_string(),
        joints: vec![GeneratedJointIn {
            name: "missing".to_string(),
            hit_radius: 0.0,
        }],
    }];
    assert!(register_generated_chains(
        &SidecarSpring::default(),
        &bones(),
        &[],
        &request(&chains, None)
    )
    .is_err());
}
