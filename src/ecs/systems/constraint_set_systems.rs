use crate::animation::{BoneId, ConstraintId, ConstraintType};
use crate::asset::AssetStorage;
use crate::ecs::component::{Constrained, ConstraintEntry, ConstraintSet};
use crate::ecs::world::{Animator, World};
use crate::hooks::model_load::LoadedModel;

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

pub fn constraint_set_find_by_bone(set: &ConstraintSet, bone_id: BoneId) -> Vec<&ConstraintEntry> {
    set.constraints
        .iter()
        .filter(|e| e.constraint.constrained_bone_id() == bone_id)
        .collect()
}

pub fn constraint_set_find(set: &ConstraintSet, id: ConstraintId) -> Option<&ConstraintEntry> {
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

pub fn constraint_set_attach_loaded(
    world: &mut World,
    _assets: &AssetStorage,
    loaded: &LoadedModel,
) {
    let constraints = &loaded.load_result.constraints;
    if constraints.is_empty() || !world.has_component::<Animator>(loaded.entity) {
        return;
    }

    let mut constraint_set = ConstraintSet::new();
    for constraint in constraints {
        constraint_set_add(
            &mut constraint_set,
            constraint.constraint_type.clone(),
            constraint.priority,
        );
    }

    world.insert_component(loaded.entity, constraint_set);
    world.insert_component(loaded.entity, Constrained);

    log!(
        "Applied {} constraints to entity {}",
        constraints.len(),
        loaded.entity
    );
}

crate::model_load_hook!("constraint_set", Rig, constraint_set_attach_loaded);
