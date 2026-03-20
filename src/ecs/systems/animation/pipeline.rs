use std::collections::HashMap;

use cgmath::Matrix4;

use crate::animation::{BoneId, BoneLocalPose, SkeletonId};
use crate::asset::AssetStorage;
use crate::ecs::resource::{AnimationType, ClipLibrary};
use crate::ecs::world::{Animator, World};
use crate::ecs::{apply_pose_overrides, compute_pose_global_transforms};
use crate::vulkanr::resource::graphics_resource::{GraphicsResources, NodeData};

use super::apply::{
    apply_morph_animation, apply_node_animation_to_single_mesh, apply_skinning_to_single_mesh,
    build_node_based_bone_transforms, compute_node_global_transforms, merge_updated_indices,
};
use super::collect::collect_animated_entities;
use super::evaluate::evaluate_entity_blend;
use super::post_process::{compute_spring_bone_result, find_shared_constraints};
use super::{AnimatedEntityInfo, AnimationEvalResult};
use crate::ecs::systems::constraint_solve_systems::apply_constraints;

pub fn run_animation_pipeline(
    world: &World,
    graphics: &mut GraphicsResources,
    nodes: &mut [NodeData],
    clip_library: &ClipLibrary,
    assets: &AssetStorage,
    dt: f32,
    pose_overrides: &HashMap<BoneId, BoneLocalPose>,
) -> AnimationEvalResult {
    let entity_infos = collect_animated_entities(world, graphics, clip_library, assets);

    let first_time = world
        .iter_components::<Animator>()
        .next()
        .map(|(_, a)| a.time)
        .unwrap_or(0.0);

    let morph_updated = if !clip_library.morph_animation.is_empty() {
        apply_morph_animation(graphics, &clip_library.morph_animation, first_time)
    } else {
        Vec::new()
    };

    if entity_infos.is_empty() {
        return AnimationEvalResult {
            updated_meshes: morph_updated,
            bone_transforms: None,
        };
    }

    let (anim_updated, bone_transforms) = apply_blended_animations(
        &entity_infos,
        world,
        graphics,
        nodes,
        assets,
        dt,
        pose_overrides,
    );

    AnimationEvalResult {
        updated_meshes: merge_updated_indices(morph_updated, anim_updated),
        bone_transforms,
    }
}

fn apply_blended_animations(
    entities: &[AnimatedEntityInfo],
    world: &World,
    graphics: &mut GraphicsResources,
    nodes: &mut [NodeData],
    assets: &AssetStorage,
    dt: f32,
    pose_overrides: &HashMap<BoneId, BoneLocalPose>,
) -> (
    Vec<usize>,
    Option<(SkeletonId, Vec<Matrix4<f32>>, AnimationType)>,
) {
    let mut updated = Vec::new();
    let mut first_bone_transforms: Option<(SkeletonId, Vec<Matrix4<f32>>, AnimationType)> = None;

    let shared_constraints = find_shared_constraints(entities, world);

    let spring_result = compute_spring_bone_result(
        entities,
        world,
        assets,
        &shared_constraints,
        pose_overrides,
        dt,
    );

    for info in entities {
        let Some(skeleton) = assets.get_skeleton_by_skeleton_id(info.skeleton_id) else {
            continue;
        };

        let has_spring = spring_result
            .as_ref()
            .map_or(false, |(skel_id, _, _)| *skel_id == info.skeleton_id);

        let (globals, _pose) = if has_spring {
            let (_, ref cached_globals, ref cached_pose) = spring_result
                .as_ref()
                .expect("has_spring is true so spring_result is Some");

            if info.animation_type == AnimationType::Node {
                compute_node_global_transforms(nodes, skeleton, cached_pose);
            }

            (cached_globals.clone(), None)
        } else {
            let Some(mut pose) = evaluate_entity_blend(info, assets) else {
                continue;
            };

            if let Some(ref cs) = shared_constraints {
                apply_constraints(cs, skeleton, &mut pose);
            }

            if !pose_overrides.is_empty() {
                apply_pose_overrides(&mut pose, pose_overrides);
            }

            if info.animation_type == AnimationType::Node {
                compute_node_global_transforms(nodes, skeleton, &pose);
            }

            let globals = compute_pose_global_transforms(skeleton, &pose);

            (globals, Some(pose))
        };

        if first_bone_transforms.is_none() {
            let gizmo_transforms = if info.animation_type == AnimationType::Node {
                build_node_based_bone_transforms(nodes, skeleton)
            } else {
                globals.clone()
            };
            first_bone_transforms = Some((
                info.skeleton_id,
                gizmo_transforms,
                info.animation_type.clone(),
            ));
        }

        let mesh_updated = match info.animation_type {
            AnimationType::Node => apply_node_animation_to_single_mesh(
                graphics,
                info.mesh_idx,
                nodes,
                info.node_animation_scale,
            ),
            _ => apply_skinning_to_single_mesh(graphics, info.mesh_idx, &globals, skeleton),
        };

        if mesh_updated && !updated.contains(&info.mesh_idx) {
            updated.push(info.mesh_idx);
        }
    }

    (updated, first_bone_transforms)
}
