use crate::animation::{BoneId, ConstraintId, ConstraintType};
use crate::ecs::component::{ConstraintEntry, ConstraintSet};

pub fn constraint_set_add(
    set: &mut ConstraintSet,
    constraint: ConstraintType,
    priority: u32,
) -> ConstraintId {
    let id = set.next_id;
    set.next_id += 1;

    let entry = ConstraintEntry {
        id,
        constraint,
        priority,
    };
    set.constraints.push(entry);
    set.constraints.sort_by_key(|e| e.priority);

    id
}

pub fn constraint_set_remove(set: &mut ConstraintSet, id: ConstraintId) -> bool {
    let before = set.constraints.len();
    set.constraints.retain(|e| e.id != id);
    set.constraints.len() < before
}

pub fn constraint_set_find_by_bone(
    set: &ConstraintSet,
    bone_id: BoneId,
) -> Vec<&ConstraintEntry> {
    set.constraints
        .iter()
        .filter(|e| e.constraint.constrained_bone_id() == bone_id)
        .collect()
}

pub fn constraint_set_find(
    set: &ConstraintSet,
    id: ConstraintId,
) -> Option<&ConstraintEntry> {
    set.constraints.iter().find(|e| e.id == id)
}

pub fn constraint_set_find_mut(
    set: &mut ConstraintSet,
    id: ConstraintId,
) -> Option<&mut ConstraintEntry> {
    set.constraints.iter_mut().find(|e| e.id == id)
}

pub fn constraint_set_enabled(set: &ConstraintSet) -> Vec<&ConstraintEntry> {
    set.constraints
        .iter()
        .filter(|e| e.constraint.is_enabled())
        .collect()
}
