use crate::animation::{BoneId, Skeleton};
use crate::ecs::component::{
    apply_ramp, ChainRamp, ColliderShape, RampEnd, RampScope, SpringBoneSetup, SpringChain,
    SpringChainId, SpringColliderDef, SpringColliderGroup, SpringColliderGroupId, SpringColliderId,
    SpringJointParam,
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

    let chain_id = setup.next_chain_id;
    setup.next_chain_id += 1;

    let joints = build_chain_joints(root_bone_id, chain_length, skeleton);
    let chain = SpringChain {
        id: chain_id,
        name: format!("Chain_{}", chain_id),
        joints,
        collider_group_ids: Vec::new(),
        center_bone_id: None,
        enabled: true,
    };
    setup.chains.push(chain);

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

fn build_chain_joints(
    root_bone_id: BoneId,
    chain_length: u32,
    skeleton: &Skeleton,
) -> Vec<SpringJointParam> {
    let mut joints = Vec::new();
    let mut current_bone = root_bone_id;

    for _ in 0..chain_length {
        joints.push(SpringJointParam {
            bone_id: current_bone,
            ..Default::default()
        });

        let children: Vec<BoneId> = skeleton
            .bones
            .iter()
            .filter(|b| b.parent_id == Some(current_bone))
            .map(|b| b.id)
            .collect();

        if let Some(&first_child) = children.first() {
            current_bone = first_child;
        } else {
            break;
        }
    }

    joints
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
    for chain in &mut setup.chains {
        if chain_ids.contains(&chain.id) {
            *chain = apply_ramp(chain, ramp);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_chain(id: SpringChainId) -> SpringChain {
        let first_bone = id * 3 + 1;
        SpringChain {
            id,
            joints: (first_bone..first_bone + 3)
                .map(|bone_id| SpringJointParam {
                    bone_id,
                    ..Default::default()
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
                stiffness: 2.0,
                ..Default::default()
            },
            tip: SpringJointParam {
                stiffness: 0.25,
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
                apply_ramp(&make_chain(chain.id), &ramp).joints
            );
            assert_eq!(chain.joints[0].stiffness, 2.0);
            assert_eq!(chain.joints[2].stiffness, 0.25);
        }
        assert_eq!(chain_joints(&world, entity, 2), untouched_before);
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

    #[test]
    fn ramp_set_end_writes_only_that_end() {
        let (mut world, _) = make_world_with_three_chains();
        let tip = SpringJointParam {
            stiffness: 0.1,
            ..Default::default()
        };

        handle_spring_chain_ramp_set_end(&mut world, RampEnd::Tip, tip.clone());

        let state = world.resource::<SpringBoneInspectorState>();
        assert_eq!(state.ramp.tip, tip);
        assert_eq!(state.ramp.root, SpringJointParam::default());
    }
}
