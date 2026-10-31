use crate::animation::{spring_bone::set_chain_collider_groups, BoneId, Skeleton};
use crate::ecs::component::{
    apply_ramp, ChainRamp, ColliderGroupOrigin, ColliderShape, RampEnd, RampScope, SpringBoneSetup,
    SpringChain, SpringChainId, SpringColliderDef, SpringColliderGroup, SpringColliderGroupId,
    SpringColliderId, SpringJointParam,
};
use crate::ecs::resource::{SpringBoneInspectorState, SpringBoneState};
use crate::ecs::world::{Entity, World};

pub fn handle_spring_chain_add(
    world: &mut World,
    entity: Entity,
    root_bone_id: BoneId,
    chain_length: u32,
    skeleton: &Skeleton,
) {
    let Some(setup) = world.get_component_mut::<SpringBoneSetup>(entity) else {
        return;
    };

    thyllore_anim_core::spring_bone::add_chain(setup, root_bone_id, chain_length, skeleton);

    reset_spring_bone_state(world);
}

pub fn handle_spring_chain_remove(world: &mut World, entity: Entity, chain_id: SpringChainId) {
    let Some(setup) = world.get_component_mut::<SpringBoneSetup>(entity) else {
        return;
    };
    setup.chains.retain(|c| c.id != chain_id);

    reset_spring_bone_state(world);
}

pub fn handle_spring_chain_update(
    world: &mut World,
    entity: Entity,
    chain_id: SpringChainId,
    chain: SpringChain,
) {
    let Some(setup) = world.get_component_mut::<SpringBoneSetup>(entity) else {
        return;
    };
    if let Some(existing) = setup.chains.iter_mut().find(|c| c.id == chain_id) {
        *existing = chain;
    }

    reset_spring_bone_state(world);
}

pub fn handle_spring_joint_update(
    world: &mut World,
    entity: Entity,
    chain_id: SpringChainId,
    joint_index: usize,
    joint: SpringJointParam,
) {
    let Some(setup) = world.get_component_mut::<SpringBoneSetup>(entity) else {
        return;
    };
    if let Some(chain) = setup.chains.iter_mut().find(|c| c.id == chain_id) {
        if joint_index < chain.joints.len() {
            chain.joints[joint_index] = joint;
        }
    }

    reset_spring_bone_state(world);
}

pub fn handle_spring_collider_add(
    world: &mut World,
    entity: Entity,
    bone_id: BoneId,
    shape: ColliderShape,
) {
    let Some(setup) = world.get_component_mut::<SpringBoneSetup>(entity) else {
        return;
    };

    let collider_id = setup.next_collider_id;
    setup.next_collider_id += 1;

    let collider = SpringColliderDef {
        id: collider_id,
        bone_id,
        offset: cgmath::Vector3::new(0.0, 0.0, 0.0),
        shape,
    };
    setup.colliders.push(collider);

    reset_spring_bone_state(world);
}

pub fn handle_spring_collider_remove(
    world: &mut World,
    entity: Entity,
    collider_id: SpringColliderId,
) {
    let Some(setup) = world.get_component_mut::<SpringBoneSetup>(entity) else {
        return;
    };
    setup.colliders.retain(|c| c.id != collider_id);

    for group in &mut setup.collider_groups {
        group.collider_ids.retain(|&id| id != collider_id);
    }

    reset_spring_bone_state(world);
}

pub fn handle_spring_collider_update(
    world: &mut World,
    entity: Entity,
    collider_id: SpringColliderId,
    collider: SpringColliderDef,
) {
    let Some(setup) = world.get_component_mut::<SpringBoneSetup>(entity) else {
        return;
    };
    if let Some(existing) = setup.colliders.iter_mut().find(|c| c.id == collider_id) {
        *existing = collider;
    }

    reset_spring_bone_state(world);
}

pub fn handle_spring_collider_group_add(world: &mut World, entity: Entity, name: String) {
    let Some(setup) = world.get_component_mut::<SpringBoneSetup>(entity) else {
        return;
    };

    let group_id = setup.next_group_id;
    setup.next_group_id += 1;

    let group = SpringColliderGroup {
        id: group_id,
        name,
        collider_ids: Vec::new(),
        origin: ColliderGroupOrigin::Authored,
    };
    setup.collider_groups.push(group);

    reset_spring_bone_state(world);
}

pub fn handle_spring_collider_group_remove(
    world: &mut World,
    entity: Entity,
    group_id: SpringColliderGroupId,
) {
    let Some(setup) = world.get_component_mut::<SpringBoneSetup>(entity) else {
        return;
    };
    setup.collider_groups.retain(|g| g.id != group_id);

    for chain in &mut setup.chains {
        chain.collider_group_ids.retain(|&id| id != group_id);
    }

    reset_spring_bone_state(world);
}

pub fn handle_spring_collider_group_update(
    world: &mut World,
    entity: Entity,
    group_id: SpringColliderGroupId,
    group: SpringColliderGroup,
) {
    let Some(setup) = world.get_component_mut::<SpringBoneSetup>(entity) else {
        return;
    };
    if let Some(existing) = setup.collider_groups.iter_mut().find(|g| g.id == group_id) {
        *existing = group;
    }

    reset_spring_bone_state(world);
}

pub(crate) fn reset_spring_bone_state(world: &mut World) {
    if let Some(mut state) = world.get_resource_mut::<SpringBoneState>() {
        state.initialized = false;
    }
}

pub fn handle_spring_chain_ramp_apply(
    world: &mut World,
    entity: Entity,
    chain_ids: &[SpringChainId],
    ramp: &ChainRamp,
) {
    let Some(setup) = world.get_component_mut::<SpringBoneSetup>(entity) else {
        return;
    };
    let ramped: Result<Vec<SpringChain>, _> = setup
        .chains
        .iter()
        .filter(|chain| chain_ids.contains(&chain.id))
        .map(|chain| {
            apply_ramp(chain, ramp)
                .map_err(|mismatch| format!("chain {:?}: {mismatch}", chain.name))
        })
        .collect();
    let ramped = match ramped {
        Ok(ramped) => ramped,
        Err(reason) => {
            msg_warn!("Chain ramp not applied: {reason}; load the ends from a chain first");
            return;
        }
    };
    for new_chain in ramped {
        if let Some(chain) = setup
            .chains
            .iter_mut()
            .find(|chain| chain.id == new_chain.id)
        {
            *chain = new_chain;
        }
    }

    reset_spring_bone_state(world);
}

pub fn handle_spring_chain_ramp_load(world: &mut World, entity: Entity, chain_id: SpringChainId) {
    let Some(setup) = world.get_component::<SpringBoneSetup>(entity) else {
        return;
    };
    let Some(chain) = setup.chains.iter().find(|c| c.id == chain_id) else {
        return;
    };
    if let Some(mut state) = world.get_resource_mut::<SpringBoneInspectorState>() {
        state.ramp.load_ends(chain);
    }
}

pub fn handle_spring_chain_ramp_form(world: &mut World, ramp: ChainRamp, scope: RampScope) {
    if let Some(mut state) = world.get_resource_mut::<SpringBoneInspectorState>() {
        state.ramp = ramp;
        state.ramp_scope = scope;
    }
}

pub fn handle_spring_chain_ramp_set_end(world: &mut World, end: RampEnd, joint: SpringJointParam) {
    if let Some(mut state) = world.get_resource_mut::<SpringBoneInspectorState>() {
        state.ramp.set_end(end, joint);
    }
}

pub fn handle_spring_chain_collider_groups_apply(
    world: &mut World,
    entity: Entity,
    chain_ids: &[SpringChainId],
    group_ids: &[SpringColliderGroupId],
) {
    let Some(setup) = world.get_component_mut::<SpringBoneSetup>(entity) else {
        return;
    };
    set_chain_collider_groups(setup, chain_ids, group_ids);

    reset_spring_bone_state(world);
}

pub fn handle_spring_chain_collider_groups_load(
    world: &mut World,
    entity: Entity,
    chain_id: SpringChainId,
) {
    let Some(setup) = world.get_component::<SpringBoneSetup>(entity) else {
        return;
    };
    let Some(chain) = setup.chains.iter().find(|c| c.id == chain_id) else {
        return;
    };
    let group_ids = chain.collider_group_ids.clone();
    if let Some(mut state) = world.get_resource_mut::<SpringBoneInspectorState>() {
        state.chain_collider_groups = group_ids;
    }
}

pub fn handle_spring_chain_collider_groups_form(
    world: &mut World,
    group_ids: Vec<SpringColliderGroupId>,
    scope: RampScope,
) {
    if let Some(mut state) = world.get_resource_mut::<SpringBoneInspectorState>() {
        state.chain_collider_groups = group_ids;
        state.chain_collider_scope = scope;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::spring_bone::{JointForce, SpringForceModel, VrmJointForce};

    fn make_chain(id: SpringChainId) -> SpringChain {
        let first_bone = id * 3 + 1;
        SpringChain {
            id,
            joints: (first_bone..first_bone + 3)
                .map(|bone_id| SpringJointParam {
                    bone_id,
                    ..SpringJointParam::for_force_model(SpringForceModel::Vrm)
                })
                .collect(),
            ..Default::default()
        }
    }

    fn make_world_with_three_chains() -> (World, Entity) {
        let mut world = World::new();
        world.insert_resource(SpringBoneInspectorState::default());
        let entity = world.entity().with_name("test").build();
        let setup = SpringBoneSetup {
            chains: (0..3).map(make_chain).collect(),
            next_chain_id: 3,
            ..Default::default()
        };
        world.insert_component(entity, setup);
        (world, entity)
    }

    fn make_ramp() -> ChainRamp {
        ChainRamp {
            root: SpringJointParam {
                force: JointForce::Vrm(VrmJointForce {
                    stiffness: 2.0,
                    gravity_power: 0.0,
                }),
                ..Default::default()
            },
            tip: SpringJointParam {
                force: JointForce::Vrm(VrmJointForce {
                    stiffness: 0.25,
                    gravity_power: 0.0,
                }),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn chain_joints(world: &World, entity: Entity, index: usize) -> Vec<SpringJointParam> {
        world
            .get_component::<SpringBoneSetup>(entity)
            .unwrap()
            .chains[index]
            .joints
            .clone()
    }

    #[test]
    fn ramp_apply_changes_only_listed_chains() {
        let (mut world, entity) = make_world_with_three_chains();
        let ramp = make_ramp();
        let untouched_before = chain_joints(&world, entity, 2);

        handle_spring_chain_ramp_apply(&mut world, entity, &[0, 1], &ramp);

        let setup = world.get_component::<SpringBoneSetup>(entity).unwrap();
        for chain in &setup.chains[..2] {
            assert_eq!(
                chain.joints,
                apply_ramp(&make_chain(chain.id), &ramp)
                    .expect("one force model")
                    .joints
            );
            assert_eq!(chain.joints[0].force.magnitudes()[0], 2.0);
            assert_eq!(chain.joints[2].force.magnitudes()[0], 0.25);
        }
        assert_eq!(chain_joints(&world, entity, 2), untouched_before);
    }

    #[test]
    fn ramp_of_another_force_model_leaves_every_chain_untouched() {
        let (mut world, entity) = make_world_with_three_chains();
        let before: Vec<_> = (0..3).map(|i| chain_joints(&world, entity, i)).collect();

        handle_spring_chain_ramp_apply(&mut world, entity, &[0, 1, 2], &ChainRamp::default());

        let after: Vec<_> = (0..3).map(|i| chain_joints(&world, entity, i)).collect();
        assert_eq!(after, before);
    }

    #[test]
    fn ramp_apply_ignores_unknown_ids() {
        let (mut world, entity) = make_world_with_three_chains();
        let before: Vec<_> = (0..3).map(|i| chain_joints(&world, entity, i)).collect();

        handle_spring_chain_ramp_apply(&mut world, entity, &[99], &make_ramp());

        let after: Vec<_> = (0..3).map(|i| chain_joints(&world, entity, i)).collect();
        assert_eq!(after, before);
    }

    #[test]
    fn ramp_load_takes_first_and_last_joint() {
        let (mut world, entity) = make_world_with_three_chains();

        handle_spring_chain_ramp_load(&mut world, entity, 1);

        let joints = chain_joints(&world, entity, 1);
        let state = world.resource::<SpringBoneInspectorState>();
        assert_eq!(state.ramp.root, joints[0]);
        assert_eq!(state.ramp.tip, joints[2]);
    }

    #[test]
    fn ramp_form_writes_ramp_and_scope() {
        let (mut world, _) = make_world_with_three_chains();
        let ramp = make_ramp();

        handle_spring_chain_ramp_form(&mut world, ramp.clone(), RampScope::AllChains);

        let state = world.resource::<SpringBoneInspectorState>();
        assert_eq!(state.ramp, ramp);
        assert_eq!(state.ramp_scope, RampScope::AllChains);
    }

    fn chain_group_ids(world: &World, entity: Entity, index: usize) -> Vec<SpringColliderGroupId> {
        world
            .get_component::<SpringBoneSetup>(entity)
            .unwrap()
            .chains[index]
            .collider_group_ids
            .clone()
    }

    fn add_group(world: &mut World, entity: Entity, id: SpringColliderGroupId) {
        let mut setup = world.get_component_mut::<SpringBoneSetup>(entity).unwrap();
        setup.collider_groups.push(SpringColliderGroup {
            id,
            name: format!("group_{id}"),
            collider_ids: Vec::new(),
            origin: ColliderGroupOrigin::Authored,
        });
    }

    #[test]
    fn collider_groups_apply_replaces_groups_of_listed_chains_only() {
        let (mut world, entity) = make_world_with_three_chains();
        add_group(&mut world, entity, 7);
        add_group(&mut world, entity, 8);

        handle_spring_chain_collider_groups_apply(&mut world, entity, &[0, 2], &[7, 8]);
        handle_spring_chain_collider_groups_apply(&mut world, entity, &[2], &[8]);

        assert_eq!(chain_group_ids(&world, entity, 0), vec![7, 8]);
        assert_eq!(
            chain_group_ids(&world, entity, 1),
            Vec::<SpringColliderGroupId>::new()
        );
        assert_eq!(chain_group_ids(&world, entity, 2), vec![8]);
    }

    #[test]
    fn collider_groups_apply_drops_unknown_group_ids() {
        let (mut world, entity) = make_world_with_three_chains();
        add_group(&mut world, entity, 7);

        handle_spring_chain_collider_groups_apply(&mut world, entity, &[1], &[7, 99]);

        assert_eq!(chain_group_ids(&world, entity, 1), vec![7]);
    }

    #[test]
    fn collider_groups_load_and_form_write_the_inspector_state() {
        let (mut world, entity) = make_world_with_three_chains();
        add_group(&mut world, entity, 7);
        handle_spring_chain_collider_groups_apply(&mut world, entity, &[1], &[7]);

        handle_spring_chain_collider_groups_load(&mut world, entity, 1);
        assert_eq!(
            world
                .resource::<SpringBoneInspectorState>()
                .chain_collider_groups,
            vec![7]
        );

        handle_spring_chain_collider_groups_form(&mut world, Vec::new(), RampScope::AllChains);
        let state = world.resource::<SpringBoneInspectorState>();
        assert!(state.chain_collider_groups.is_empty());
        assert_eq!(state.chain_collider_scope, RampScope::AllChains);
    }

    #[test]
    fn ramp_set_end_writes_only_that_end() {
        let (mut world, _) = make_world_with_three_chains();
        let tip = SpringJointParam {
            force: JointForce::Vrm(VrmJointForce {
                stiffness: 0.1,
                gravity_power: 0.0,
            }),
            ..Default::default()
        };

        handle_spring_chain_ramp_set_end(&mut world, RampEnd::Tip, tip.clone());

        let state = world.resource::<SpringBoneInspectorState>();
        assert_eq!(state.ramp.tip, tip);
        assert_eq!(state.ramp.root, SpringJointParam::default());
    }
}
