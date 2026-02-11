use crate::animation::{BoneId, Skeleton};
use crate::ecs::component::{
    ColliderShape, SpringBoneSetup, SpringChain, SpringChainId, SpringColliderDef,
    SpringColliderGroup, SpringColliderGroupId, SpringColliderId, SpringJointParam,
};
use crate::ecs::resource::SpringBoneState;
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

fn reset_spring_bone_state(world: &mut World) {
    if let Some(mut state) = world.get_resource_mut::<SpringBoneState>() {
        state.initialized = false;
    }
}
