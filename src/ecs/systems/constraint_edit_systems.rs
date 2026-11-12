use crate::animation::{
    AimConstraintData, BoneId, ConstraintId, ConstraintType, IkConstraintData,
    ParentConstraintData, PositionConstraintData, RotationConstraintData, ScaleConstraintData,
};
use crate::ecs::component::{Constrained, ConstraintSet};
use crate::ecs::resource::gizmo::ConstraintGizmoData;
use crate::ecs::world::{Entity, World};

pub fn handle_constraint_add(
    world: &mut World,
    entity: Entity,
    type_index: u8,
    constrained_bone: Option<BoneId>,
) {
    let Some(mut constraint) = create_default_constraint(type_index) else {
        return;
    };
    if let Some(bone) = constrained_bone {
        constraint.set_constrained_bone_id(bone);
    }
    let priority = constraint.default_priority();

    log!(
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

fn create_default_constraint(type_index: u8) -> Option<ConstraintType> {
    match type_index {
        0 => Some(ConstraintType::Ik(IkConstraintData::default())),
        1 => Some(ConstraintType::Aim(AimConstraintData::default())),
        2 => Some(ConstraintType::Parent(ParentConstraintData::default())),
        3 => Some(ConstraintType::Position(PositionConstraintData::default())),
        4 => Some(ConstraintType::Rotation(RotationConstraintData::default())),
        5 => Some(ConstraintType::Scale(ScaleConstraintData::default())),
        _ => {
            log!(
                "Invalid constraint type index: {}. Valid range: 0-5",
                type_index
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn added_constraint(constrained_bone: Option<BoneId>) -> ConstraintType {
        let mut world = World::new();
        let entity = world.entity().with_name("rig").build();
        handle_constraint_add(&mut world, entity, 3, constrained_bone);
        let set = world
            .get_component::<ConstraintSet>(entity)
            .expect("ConstraintSet inserted on first add");
        set.constraints[0].constraint.clone()
    }

    #[test]
    fn add_applies_the_requested_constrained_bone() {
        assert_eq!(added_constraint(Some(9)).constrained_bone_id(), 9);
    }

    #[test]
    fn add_without_a_bone_keeps_the_type_default() {
        let default_bone = PositionConstraintData::default().constrained_bone;
        assert_eq!(added_constraint(None).constrained_bone_id(), default_bone);
    }
}
