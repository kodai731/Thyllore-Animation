use std::collections::HashMap;

use cgmath::Matrix4;

use crate::animation::{BoneId, BoneLocalPose, SkeletonId, SkeletonPose};
use crate::asset::AssetStorage;
use crate::ecs::component::{ConstraintSet, SpringBoneSetup, WithSpringBone};
use crate::ecs::resource::{SpringBoneMode, SpringBoneState};
use crate::ecs::systems::spring_bone_systems::spring_space_for_entity;
use crate::ecs::world::World;
use crate::ecs::{apply_pose_overrides, compute_pose_global_transforms, create_pose_from_rest};
use thyllore_anim_core::spring_bone::SpringSpace;

use super::evaluate::evaluate_entity_blend;
use super::AnimatedEntityInfo;
use crate::ecs::systems::constraint_solve_systems::apply_constraints;

pub(crate) fn find_shared_constraints(
    entities: &[AnimatedEntityInfo],
    world: &World,
) -> Option<ConstraintSet> {
    entities.iter().find_map(|info| {
        world
            .get_component::<ConstraintSet>(info.entity)
            .map(|cs| cs.clone())
    })
}

pub(crate) fn compute_spring_bone_result(
    entities: &[AnimatedEntityInfo],
    world: &World,
    assets: &AssetStorage,
    shared_constraints: &Option<ConstraintSet>,
    pose_overrides: &HashMap<BoneId, BoneLocalPose>,
    dt: f32,
) -> Option<(SkeletonId, Vec<Matrix4<f32>>, SkeletonPose)> {
    let info = entities
        .iter()
        .find(|e| world.has_component::<WithSpringBone>(e.entity))?;

    let setup = world.get_component::<SpringBoneSetup>(info.entity)?;
    let setup_clone = setup.clone();
    let mut pose = evaluate_entity_blend(info, assets)?;
    let skeleton = assets.get_skeleton_by_skeleton_id(info.skeleton_id)?;

    if let Some(ref cs) = shared_constraints {
        apply_constraints(cs, skeleton, &mut pose);
    }

    if !pose_overrides.is_empty() {
        apply_pose_overrides(&mut pose, pose_overrides);
    }

    let mut globals = compute_pose_global_transforms(skeleton, &pose);

    let space = spring_space_for_entity(world, info.entity);
    apply_spring_bone_simulation(
        world,
        &setup_clone,
        skeleton,
        &mut globals,
        &mut pose,
        dt,
        space,
    );

    Some((info.skeleton_id, globals, pose))
}

fn apply_spring_bone_simulation(
    world: &World,
    setup: &SpringBoneSetup,
    skeleton: &crate::animation::Skeleton,
    globals: &mut [Matrix4<f32>],
    pose: &mut SkeletonPose,
    dt: f32,
    space: SpringSpace,
) {
    use crate::animation::spring_bone::{
        collect_affected_bone_ids, spring_bone_initialize, spring_bone_reseat_tails,
        spring_bone_update, spring_bone_write_back_to_pose,
    };

    if let Some(mut sb_state) = world.get_resource_mut::<SpringBoneState>() {
        match sb_state.mode {
            SpringBoneMode::Realtime => {
                if !sb_state.initialized {
                    let rest_globals =
                        compute_pose_global_transforms(skeleton, &create_pose_from_rest(skeleton));
                    sb_state.simulation = spring_bone_initialize(setup, skeleton, &rest_globals);
                    sb_state.initialized = true;
                    spring_bone_reseat_tails(setup, &mut sb_state.simulation, skeleton, globals);
                }

                spring_bone_update(
                    setup,
                    &mut sb_state.simulation,
                    skeleton,
                    globals,
                    pose,
                    dt,
                    space,
                );

                let affected_ids = collect_affected_bone_ids(setup);
                spring_bone_write_back_to_pose(skeleton, globals, pose, &affected_ids);
            }
            SpringBoneMode::Baked | SpringBoneMode::BakedOverride => {}
        }
    }
}
