mod support;

use std::collections::{BTreeMap, BTreeSet};

use support::test_humanoid::{
    build_stick_mesh, mirrored_role, test_humanoid_bones, write_test_humanoid_fbx, StickTip,
};
use thyllore_avatar_core::expression::components::side::Side;
use thyllore_avatar_core::humanoid::components::role::HumanoidRole;

#[test]
fn test_humanoid_covers_every_role_once() {
    let bones = test_humanoid_bones();

    assert_eq!(bones.len(), 55, "expected 55 bones, got {}", bones.len());

    let mut counts: BTreeMap<HumanoidRole, usize> = BTreeMap::new();
    for bone in &bones {
        *counts.entry(bone.role).or_insert(0) += 1;
    }

    let all_roles: BTreeSet<HumanoidRole> = HumanoidRole::ALL.iter().copied().collect();
    let found_roles: BTreeSet<HumanoidRole> = counts.keys().copied().collect();

    assert_eq!(
        found_roles, all_roles,
        "roles in bones differ from HumanoidRole::ALL"
    );

    for (role, count) in &counts {
        assert_eq!(
            *count, 1,
            "role {:?} appears {} times, expected exactly 1",
            role, count
        );
    }
}

#[test]
fn test_humanoid_parents_precede_children() {
    let bones = test_humanoid_bones();

    for (i, bone) in bones.iter().enumerate() {
        if let Some(parent) = bone.parent {
            assert!(
                parent < i,
                "bone {} ({:?}) at index {} has parent index {} which is not before it",
                i,
                bone.role,
                i,
                parent,
            );
        }
    }
}

#[test]
fn test_humanoid_right_side_mirrors_left() {
    let bones = test_humanoid_bones();

    let role_to_idx: BTreeMap<HumanoidRole, usize> =
        bones.iter().enumerate().map(|(i, b)| (b.role, i)).collect();

    for bone in &bones {
        if bone.role.side() != Some(Side::Left) {
            continue;
        }
        let left_role = bone.role;
        let right_role = mirrored_role(left_role);
        let left_idx = *role_to_idx.get(&left_role).unwrap();
        let right_idx = *role_to_idx.get(&right_role).unwrap();
        let left = &bones[left_idx];
        let right = &bones[right_idx];

        assert!(
            (left.position[0] + right.position[0]).abs() < 1e-9,
            "x mirror failed for {:?}/{:?}: left.x={:.4}, right.x={:.4}",
            left_role,
            right_role,
            left.position[0],
            right.position[0],
        );
        assert!(
            (left.position[1] - right.position[1]).abs() < 1e-9,
            "y match failed for {:?}/{:?}: left.y={:.4}, right.y={:.4}",
            left_role,
            right_role,
            left.position[1],
            right.position[1],
        );
        assert!(
            (left.position[2] - right.position[2]).abs() < 1e-9,
            "z match failed for {:?}/{:?}: left.z={:.4}, right.z={:.4}",
            left_role,
            right_role,
            left.position[2],
            right.position[2],
        );

        match (&left.tip, &right.tip) {
            (StickTip::Child(lr), StickTip::Child(rr)) => {
                let expected_right = mirrored_role(*lr);
                assert_eq!(
                    *rr, expected_right,
                    "child tip mirror failed for {:?}/{:?}: left child={:?}, right child={:?}",
                    left_role, right_role, lr, rr,
                );
            }
            (StickTip::Leaf(lo), StickTip::Leaf(ro)) => {
                assert!(
                    (lo[0] + ro[0]).abs() < 1e-9,
                    "leaf x mirror failed for {:?}/{:?}",
                    left_role,
                    right_role,
                );
                assert!(
                    (lo[1] - ro[1]).abs() < 1e-9,
                    "leaf y match failed for {:?}/{:?}",
                    left_role,
                    right_role,
                );
                assert!(
                    (lo[2] - ro[2]).abs() < 1e-9,
                    "leaf z match failed for {:?}/{:?}",
                    left_role,
                    right_role,
                );
            }
            _ => panic!(
                "tip kind mismatch for {:?}/{:?}: left={:?}, right={:?}",
                left_role, right_role, left.tip, right.tip,
            ),
        }
    }
}

#[test]
fn test_humanoid_mesh_binds_every_vertex_once() {
    let bones = test_humanoid_bones();
    let mesh = build_stick_mesh(&bones);

    let total_vertices = mesh.vertices.len();
    let mut counts = vec![0u32; total_vertices];

    for cluster in &mesh.cluster_vertex_indices {
        for &vi in cluster {
            counts[vi as usize] += 1;
        }
    }

    for (vi, count) in counts.iter().enumerate() {
        assert_eq!(
            *count, 1,
            "vertex {} is in {} clusters (expected exactly 1)",
            vi, count
        );
    }
}

#[test]
fn test_humanoid_fbx_has_white_material_and_a_cluster_per_role() {
    let fbx = write_test_humanoid_fbx();

    assert!(
        fbx.contains("Material::White"),
        "FBX should contain Material::White"
    );

    let cluster_count = fbx.matches("\"Cluster\" {").count();
    assert_eq!(
        cluster_count, 55,
        "expected 55 Cluster entries, got {}",
        cluster_count
    );
}
