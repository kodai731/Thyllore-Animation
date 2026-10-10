#![cfg(test)]
mod support;

use cgmath::{InnerSpace, Quaternion, Vector3};

use support::canonical_bones;
use support::rig_convention::rig_convention;
use support::rig_names::CONVENTIONS;
use support::rig_nodes::{
    bone_direction, bone_local_axis, build_rig_nodes, first_child_bone_index,
};

#[test]
fn node_counts() {
    assert_eq!(build_rig_nodes(&rig_convention("blender")).len(), 25);
    assert_eq!(build_rig_nodes(&rig_convention("blender_apose")).len(), 25);
    assert_eq!(build_rig_nodes(&rig_convention("maya")).len(), 19);
    assert_eq!(build_rig_nodes(&rig_convention("mixamo")).len(), 19);
    assert_eq!(build_rig_nodes(&rig_convention("max_biped")).len(), 20);
    assert_eq!(build_rig_nodes(&rig_convention("unreal")).len(), 20);
    assert_eq!(build_rig_nodes(&rig_convention("vrm_normalized")).len(), 19);
    assert_eq!(build_rig_nodes(&rig_convention("adversarial")).len(), 19);
}

#[test]
fn role_node_positions_match_file_positions() {
    for &convention_id in &CONVENTIONS {
        let convention = rig_convention(convention_id);
        let nodes = build_rig_nodes(&convention);
        let positions = support::rig_positions::file_positions(&convention);
        let bones = canonical_bones();

        for (i, bone) in bones.iter().enumerate() {
            let node_index = nodes
                .iter()
                .position(|n| n.role == Some(bone.role))
                .unwrap();
            let node = &nodes[node_index];
            let expected = &positions[i];
            assert!(
                (node.world_position.x - expected[0]).abs() < 1e-6,
                "{convention_id} bone {} x: got {}, expected {}",
                bone.role,
                node.world_position.x,
                expected[0]
            );
            assert!(
                (node.world_position.y - expected[1]).abs() < 1e-6,
                "{convention_id} bone {} y: got {}, expected {}",
                bone.role,
                node.world_position.y,
                expected[1]
            );
            assert!(
                (node.world_position.z - expected[2]).abs() < 1e-6,
                "{convention_id} bone {} z: got {}, expected {}",
                bone.role,
                node.world_position.z,
                expected[2]
            );
        }
    }
}

#[test]
fn along_bone_axis_aligned_with_direction() {
    let conventions: &[&str] = &["maya", "mixamo"];
    for &convention_id in conventions {
        let convention = rig_convention(convention_id);
        let nodes = build_rig_nodes(&convention);
        let bones = canonical_bones();

        for (i, bone) in bones.iter().enumerate() {
            let node_index = nodes
                .iter()
                .position(|n| n.role == Some(bone.role))
                .unwrap();
            let node = &nodes[node_index];
            let axis = bone_local_axis(&convention, i);
            let rotated_axis = node.world_rotation * axis;
            let child_bone_index = first_child_bone_index(&bones, i);
            let positions = support::rig_positions::file_positions(&convention);
            let dir = bone_direction(&positions, i, child_bone_index);
            let dot = rotated_axis.dot(dir);
            assert!(
                dot > 1.0 - 1e-9,
                "{convention_id} bone {} (index {i}): dot={dot}, axis={axis:?}, dir={dir:?}",
                bone.role
            );
        }
    }
}

#[test]
fn random_bone_axis_aligned_with_direction() {
    let convention = rig_convention("adversarial");
    let nodes = build_rig_nodes(&convention);
    let bones = canonical_bones();

    for (i, bone) in bones.iter().enumerate() {
        let node_index = nodes
            .iter()
            .position(|n| n.role == Some(bone.role))
            .unwrap();
        let node = &nodes[node_index];
        let axis = bone_local_axis(&convention, i);
        let rotated_axis = node.world_rotation * axis;
        let child_bone_index = first_child_bone_index(&bones, i);
        let positions = support::rig_positions::file_positions(&convention);
        let dir = bone_direction(&positions, i, child_bone_index);
        let dot = rotated_axis.dot(dir);
        assert!(
            dot > 1.0 - 1e-9,
            "bone {} (index {i}): dot={dot}, axis={axis:?}, dir={dir:?}",
            bone.role
        );
    }
}

#[test]
fn world_aligned_all_bones_have_qc() {
    let convention = rig_convention("vrm_normalized");
    let nodes = build_rig_nodes(&convention);
    let bones = canonical_bones();
    let qc = Quaternion::from(support::rig_positions::character_to_file_rotation(
        convention.character_to_file,
    ));

    for bone in &bones {
        let node_index = nodes
            .iter()
            .position(|n| n.role == Some(bone.role))
            .unwrap();
        let node = &nodes[node_index];
        assert!(
            (node.world_rotation.s - qc.s).abs() < 1e-9
                && (node.world_rotation.v.x - qc.v.x).abs() < 1e-9
                && (node.world_rotation.v.y - qc.v.y).abs() < 1e-9
                && (node.world_rotation.v.z - qc.v.z).abs() < 1e-9,
            "bone {}: got {:?}, expected {:?}",
            bone.role,
            node.world_rotation,
            qc
        );
    }
}

#[test]
fn container_hips_parent() {
    let container_conventions: &[&str] = &["blender", "max_biped", "unreal"];
    for &convention_id in container_conventions {
        let convention = rig_convention(convention_id);
        let nodes = build_rig_nodes(&convention);
        let hips_index = nodes.iter().position(|n| n.role == Some("Hips")).unwrap();
        assert!(
            nodes[hips_index].parent == Some(0),
            "{convention_id}: Hips parent is {:?}, expected Some(0)",
            nodes[hips_index].parent
        );
        let spine_index = nodes.iter().position(|n| n.role == Some("Spine")).unwrap();
        assert_eq!(
            nodes[spine_index].parent,
            Some(hips_index),
            "{convention_id}: Spine parent must be the Hips node"
        );
    }

    let no_container_conventions: &[&str] = &["maya", "mixamo", "vrm_normalized", "adversarial"];
    for &convention_id in no_container_conventions {
        let convention = rig_convention(convention_id);
        let nodes = build_rig_nodes(&convention);
        let hips_index = nodes.iter().position(|n| n.role == Some("Hips")).unwrap();
        assert!(
            nodes[hips_index].parent.is_none(),
            "{convention_id}: Hips parent is {:?}, expected None",
            nodes[hips_index].parent
        );
    }
}

#[test]
fn deterministic() {
    for &convention_id in &CONVENTIONS {
        let convention = rig_convention(convention_id);
        let nodes1 = build_rig_nodes(&convention);
        let nodes2 = build_rig_nodes(&convention);
        assert_eq!(nodes1.len(), nodes2.len());
        for (i, (n1, n2)) in nodes1.iter().zip(nodes2.iter()).enumerate() {
            assert_eq!(n1.name, n2.name, "node {i} name mismatch");
            assert_eq!(n1.parent, n2.parent, "node {i} parent mismatch");
            assert_eq!(n1.role, n2.role, "node {i} role mismatch");
            let diff = n1.world_position - n2.world_position;
            assert!(diff.dot(diff) < 1e-18, "node {i} position mismatch");
            assert!(
                (n1.world_rotation.s - n2.world_rotation.s).abs() < 1e-15
                    && (n1.world_rotation.v.x - n2.world_rotation.v.x).abs() < 1e-15
                    && (n1.world_rotation.v.y - n2.world_rotation.v.y).abs() < 1e-15
                    && (n1.world_rotation.v.z - n2.world_rotation.v.z).abs() < 1e-15,
                "node {i} rotation mismatch"
            );
        }
    }
}

#[test]
fn adversarial_has_at_least_3_bone_axis_types() {
    let convention = rig_convention("adversarial");
    let bones = canonical_bones();
    let mut distinct_axes: Vec<Vector3<f64>> = Vec::new();
    for i in 0..bones.len() {
        let axis = bone_local_axis(&convention, i);
        let mut is_duplicate = false;
        for &existing in &distinct_axes {
            let diff = axis - existing;
            if diff.dot(diff) < 1e-12 {
                is_duplicate = true;
                break;
            }
        }
        if !is_duplicate {
            distinct_axes.push(axis);
        }
    }
    assert!(
        distinct_axes.len() >= 3,
        "adversarial has only {} distinct bone axes, expected at least 3",
        distinct_axes.len()
    );
}
