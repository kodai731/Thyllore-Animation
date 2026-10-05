#![cfg(test)]
mod support;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use support::fixture_bones::load_rig_from_fbx_text;
use support::test_humanoid::{
    build_stick_mesh, extra_bones, skeleton, write_test_humanoid_fbx, CanonicalTip,
};
use thyllore_avatar_core::expression::components::side::Side;
use thyllore_avatar_core::humanoid::canonical::fixtures::ExtraParent;
use thyllore_avatar_core::humanoid::components::role::HumanoidRole;

#[test]
fn test_humanoid_covers_every_role_once() {
    let bones = skeleton();

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
    let bones = skeleton();

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
    let bones = skeleton();

    let role_to_idx: BTreeMap<HumanoidRole, usize> =
        bones.iter().enumerate().map(|(i, b)| (b.role, i)).collect();

    for bone in &bones {
        if bone.role.side() != Some(Side::Left) {
            continue;
        }
        let left_role = bone.role;
        let right_role = left_role.mirrored();
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
            (CanonicalTip::Child(lr), CanonicalTip::Child(rr)) => {
                let expected_right = lr.mirrored();
                assert_eq!(
                    *rr, expected_right,
                    "child tip mirror failed for {:?}/{:?}: left child={:?}, right child={:?}",
                    left_role, right_role, lr, rr,
                );
            }
            (CanonicalTip::Leaf(lo), CanonicalTip::Leaf(ro)) => {
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
    let bones = skeleton();
    let extras = extra_bones();
    let mesh = build_stick_mesh(&bones, &extras);

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
fn test_humanoid_fbx_has_white_and_cloth_materials_and_a_cluster_per_bone() {
    let fbx = write_test_humanoid_fbx();

    assert!(
        fbx.contains("Material::White"),
        "FBX should contain Material::White"
    );

    assert!(
        fbx.contains("Material::Cloth"),
        "FBX should contain Material::Cloth"
    );

    let cluster_count = fbx.matches("\"Cluster\" {").count();
    assert_eq!(
        cluster_count, 94,
        "expected 94 Cluster entries (55 role + 39 extra), got {}",
        cluster_count
    );
}

fn asset_path() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf();
    root.join("assets/models/test_humanoid/test_humanoid.fbx")
}

#[test]
#[ignore]
fn generate_test_humanoid() {
    let path = asset_path();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let fbx = write_test_humanoid_fbx();
    std::fs::write(&path, fbx).unwrap();
}

#[test]
fn test_humanoid_matches_generator() {
    let path = asset_path();
    let recorded = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => panic!(
            "asset not found at {:?}: {}\n\ngenerate it with:\ncargo test -p thyllore-avatar-core --test test_humanoid_asset generate_test_humanoid -- --ignored",
            path, e
        ),
    };
    let generated = write_test_humanoid_fbx();
    assert_eq!(
        recorded, generated,
        "asset file differs from generator output"
    );
}

#[test]
fn test_humanoid_maps_every_role() {
    let (bones, _) = load_rig_from_fbx_text("test_humanoid", &write_test_humanoid_fbx());
    let (mapping, unresolved) =
        thyllore_avatar_core::humanoid::systems::name_match::infer_mapping(&bones);

    let found: BTreeSet<HumanoidRole> = mapping.by_role.keys().copied().collect();
    let all: BTreeSet<HumanoidRole> = HumanoidRole::ALL.iter().copied().collect();
    let missing: Vec<_> = all.difference(&found).collect();
    assert!(
        missing.is_empty(),
        "not all roles mapped; missing={:?}",
        missing
    );
    assert!(unresolved.is_empty(), "unresolved roles: {:?}", unresolved);

    let rest_pose =
        thyllore_avatar_core::humanoid::systems::pose::detect_rest_pose(&mapping, &bones);
    assert_eq!(
        rest_pose,
        thyllore_avatar_core::humanoid::components::rest_pose::RestPose::TPose
    );
}

#[test]
fn test_humanoid_rest_rotations_are_identity() {
    let (bones, skeleton) = load_rig_from_fbx_text("test_humanoid", &write_test_humanoid_fbx());

    for (i, bone) in skeleton.bones.iter().enumerate() {
        if HumanoidRole::from_unity_name(&bones[i].name).is_none() {
            continue;
        }
        let q = bone.world_rotation;
        let diff_pos = (q.s - 1.0).abs() + q.v.x.abs() + q.v.y.abs() + q.v.z.abs();
        let diff_neg = (q.s + 1.0).abs() + q.v.x.abs() + q.v.y.abs() + q.v.z.abs();
        assert!(
            diff_pos < 1e-5 || diff_neg < 1e-5,
            "bone '{}' (index {}) world_rotation {:?} is not identity",
            bones[i].name,
            i,
            q
        );
    }
}

#[test]
fn extra_bones_have_unique_names_without_dots() {
    let bones = extra_bones();

    assert_eq!(
        bones.len(),
        39,
        "expected 39 extra bones, got {}",
        bones.len()
    );

    let names: Vec<_> = bones.iter().map(|b| b.name.as_str()).collect();
    let unique: BTreeSet<_> = names.iter().copied().collect();
    assert_eq!(unique.len(), names.len(), "duplicate names found");

    for name in &names {
        assert!(!name.contains('.'), "bone name '{}' contains a dot", name);
    }

    for role in HumanoidRole::ALL {
        let unity_name = role.unity_name();
        assert!(
            !names.contains(&unity_name),
            "extra bone name '{}' matches HumanoidRole {:?} unity_name",
            unity_name,
            role
        );
    }

    for (i, bone) in bones.iter().enumerate() {
        if let ExtraParent::Row(parent_row) = bone.parent {
            assert!(
                parent_row < i,
                "bone at index {} has Row({}) parent which is not before it",
                i,
                parent_row
            );
        }
    }
}
