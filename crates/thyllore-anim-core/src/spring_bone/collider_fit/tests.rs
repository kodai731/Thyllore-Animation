use super::*;
use crate::spring_bone::{SpringChain, SpringColliderGroup, SpringJointParam};
use cgmath::{Matrix4, SquareMatrix, Vector4};

fn make_cylinder_points(radius: f32, length: f32, samples: usize) -> Vec<Vector3<f32>> {
    let mut points = Vec::new();
    for i in 0..samples {
        let angle = 2.0 * std::f32::consts::PI * (i as f32) / (samples as f32);
        let x = length * 0.5;
        let y = radius * angle.cos();
        let z = radius * angle.sin();
        points.push(Vector3::new(x, y, z));
    }
    for i in 0..samples {
        let angle = 2.0 * std::f32::consts::PI * (i as f32) / (samples as f32);
        let x = -length * 0.5;
        let y = radius * angle.cos();
        let z = radius * angle.sin();
        points.push(Vector3::new(x, y, z));
    }
    points
}

fn make_test_skin(
    local_points: &[Vector3<f32>],
    inverse_bind_pose: Matrix4<f32>,
) -> (SkinData, Skeleton) {
    let bind_pose = inverse_bind_pose.invert().unwrap();
    let world_positions: Vec<Vector3<f32>> = local_points
        .iter()
        .map(|p| {
            let transformed = bind_pose * Vector4::new(p.x, p.y, p.z, 1.0);
            Vector3::new(transformed.x, transformed.y, transformed.z)
        })
        .collect();
    make_test_skin_from_world(&world_positions, inverse_bind_pose)
}

fn make_test_skin_from_world(
    world_positions: &[Vector3<f32>],
    inverse_bind_pose: Matrix4<f32>,
) -> (SkinData, Skeleton) {
    let mut skeleton = Skeleton::new("test");
    let b0 = skeleton.add_bone("bone0", None);
    let _b1 = skeleton.add_bone("bone1", Some(b0));

    *skeleton.get_bone_mut(b0).unwrap() = Bone {
        id: b0,
        name: "bone0".to_string(),
        parent_id: None,
        children: vec![_b1],
        local_transform: Matrix4::identity(),
        inverse_bind_pose,
        node_index: None,
    };

    let mut skin = SkinData::default();
    for p in world_positions {
        skin.base_positions.push(*p);
        skin.bone_indices.push(cgmath::Vector4::new(b0, 0, 0, 0));
        skin.bone_weights
            .push(cgmath::Vector4::new(1.0, 0.0, 0.0, 0.0));
        skin.base_normals.push(Vector3::new(0.0, 0.0, 1.0));
    }

    (skin, skeleton)
}

#[test]
fn test_cylinder_fit() {
    let local_points = make_cylinder_points(0.5, 2.0, 32);
    let identity = Matrix4::identity();
    let (skin, skeleton) = make_test_skin(&local_points, identity);

    let params = CapsuleFitParams {
        min_weight: 0.1,
        radius_percentile: 0.5,
        radius_scale: 1.0,
        min_vertex_count: 4,
    };

    let capsules = fit_bone_capsules(&[&skin], &skeleton, &params);

    assert_eq!(capsules.len(), 1);
    let cap = &capsules[0];
    assert_eq!(cap.bone_id, 0);

    assert!((cap.radius - 0.5).abs() < 1e-3, "radius: {}", cap.radius);
    assert!((cap.head.x - (-0.5)).abs() < 1e-3, "head.x: {}", cap.head.x);
    assert!((cap.tail.x - 0.5).abs() < 1e-3, "tail.x: {}", cap.tail.x);
}

#[test]
fn test_unassigned_bone_not_output() {
    let local_points = make_cylinder_points(0.5, 2.0, 8);
    let identity = Matrix4::identity();
    let (skin, skeleton) = make_test_skin(&local_points, identity);

    let params = CapsuleFitParams {
        min_weight: 0.1,
        radius_percentile: 0.5,
        radius_scale: 1.0,
        min_vertex_count: 4,
    };

    let capsules = fit_bone_capsules(&[&skin], &skeleton, &params);

    for cap in &capsules {
        assert_ne!(cap.bone_id, 1, "bone 1 should not be in output");
    }
}

#[test]
fn test_inverse_bind_pose_translation() {
    let world_positions = make_cylinder_points(0.5, 2.0, 32);
    let inverse_bind_pose = Matrix4::from_translation(Vector3::new(-1.0, 0.0, 0.0));
    let (skin, skeleton) = make_test_skin_from_world(&world_positions, inverse_bind_pose);

    let params = CapsuleFitParams {
        min_weight: 0.1,
        radius_percentile: 0.5,
        radius_scale: 1.0,
        min_vertex_count: 4,
    };

    let capsules = fit_bone_capsules(&[&skin], &skeleton, &params);

    assert_eq!(capsules.len(), 1);
    let cap = &capsules[0];

    assert!((cap.head.x - (-1.5)).abs() < 1e-3, "head.x: {}", cap.head.x);
    assert!((cap.tail.x - (-0.5)).abs() < 1e-3, "tail.x: {}", cap.tail.x);
}
#[test]
fn test_radius_scale() {
    let local_points = make_cylinder_points(0.5, 2.0, 32);
    let identity = Matrix4::identity();
    let (skin, skeleton) = make_test_skin(&local_points, identity);

    let params = CapsuleFitParams {
        min_weight: 0.1,
        radius_percentile: 0.5,
        radius_scale: 0.8,
        min_vertex_count: 4,
    };

    let capsules = fit_bone_capsules(&[&skin], &skeleton, &params);

    assert_eq!(capsules.len(), 1);
    let cap = &capsules[0];
    assert!((cap.radius - 0.4).abs() < 1e-3, "radius: {}", cap.radius);
}

#[test]
fn test_two_skin_split() {
    let full_points = make_cylinder_points(0.5, 2.0, 32);
    let identity = Matrix4::identity();
    let (skin_full, skeleton) = make_test_skin(&full_points, identity);

    let params = CapsuleFitParams {
        min_weight: 0.1,
        radius_percentile: 0.5,
        radius_scale: 1.0,
        min_vertex_count: 4,
    };

    let capsules_full = fit_bone_capsules(&[&skin_full], &skeleton, &params);
    assert_eq!(capsules_full.len(), 1);
    let cap_full = &capsules_full[0];

    let left_points: Vec<Vector3<f32>> =
        full_points.iter().filter(|p| p.x < 0.0).cloned().collect();
    let right_points: Vec<Vector3<f32>> =
        full_points.iter().filter(|p| p.x >= 0.0).cloned().collect();

    let (skin_left, _) = make_test_skin(&left_points, identity);
    let (skin_right, _) = make_test_skin(&right_points, identity);

    let capsules_split = fit_bone_capsules(&[&skin_left, &skin_right], &skeleton, &params);
    assert_eq!(capsules_split.len(), 1);
    let cap_split = &capsules_split[0];

    assert!(
        (cap_full.head.x - cap_split.head.x).abs() < 1e-3,
        "head.x: full={} split={}",
        cap_full.head.x,
        cap_split.head.x
    );
    assert!(
        (cap_full.tail.x - cap_split.tail.x).abs() < 1e-3,
        "tail.x: full={} split={}",
        cap_full.tail.x,
        cap_split.tail.x
    );
    assert!(
        (cap_full.radius - cap_split.radius).abs() < 1e-3,
        "radius: full={} split={}",
        cap_full.radius,
        cap_split.radius
    );
}

#[test]
fn test_replace_fitted_body_colliders_basic() {
    let mut setup = SpringBoneSetup::default();
    setup.chains.push(SpringChain {
        id: 0,
        name: "chain0".to_string(),
        joints: vec![SpringJointParam {
            bone_id: 0,
            ..Default::default()
        }],
        collider_group_ids: Vec::new(),
        center_bone_id: None,
        enabled: true,
        origin: Default::default(),
    });
    setup.next_chain_id = 1;

    let capsules = vec![
        FittedCapsule {
            bone_id: 0,
            head: Vector3::new(0.0, 0.0, 0.0),
            tail: Vector3::new(0.0, 1.0, 0.0),
            radius: 0.1,
        },
        FittedCapsule {
            bone_id: 1,
            head: Vector3::new(0.0, 1.0, 0.0),
            tail: Vector3::new(0.0, 2.0, 0.0),
            radius: 0.08,
        },
    ];

    replace_fitted_body_colliders(&mut setup, &capsules);

    assert_eq!(setup.colliders.len(), 2);
    assert_eq!(setup.collider_groups.len(), 1);
    let group = &setup.collider_groups[0];
    assert_eq!(group.name, FITTED_BODY_GROUP_NAME);
    assert_eq!(group.collider_ids.len(), 2);
    assert_eq!(setup.chains[0].collider_group_ids.len(), 1);
    assert_eq!(setup.chains[0].collider_group_ids[0], group.id);
}

#[test]
fn test_replace_fitted_body_colliders_idempotent() {
    let mut setup = SpringBoneSetup::default();
    setup.chains.push(SpringChain {
        id: 0,
        name: "chain0".to_string(),
        joints: vec![SpringJointParam {
            bone_id: 0,
            ..Default::default()
        }],
        collider_group_ids: Vec::new(),
        center_bone_id: None,
        enabled: true,
        origin: Default::default(),
    });
    setup.next_chain_id = 1;

    let capsules = vec![
        FittedCapsule {
            bone_id: 0,
            head: Vector3::new(0.0, 0.0, 0.0),
            tail: Vector3::new(0.0, 1.0, 0.0),
            radius: 0.1,
        },
        FittedCapsule {
            bone_id: 1,
            head: Vector3::new(0.0, 1.0, 0.0),
            tail: Vector3::new(0.0, 2.0, 0.0),
            radius: 0.08,
        },
    ];

    replace_fitted_body_colliders(&mut setup, &capsules);
    let first_count = setup.colliders.len();
    let first_group_count = setup.collider_groups.len();
    let first_ref_count = setup.chains[0].collider_group_ids.len();

    replace_fitted_body_colliders(&mut setup, &capsules);

    assert_eq!(setup.colliders.len(), first_count);
    assert_eq!(setup.collider_groups.len(), first_group_count);
    assert_eq!(setup.chains[0].collider_group_ids.len(), first_ref_count);
}

#[test]
fn test_replace_fitted_body_colliders_preserves_other_groups() {
    let mut setup = SpringBoneSetup::default();
    setup.chains.push(SpringChain {
        id: 0,
        name: "chain0".to_string(),
        joints: vec![SpringJointParam {
            bone_id: 0,
            ..Default::default()
        }],
        collider_group_ids: Vec::new(),
        center_bone_id: None,
        enabled: true,
        origin: Default::default(),
    });
    setup.next_chain_id = 1;

    setup.colliders.push(SpringColliderDef {
        id: 0,
        bone_id: 2,
        offset: Vector3::new(1.0, 0.0, 0.0),
        shape: ColliderShape::Sphere { radius: 0.2 },
    });
    setup.next_collider_id = 1;
    setup.collider_groups.push(SpringColliderGroup {
        id: 0,
        name: "User Group".to_string(),
        collider_ids: vec![0],
    });
    setup.next_group_id = 1;
    setup.chains[0].collider_group_ids.push(0);

    let capsules = vec![FittedCapsule {
        bone_id: 0,
        head: Vector3::new(0.0, 0.0, 0.0),
        tail: Vector3::new(0.0, 1.0, 0.0),
        radius: 0.1,
    }];

    replace_fitted_body_colliders(&mut setup, &capsules);

    assert_eq!(setup.collider_groups.len(), 2);
    let user_group = setup
        .collider_groups
        .iter()
        .find(|g| g.name == "User Group")
        .expect("User Group should still exist");
    assert_eq!(user_group.collider_ids, vec![0]);
    let fitted_group = setup
        .collider_groups
        .iter()
        .find(|g| g.name == FITTED_BODY_GROUP_NAME)
        .expect("Fitted Body group should exist");
    assert_eq!(fitted_group.collider_ids.len(), 1);
    assert!(setup.chains[0].collider_group_ids.contains(&user_group.id));
    assert!(setup.chains[0]
        .collider_group_ids
        .contains(&fitted_group.id));
}

fn capsule_on(bone_id: BoneId, radius: f32) -> FittedCapsule {
    FittedCapsule {
        bone_id,
        head: Vector3::new(0.0, 0.0, 0.0),
        tail: Vector3::new(0.0, 1.0, 0.0),
        radius,
    }
}

#[test]
fn ensure_fitted_collider_creates_the_group_once() {
    let mut setup = SpringBoneSetup::default();

    let first = ensure_fitted_collider(&mut setup, &capsule_on(3, 0.1));
    let second = ensure_fitted_collider(&mut setup, &capsule_on(4, 0.2));

    assert_eq!(first, second);
    assert_eq!(setup.collider_groups.len(), 1);
    assert_eq!(setup.collider_groups[0].name, FITTED_BODY_GROUP_NAME);
    assert_eq!(setup.collider_groups[0].collider_ids.len(), 2);
    assert_eq!(setup.colliders.len(), 2);
}

#[test]
fn ensure_fitted_collider_replaces_the_collider_of_the_same_bone() {
    let mut setup = SpringBoneSetup::default();
    replace_fitted_body_colliders(&mut setup, &[capsule_on(3, 0.1), capsule_on(5, 0.3)]);

    ensure_fitted_collider(&mut setup, &capsule_on(3, 0.25));

    assert_eq!(setup.colliders.len(), 2);
    let on_bone_3: Vec<_> = setup.colliders.iter().filter(|c| c.bone_id == 3).collect();
    assert_eq!(on_bone_3.len(), 1);
    match on_bone_3[0].shape {
        ColliderShape::Capsule { radius, .. } => assert!((radius - 0.25).abs() < 1e-6),
        ColliderShape::Sphere { .. } => panic!("fitted collider is a capsule"),
    }
    let group = &setup.collider_groups[0];
    assert_eq!(group.collider_ids.len(), 2);
    assert!(group.collider_ids.contains(&on_bone_3[0].id));
}

#[test]
fn ensure_fitted_collider_keeps_other_groups_and_chain_bindings() {
    let mut setup = SpringBoneSetup::default();
    setup.collider_groups.push(SpringColliderGroup {
        id: 7,
        name: "Hand".to_string(),
        collider_ids: vec![],
    });
    setup.next_group_id = 8;

    let group_id = ensure_fitted_collider(&mut setup, &capsule_on(2, 0.1));

    assert_eq!(group_id, 8);
    assert_eq!(setup.collider_groups.len(), 2);
    assert_eq!(setup.collider_groups[0].name, "Hand");
}
