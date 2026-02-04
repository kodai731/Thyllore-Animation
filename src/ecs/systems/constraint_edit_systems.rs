use crate::animation::{
    AimConstraintData, BoneId, ConstraintId, ConstraintType,
    IkConstraintData, ParentConstraintData,
    PositionConstraintData, RotationConstraintData,
    ScaleConstraintData, Skeleton,
};
use crate::asset::AssetStorage;
use crate::debugview::gizmo::ConstraintGizmoData;
use crate::ecs::component::{Constrained, ConstraintSet};
use crate::ecs::world::{Entity, World};

pub fn handle_constraint_add(
    world: &mut World,
    entity: Entity,
    type_index: u8,
    assets: &AssetStorage,
) {
    let skeleton = first_skeleton(assets);
    let (bone_a, bone_b) =
        pick_default_bones(skeleton.as_ref());
    let constraint =
        create_default_constraint(type_index, bone_a, bone_b);
    let priority = constraint.default_priority();

    crate::log!(
        "[ConstraintAdd] entity={:?} type={} bones=({},{}) priority={}",
        entity, type_index, bone_a, bone_b, priority
    );

    let has_set = world
        .get_component::<ConstraintSet>(entity)
        .is_some();

    if !has_set {
        world.insert_component(entity, ConstraintSet::new());
        world.insert_component(entity, Constrained);
    }

    if let Some(set) =
        world.get_component_mut::<ConstraintSet>(entity)
    {
        set.add_constraint(constraint, priority);
    }

    if let Some(mut gizmo) =
        world.get_resource_mut::<ConstraintGizmoData>()
    {
        gizmo.visible = true;
    }
}

pub fn handle_constraint_remove(
    world: &mut World,
    entity: Entity,
    id: ConstraintId,
) {
    let is_empty = {
        let set =
            world.get_component_mut::<ConstraintSet>(entity);
        if let Some(set) = set {
            set.remove_constraint(id);
            set.constraints.is_empty()
        } else {
            return;
        }
    };

    if is_empty {
        world.remove_component::<ConstraintSet>(entity);
        world.remove_component::<Constrained>(entity);

        if let Some(mut gizmo) =
            world.get_resource_mut::<ConstraintGizmoData>()
        {
            gizmo.visible = false;
        }
    }
}

pub fn handle_constraint_update(
    world: &mut World,
    entity: Entity,
    id: ConstraintId,
    constraint: &ConstraintType,
) {
    if let Some(set) =
        world.get_component_mut::<ConstraintSet>(entity)
    {
        if let Some(entry) = set.find_constraint_mut(id) {
            entry.constraint = constraint.clone();
        }
    }
}

fn first_skeleton(
    assets: &AssetStorage,
) -> Option<Skeleton> {
    assets
        .skeletons
        .values()
        .next()
        .map(|s| s.skeleton.clone())
}

fn pick_default_bones(
    skeleton: Option<&Skeleton>,
) -> (BoneId, BoneId) {
    let skeleton = match skeleton {
        Some(s) if s.bones.len() >= 2 => s,
        _ => return (0, 0),
    };

    let spine = find_bone_by_patterns(
        skeleton,
        &["spine", "chest"],
    );
    let hips = find_bone_by_patterns(
        skeleton,
        &["hips", "root", "pelvis"],
    );

    if let (Some(a), Some(b)) = (spine, hips) {
        if a != b {
            return (a, b);
        }
    }

    let head = find_bone_by_patterns(
        skeleton,
        &["head", "neck"],
    );
    let root_child = find_root_child(skeleton);

    if let (Some(a), Some(b)) = (head, root_child) {
        if a != b {
            return (a, b);
        }
    }

    if let Some((parent, child)) =
        find_mid_hierarchy_pair(skeleton)
    {
        return (child, parent);
    }

    let root_child_id = find_root_child(skeleton);
    let child = root_child_id.unwrap_or(1);
    let grandchild = skeleton
        .get_bone(child)
        .and_then(|b| b.children.first().copied())
        .unwrap_or(child);

    if grandchild != child {
        (grandchild, child)
    } else {
        (child, 0)
    }
}

fn find_bone_by_patterns(
    skeleton: &Skeleton,
    patterns: &[&str],
) -> Option<BoneId> {
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

fn find_root_child(skeleton: &Skeleton) -> Option<BoneId> {
    for &root_id in &skeleton.root_bone_ids {
        if let Some(root) = skeleton.get_bone(root_id) {
            if let Some(&first_child) = root.children.first()
            {
                return Some(first_child);
            }
        }
    }
    None
}

fn find_mid_hierarchy_pair(
    skeleton: &Skeleton,
) -> Option<(BoneId, BoneId)> {
    let bone_count = skeleton.bones.len();
    if bone_count < 3 {
        return None;
    }

    let mid = bone_count / 2;
    let mid_bone = &skeleton.bones[mid];

    if let Some(&child) = mid_bone.children.first() {
        return Some((mid_bone.id, child));
    }

    if let Some(parent_id) = mid_bone.parent_id {
        return Some((parent_id, mid_bone.id));
    }

    None
}

fn create_default_constraint(
    type_index: u8,
    bone_a: BoneId,
    bone_b: BoneId,
) -> ConstraintType {
    match type_index {
        0 => ConstraintType::Ik(IkConstraintData {
            effector_bone: bone_b,
            target_bone: bone_a,
            ..Default::default()
        }),
        1 => ConstraintType::Aim(AimConstraintData {
            source_bone: bone_a,
            target_bone: bone_b,
            ..Default::default()
        }),
        2 => ConstraintType::Parent(ParentConstraintData {
            constrained_bone: bone_a,
            sources: vec![(bone_b, 1.0)],
            ..Default::default()
        }),
        3 => ConstraintType::Position(PositionConstraintData {
            constrained_bone: bone_a,
            target_bone: bone_b,
            ..Default::default()
        }),
        4 => ConstraintType::Rotation(RotationConstraintData {
            constrained_bone: bone_a,
            target_bone: bone_b,
            ..Default::default()
        }),
        5 => ConstraintType::Scale(ScaleConstraintData {
            constrained_bone: bone_a,
            target_bone: bone_b,
            ..Default::default()
        }),
        _ => ConstraintType::Position(PositionConstraintData {
            constrained_bone: bone_a,
            target_bone: bone_b,
            ..Default::default()
        }),
    }
}
