use cgmath::Vector3;

use crate::animation::{BoneId, Skeleton};
use crate::asset::storage::AssetStorage;
use crate::ecs::component::{
    SpringBoneSetup, SpringChain, SpringJointParam, WithSpringBone,
};
use crate::ecs::resource::SpringBoneState;
use crate::ecs::world::{Animator, World};

pub fn create_test_spring_bones(world: &mut World, assets: &AssetStorage) {
    let entities = world.component_entities::<Animator>();
    if entities.is_empty() {
        crate::log!("No animated entity found for spring bones");
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

    let chains = detect_spring_bone_chains(skeleton);
    if chains.is_empty() {
        crate::log!("No suitable bone chains detected for spring bones");
        return;
    }

    let mut setup = SpringBoneSetup::default();
    for (chain_idx, chain_bones) in chains.iter().enumerate() {
        let joints: Vec<SpringJointParam> = chain_bones
            .iter()
            .map(|&bone_id| SpringJointParam {
                bone_id,
                stiffness: 0.5,
                drag_force: 0.7,
                gravity_power: 1.0,
                gravity_dir: Vector3::new(0.0, -1.0, 0.0),
                hit_radius: 0.02,
            })
            .collect();

        let chain_name = format_chain_name(skeleton, chain_bones);

        setup.chains.push(SpringChain {
            id: chain_idx as u32,
            name: chain_name,
            joints,
            enabled: true,
            ..Default::default()
        });

        setup.next_chain_id = (chain_idx + 1) as u32;
    }

    crate::log!(
        "Created {} spring bone chains for entity {:?}:",
        setup.chains.len(),
        first_entity
    );
    for chain in &setup.chains {
        let bone_names: Vec<String> = chain
            .joints
            .iter()
            .map(|j| bone_name(skeleton, j.bone_id))
            .collect();
        crate::log!("  Chain '{}': [{}]", chain.name, bone_names.join(" -> "));
    }

    world.insert_component(first_entity, setup);
    world.insert_component(first_entity, WithSpringBone);
    world.insert_resource(SpringBoneState::default());
}

pub fn clear_spring_bones(world: &mut World) {
    let entities = world.component_entities::<WithSpringBone>();
    if entities.is_empty() {
        crate::log!("No spring bone entities to clear");
        return;
    }

    for entity in entities {
        world.remove_component::<SpringBoneSetup>(entity);
        world.remove_component::<WithSpringBone>(entity);
        crate::log!(
            "Cleared spring bones from entity {:?}",
            entity
        );
    }

    if world.contains_resource::<SpringBoneState>() {
        world.insert_resource(SpringBoneState::default());
    }
}

fn detect_spring_bone_chains(skeleton: &Skeleton) -> Vec<Vec<BoneId>> {
    crate::log!(
        "[SpringBone] Detecting chains from skeleton '{}' ({} bones)",
        skeleton.name,
        skeleton.bones.len()
    );

    let mut chains = Vec::new();
    let mut used_bone_ids = std::collections::HashSet::new();

    for bone in &skeleton.bones {
        if !bone.children.is_empty() {
            continue;
        }

        let depth = compute_bone_depth(skeleton, bone.id);
        crate::log!(
            "[SpringBone]   Leaf bone '{}' (id={}): depth={}",
            bone.name,
            bone.id,
            depth
        );

        if depth < 3 {
            crate::log!("[SpringBone]     -> skipped (depth < 3)");
            continue;
        }

        if is_near_root(skeleton, bone.id) {
            crate::log!("[SpringBone]     -> skipped (near root)");
            continue;
        }

        let chain = build_chain_from_leaf(skeleton, bone.id, 5);
        let unique_chain: Vec<BoneId> = chain
            .into_iter()
            .filter(|id| !used_bone_ids.contains(id))
            .collect();

        if unique_chain.len() >= 2 {
            for &id in &unique_chain {
                used_bone_ids.insert(id);
            }

            let names: Vec<String> = unique_chain
                .iter()
                .map(|&id| bone_name(skeleton, id))
                .collect();
            crate::log!(
                "[SpringBone]     -> chain detected ({} bones): [{}]",
                unique_chain.len(),
                names.join(" -> ")
            );
            chains.push(unique_chain);
        } else {
            crate::log!(
                "[SpringBone]     -> skipped (all bones already used by other chains)"
            );
        }
    }

    crate::log!("[SpringBone] Total chains detected: {}", chains.len());
    chains
}

fn build_chain_from_leaf(
    skeleton: &Skeleton,
    leaf_id: BoneId,
    max_length: usize,
) -> Vec<BoneId> {
    let mut chain = vec![leaf_id];
    let mut current = leaf_id;

    for _ in 0..max_length - 1 {
        let Some(bone) = skeleton.get_bone(current) else {
            break;
        };
        let Some(parent_id) = bone.parent_id else {
            break;
        };

        if is_near_root(skeleton, parent_id) {
            break;
        }

        chain.push(parent_id);
        current = parent_id;
    }

    chain.reverse();
    chain
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

fn is_near_root(skeleton: &Skeleton, bone_id: BoneId) -> bool {
    compute_bone_depth(skeleton, bone_id) <= 1
}

fn format_chain_name(skeleton: &Skeleton, bones: &[BoneId]) -> String {
    if bones.is_empty() {
        return String::from("empty");
    }

    let first = bone_name(skeleton, bones[0]);
    let last = bone_name(skeleton, *bones.last().unwrap());

    if bones.len() == 1 {
        first
    } else {
        format!("{} -> {}", first, last)
    }
}

fn bone_name(skeleton: &Skeleton, id: BoneId) -> String {
    skeleton
        .get_bone(id)
        .map(|b| b.name.clone())
        .unwrap_or_else(|| format!("bone#{}", id))
}
