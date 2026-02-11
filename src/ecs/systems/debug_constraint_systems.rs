use cgmath::Vector3;

use crate::animation::{
    AimConstraintData, BoneId, ConstraintType, IkConstraintData, PositionConstraintData, Skeleton,
    PRIORITY_AIM, PRIORITY_IK, PRIORITY_POSITION,
};
use crate::asset::storage::AssetStorage;
use crate::debugview::gizmo::ConstraintGizmoData;
use crate::ecs::component::{Constrained, ConstraintSet};
use crate::ecs::world::{Animator, Entity, World};

pub fn create_test_constraints(world: &mut World, assets: &AssetStorage) {
    let entities = world.component_entities::<Animator>();
    if entities.is_empty() {
        crate::log!("No animated entity found for test constraints");
        return;
    }

    let skeleton = match assets.skeletons.values().next() {
        Some(asset) => &asset.skeleton,
        None => {
            crate::log!("No skeleton found in AssetStorage");
            return;
        }
    };

    let first_entity = entities[0];
    create_constraints_for_entity(world, skeleton, first_entity);

    if world.contains_resource::<ConstraintGizmoData>() {
        let mut cg = world.resource_mut::<ConstraintGizmoData>();
        cg.visible = true;
    }
}

pub fn clear_test_constraints(world: &mut World) {
    let entities = world.component_entities::<Constrained>();
    if entities.is_empty() {
        crate::log!("No constrained entities to clear");
        return;
    }

    for entity in entities {
        world.remove_component::<ConstraintSet>(entity);
        world.remove_component::<Constrained>(entity);
        crate::log!("Cleared test constraints from entity {:?}", entity);
    }

    if world.contains_resource::<ConstraintGizmoData>() {
        let mut cg = world.resource_mut::<ConstraintGizmoData>();
        cg.visible = false;
    }
}

fn create_constraints_for_entity(world: &mut World, skeleton: &Skeleton, entity: Entity) {
    if skeleton.bones.len() < 2 {
        crate::log!(
            "Skeleton '{}' has only {} bones, creating position-only constraint",
            skeleton.name,
            skeleton.bones.len()
        );
        let mut set = ConstraintSet::new();
        if skeleton.bones.len() == 2 {
            add_position_constraint(&mut set, 0, 1);
        }
        apply_constraint_set(world, entity, set);
        return;
    }

    let mut set = ConstraintSet::new();

    add_ik_constraints(&mut set, skeleton);
    add_aim_constraint(&mut set, skeleton);
    add_position_constraint_from_skeleton(&mut set, skeleton);

    let count = set.constraints.len();
    if count == 0 {
        crate::log!(
            "Could not create any test constraints for skeleton '{}'",
            skeleton.name
        );
        return;
    }

    crate::log!(
        "Created {} test constraints for skeleton '{}' (entity {:?})",
        count,
        skeleton.name,
        entity
    );
    for entry in &set.constraints {
        crate::log!("  {:?} priority={}", entry.constraint, entry.priority);
    }

    apply_constraint_set(world, entity, set);
}

fn apply_constraint_set(world: &mut World, entity: Entity, set: ConstraintSet) {
    world.insert_component(entity, set);
    world.insert_component(entity, Constrained);
}

fn find_bone_by_patterns(skeleton: &Skeleton, patterns: &[&str]) -> Option<BoneId> {
    for bone in &skeleton.bones {
        let lower = bone.name.to_lowercase();
        for pat in patterns {
            if lower.contains(pat) {
                return Some(bone.id);
            }
        }
    }
    None
}

fn find_leaf_bone_with_depth(skeleton: &Skeleton, min_depth: u32) -> Option<BoneId> {
    for bone in &skeleton.bones {
        if !bone.children.is_empty() {
            continue;
        }

        let depth = compute_bone_depth(skeleton, bone.id);
        if depth >= min_depth {
            return Some(bone.id);
        }
    }
    None
}

fn compute_bone_depth(skeleton: &Skeleton, bone_id: BoneId) -> u32 {
    let mut depth = 0;
    let mut current = bone_id;
    while let Some(bone) = skeleton.get_bone(current) {
        match bone.parent_id {
            Some(parent) => {
                depth += 1;
                current = parent;
            }
            None => break,
        }
    }
    depth
}

fn find_root_child(skeleton: &Skeleton) -> Option<BoneId> {
    for &root_id in &skeleton.root_bone_ids {
        if let Some(root) = skeleton.get_bone(root_id) {
            if let Some(&first_child) = root.children.first() {
                return Some(first_child);
            }
        }
    }
    None
}

fn add_ik_constraints(set: &mut ConstraintSet, skeleton: &Skeleton) {
    let effector = find_bone_by_patterns(skeleton, &["hand", "wrist", "foot", "ankle"])
        .or_else(|| find_leaf_bone_with_depth(skeleton, 2));

    let effector_id = match effector {
        Some(id) => id,
        None => return,
    };

    let target_id = match find_ik_target(skeleton, effector_id) {
        Some(id) => id,
        None => return,
    };

    let ik = ConstraintType::Ik(IkConstraintData {
        chain_length: 2,
        target_bone: target_id,
        effector_bone: effector_id,
        pole_vector: Some(Vector3::new(0.0, 1.0, 0.0)),
        enabled: true,
        weight: 1.0,
        ..Default::default()
    });
    super::constraint_set_systems::constraint_set_add(set, ik, PRIORITY_IK);

    crate::log!(
        "  IK: effector={} target={}",
        bone_name(skeleton, effector_id),
        bone_name(skeleton, target_id)
    );
}

fn find_ik_target(skeleton: &Skeleton, effector_id: BoneId) -> Option<BoneId> {
    let bone = skeleton.get_bone(effector_id)?;
    let parent = skeleton.get_bone(bone.parent_id?)?;
    parent.parent_id
}

fn add_aim_constraint(set: &mut ConstraintSet, skeleton: &Skeleton) {
    let source =
        find_bone_by_patterns(skeleton, &["head", "neck"]).or_else(|| find_root_child(skeleton));

    let source_id = match source {
        Some(id) => id,
        None => return,
    };

    let target_id = skeleton.root_bone_ids.first().copied().unwrap_or(0);
    if target_id == source_id {
        return;
    }

    let aim = ConstraintType::Aim(AimConstraintData {
        source_bone: source_id,
        target_bone: target_id,
        aim_axis: Vector3::new(0.0, 0.0, 1.0),
        up_axis: Vector3::new(0.0, 1.0, 0.0),
        enabled: true,
        weight: 1.0,
        ..Default::default()
    });
    super::constraint_set_systems::constraint_set_add(set, aim, PRIORITY_AIM);

    crate::log!(
        "  Aim: source={} target={}",
        bone_name(skeleton, source_id),
        bone_name(skeleton, target_id)
    );
}

fn add_position_constraint_from_skeleton(set: &mut ConstraintSet, skeleton: &Skeleton) {
    let constrained = find_bone_by_patterns(skeleton, &["spine", "chest"])
        .or_else(|| find_second_root_child(skeleton));

    let constrained_id = match constrained {
        Some(id) => id,
        None => return,
    };

    let target = find_bone_by_patterns(skeleton, &["hips", "root", "pelvis"])
        .or_else(|| skeleton.root_bone_ids.first().copied());

    let target_id = match target {
        Some(id) => id,
        None => return,
    };

    if constrained_id == target_id {
        return;
    }

    add_position_constraint(set, constrained_id, target_id);

    crate::log!(
        "  Position: constrained={} target={}",
        bone_name(skeleton, constrained_id),
        bone_name(skeleton, target_id)
    );
}

fn find_second_root_child(skeleton: &Skeleton) -> Option<BoneId> {
    for &root_id in &skeleton.root_bone_ids {
        if let Some(root) = skeleton.get_bone(root_id) {
            if root.children.len() >= 2 {
                return Some(root.children[1]);
            }
        }
    }
    None
}

fn add_position_constraint(set: &mut ConstraintSet, constrained_id: BoneId, target_id: BoneId) {
    let pos = ConstraintType::Position(PositionConstraintData {
        constrained_bone: constrained_id,
        target_bone: target_id,
        offset: Vector3::new(0.0, 0.0, 0.0),
        affect_axes: [true, true, true],
        enabled: true,
        weight: 1.0,
    });
    super::constraint_set_systems::constraint_set_add(set, pos, PRIORITY_POSITION);
}

fn bone_name(skeleton: &Skeleton, id: BoneId) -> String {
    skeleton
        .get_bone(id)
        .map(|b| b.name.clone())
        .unwrap_or_else(|| format!("bone#{}", id))
}
