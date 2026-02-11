use crate::animation::{
    AimConstraintData, ConstraintId, ConstraintType, IkConstraintData, ParentConstraintData,
    PositionConstraintData, RotationConstraintData, ScaleConstraintData,
};
use crate::debugview::gizmo::ConstraintGizmoData;
use crate::ecs::component::{Constrained, ConstraintSet};
use crate::ecs::world::{Entity, World};

pub fn handle_constraint_add(world: &mut World, entity: Entity, type_index: u8) {
    let constraint = create_default_constraint(type_index);
    let priority = constraint.default_priority();

    crate::log!(
        "[ConstraintAdd] entity={:?} type={} priority={}",
        entity,
        type_index,
        priority
    );

    let has_set = world.get_component::<ConstraintSet>(entity).is_some();

    if !has_set {
        world.insert_component(entity, ConstraintSet::new());
        world.insert_component(entity, Constrained);
    }

    if let Some(set) = world.get_component_mut::<ConstraintSet>(entity) {
        super::constraint_set_systems::constraint_set_add(set, constraint, priority);
    }

    if let Some(mut gizmo) = world.get_resource_mut::<ConstraintGizmoData>() {
        gizmo.visible = true;
    }
}

pub fn handle_constraint_remove(world: &mut World, entity: Entity, id: ConstraintId) {
    let is_empty = {
        let set = world.get_component_mut::<ConstraintSet>(entity);
        if let Some(set) = set {
            super::constraint_set_systems::constraint_set_remove(set, id);
            set.constraints.is_empty()
        } else {
            return;
        }
    };

    if is_empty {
        world.remove_component::<ConstraintSet>(entity);
        world.remove_component::<Constrained>(entity);

        if let Some(mut gizmo) = world.get_resource_mut::<ConstraintGizmoData>() {
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
    if let Some(set) = world.get_component_mut::<ConstraintSet>(entity) {
        if let Some(entry) = super::constraint_set_systems::constraint_set_find_mut(set, id) {
            entry.constraint = constraint.clone();
        }
    }
}

fn create_default_constraint(type_index: u8) -> ConstraintType {
    match type_index {
        0 => ConstraintType::Ik(IkConstraintData::default()),
        1 => ConstraintType::Aim(AimConstraintData::default()),
        2 => ConstraintType::Parent(ParentConstraintData::default()),
        3 => ConstraintType::Position(PositionConstraintData::default()),
        4 => ConstraintType::Rotation(RotationConstraintData::default()),
        5 => ConstraintType::Scale(ScaleConstraintData::default()),
        _ => ConstraintType::Position(PositionConstraintData::default()),
    }
}
