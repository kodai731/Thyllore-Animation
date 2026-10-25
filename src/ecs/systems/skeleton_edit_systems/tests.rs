use super::*;
use cgmath::Matrix4;

fn build_skin(influences: &[[(u32, f32); 4]]) -> SkinData {
    SkinData {
        skeleton_id: 0,
        bone_indices: influences
            .iter()
            .map(|v| Vector4::new(v[0].0, v[1].0, v[2].0, v[3].0))
            .collect(),
        bone_weights: influences
            .iter()
            .map(|v| Vector4::new(v[0].1, v[1].1, v[2].1, v[3].1))
            .collect(),
        base_positions: vec![Vector3::new(0.0, 0.0, 0.0); influences.len()],
        base_normals: vec![Vector3::new(0.0, 1.0, 0.0); influences.len()],
    }
}

fn remove_middle_of_three() -> (Vec<BoneId>, BoneRemap, Option<BoneId>) {
    let mut skeleton = Skeleton::new("s");
    let root = skeleton.add_bone("root", None);
    let middle = skeleton.add_bone("middle", Some(root));
    skeleton.add_bone("tip", Some(middle));
    skeleton.add_bone("side", Some(root));
    for bone in &mut skeleton.bones {
        bone.inverse_bind_pose = Matrix4::from_scale(1.0);
    }
    let removed = skeleton.remove_bone(middle).unwrap();
    (removed.removed, removed.remap, removed.parent)
}

#[test]
fn removed_subtree_weight_moves_to_its_parent_and_merges() {
    let (removed, remap, parent) = remove_middle_of_three();
    let mut skin = build_skin(&[
        [(0, 0.5), (1, 0.3), (2, 0.2), (0, 0.0)],
        [(2, 1.0), (0, 0.0), (0, 0.0), (0, 0.0)],
        [(3, 0.5), (1, 0.5), (0, 0.0), (0, 0.0)],
    ]);

    merge_removed_bone_weights(&mut skin, &removed, parent, &remap);

    assert_eq!(skin.bone_indices[0], Vector4::new(0, 0, 0, 0));
    assert!((skin.bone_weights[0].x - 1.0).abs() < 1e-6);
    assert_eq!(skin.bone_indices[1], Vector4::new(0, 0, 0, 0));
    assert!((skin.bone_weights[1].x - 1.0).abs() < 1e-6);
    assert_eq!(skin.bone_indices[2], Vector4::new(0, 1, 0, 0));
    assert!((skin.bone_weights[2].x - 0.5).abs() < 1e-6);
    assert!((skin.bone_weights[2].y - 0.5).abs() < 1e-6);
}

#[test]
fn removed_root_weight_is_dropped_and_the_rest_renormalized() {
    let mut skeleton = Skeleton::new("s");
    let root = skeleton.add_bone("root", None);
    skeleton.add_bone("child", Some(root));
    skeleton.add_bone("other", None);
    for bone in &mut skeleton.bones {
        bone.inverse_bind_pose = Matrix4::from_scale(1.0);
    }
    let removed = skeleton.remove_bone(root).unwrap();
    let mut skin = build_skin(&[[(0, 0.25), (1, 0.25), (2, 0.5), (0, 0.0)]]);

    merge_removed_bone_weights(&mut skin, &removed.removed, removed.parent, &removed.remap);

    assert_eq!(skin.bone_indices[0], Vector4::new(0, 0, 0, 0));
    assert!((skin.bone_weights[0].x - 1.0).abs() < 1e-6);
}

#[test]
fn restore_owned_components_removes_setups_the_snapshot_does_not_have() {
    let mut world = World::new();
    let created_by_edit = world.spawn();
    world.insert_component(created_by_edit, SpringBoneSetup::default());
    world.insert_component(created_by_edit, WithSpringBone);

    restore_owned_components::<SpringBoneSetup, WithSpringBone>(&mut world, Vec::new());

    assert!(!world.has_component::<SpringBoneSetup>(created_by_edit));
    assert!(!world.has_component::<WithSpringBone>(created_by_edit));
}

#[test]
fn restore_owned_components_reinserts_setups_the_snapshot_has() {
    let mut world = World::new();
    let owner = world.spawn();
    let mut setup = SpringBoneSetup::default();
    setup.next_chain_id = 7;

    restore_owned_components::<SpringBoneSetup, WithSpringBone>(&mut world, vec![(owner, setup)]);

    assert_eq!(
        world
            .get_component::<SpringBoneSetup>(owner)
            .map(|s| s.next_chain_id),
        Some(7)
    );
    assert!(world.has_component::<WithSpringBone>(owner));
}

#[test]
fn restore_skin_weights_writes_the_snapshot_back_when_sizes_match() {
    let mut skin = build_skin(&[[(0, 1.0), (0, 0.0), (0, 0.0), (0, 0.0)]]);
    let snapshot = SkinSnapshot {
        mesh_index: 0,
        bone_indices: vec![Vector4::new(2, 1, 0, 0)],
        bone_weights: vec![Vector4::new(0.5, 0.5, 0.0, 0.0)],
    };

    restore_skin_weights(&mut skin, &snapshot);

    assert_eq!(skin.bone_indices[0], Vector4::new(2, 1, 0, 0));
    assert_eq!(skin.bone_weights[0], Vector4::new(0.5, 0.5, 0.0, 0.0));
}

#[test]
fn constraints_that_lose_a_bone_are_dropped_and_others_renumbered() {
    let (_, remap, _) = remove_middle_of_three();
    let mut kept = ConstraintType::Position(crate::animation::PositionConstraintData {
        constrained_bone: 3,
        target_bone: 0,
        ..Default::default()
    });
    let mut dropped = ConstraintType::Aim(crate::animation::AimConstraintData {
        source_bone: 1,
        target_bone: 2,
        ..Default::default()
    });

    assert!(remap_constraint(&mut kept, &remap));
    assert!(!remap_constraint(&mut dropped, &remap));
    match kept {
        ConstraintType::Position(data) => assert_eq!(data.constrained_bone, 1),
        _ => unreachable!(),
    }
}

#[test]
fn spring_chains_drop_removed_joints_and_empty_chains() {
    let (_, remap, _) = remove_middle_of_three();
    let mut setup = SpringBoneSetup::default();
    setup.chains.push(crate::ecs::component::SpringChain {
        joints: vec![
            crate::ecs::component::SpringJointParam {
                bone_id: 1,
                ..Default::default()
            },
            crate::ecs::component::SpringJointParam {
                bone_id: 3,
                ..Default::default()
            },
        ],
        ..Default::default()
    });
    setup.chains.push(crate::ecs::component::SpringChain {
        joints: vec![crate::ecs::component::SpringJointParam {
            bone_id: 1,
            ..Default::default()
        }],
        ..Default::default()
    });

    remap_spring_setup(&mut setup, &remap);

    assert_eq!(setup.chains.len(), 1);
    assert_eq!(setup.chains[0].joints.len(), 1);
    assert_eq!(setup.chains[0].joints[0].bone_id, 1);
}
