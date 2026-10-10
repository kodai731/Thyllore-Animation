use std::collections::HashMap;
use std::time::Instant;

use cgmath::Matrix4;

use crate::animation::{BoneId, BoneLocalPose, Skeleton, SkeletonId, SkeletonPose};
use crate::asset::AssetStorage;
use crate::ecs::component::ConstraintSet;
use crate::ecs::resource::{AnimationType, ClipLibrary, PoseApplyCache};
use crate::ecs::world::{Entity, World};
use crate::ecs::{apply_pose_overrides, compute_pose_global_transforms};
use crate::vulkanr::resource::graphics_resource::{GraphicsResources, NodeData};

use super::apply::{
    apply_node_animation_to_single_mesh, apply_skinning_to_single_mesh,
    build_node_based_bone_transforms, compute_node_global_transforms,
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
    pose_apply_cache: &mut PoseApplyCache,
    sub: &mut HashMap<String, f32>,
) -> AnimationEvalResult {
    let t = Instant::now();
    let entity_infos = collect_animated_entities(world, graphics, clip_library, assets);
    sub.insert(
        "pipeline.collect".to_string(),
        t.elapsed().as_secs_f32() * 1000.0,
    );
    if entity_infos.is_empty() {
        return AnimationEvalResult {
            updated_meshes: Vec::new(),
            bone_transforms: None,
        };
    }

    let (updated_meshes, bone_transforms) = apply_blended_animations(
        &entity_infos,
        world,
        graphics,
        nodes,
        assets,
        dt,
        pose_overrides,
        pose_apply_cache,
        sub,
    );

    AnimationEvalResult {
        updated_meshes,
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
    pose_apply_cache: &mut PoseApplyCache,
    sub: &mut HashMap<String, f32>,
) -> (
    Vec<usize>,
    Option<(SkeletonId, Vec<Matrix4<f32>>, AnimationType)>,
) {
    let mut updated = Vec::new();
    let mut first_bone_transforms: Option<(SkeletonId, Vec<Matrix4<f32>>, AnimationType)> = None;

    let shared_constraints = find_shared_constraints(entities, world);

    let t = Instant::now();
    let spring_result = compute_spring_bone_result(
        entities,
        world,
        assets,
        &shared_constraints,
        pose_overrides,
        dt,
    );
    sub.insert(
        "pipeline.spring".to_string(),
        t.elapsed().as_secs_f32() * 1000.0,
    );

    let pose_inputs = SharedPoseInputs {
        assets,
        constraints: &shared_constraints,
        spring_result: &spring_result,
        pose_overrides,
    };
    let mut evaluated_globals: HashMap<(Entity, SkeletonId), Option<Vec<Matrix4<f32>>>> =
        HashMap::new();

    for info in entities {
        let Some(skeleton) = assets.get_skeleton_by_skeleton_id(info.skeleton_id) else {
            continue;
        };

        let key = (info.entity, info.skeleton_id);
        let globals: &Option<Vec<Matrix4<f32>>> = if evaluated_globals.contains_key(&key) {
            evaluated_globals.get(&key).unwrap()
        } else {
            let t = Instant::now();
            let result = evaluate_skeleton_globals(info, skeleton, nodes, &pose_inputs);
            sub.entry("pipeline.pose".to_string())
                .and_modify(|v| *v += t.elapsed().as_secs_f32() * 1000.0)
                .or_insert(t.elapsed().as_secs_f32() * 1000.0);
            evaluated_globals.insert(key, result);
            evaluated_globals.get(&key).unwrap()
        };
        let Some(globals) = globals else {
            continue;
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
            AnimationType::Node => {
                let mesh_ref = &graphics.meshes[info.mesh_idx];
                let node_opt = mesh_ref
                    .node_index
                    .and_then(|idx| nodes.iter().find(|n| n.index == idx));
                if let Some(node) = node_opt {
                    let current_value = (node.global_transform, info.node_animation_scale);
                    if should_skip_node(pose_apply_cache, info.mesh_idx, current_value) {
                        continue;
                    }
                    pose_apply_cache
                        .node_cache
                        .insert(info.mesh_idx, current_value);
                }
                let t = Instant::now();
                let updated = apply_node_animation_to_single_mesh(
                    graphics,
                    info.mesh_idx,
                    nodes,
                    info.node_animation_scale,
                );
                sub.entry("pipeline.node_apply".to_string())
                    .and_modify(|v| *v += t.elapsed().as_secs_f32() * 1000.0)
                    .or_insert(t.elapsed().as_secs_f32() * 1000.0);
                updated
            }
            _ => {
                if should_skip_skinned(pose_apply_cache, info.mesh_idx, globals) {
                    continue;
                }
                pose_apply_cache
                    .skinned_cache
                    .insert(info.mesh_idx, globals.clone());
                let t = Instant::now();
                let updated =
                    apply_skinning_to_single_mesh(graphics, info.mesh_idx, globals, skeleton);
                sub.entry("pipeline.skinning".to_string())
                    .and_modify(|v| *v += t.elapsed().as_secs_f32() * 1000.0)
                    .or_insert(t.elapsed().as_secs_f32() * 1000.0);
                updated
            }
        };

        if mesh_updated && !updated.contains(&info.mesh_idx) {
            updated.push(info.mesh_idx);
        }
    }

    (updated, first_bone_transforms)
}

struct SharedPoseInputs<'a> {
    assets: &'a AssetStorage,
    constraints: &'a Option<ConstraintSet>,
    spring_result: &'a Option<(SkeletonId, Vec<Matrix4<f32>>, SkeletonPose)>,
    pose_overrides: &'a HashMap<BoneId, BoneLocalPose>,
}

fn evaluate_skeleton_globals(
    info: &AnimatedEntityInfo,
    skeleton: &Skeleton,
    nodes: &mut [NodeData],
    inputs: &SharedPoseInputs,
) -> Option<Vec<Matrix4<f32>>> {
    let spring_result = inputs
        .spring_result
        .as_ref()
        .filter(|(skeleton_id, _, _)| *skeleton_id == info.skeleton_id);
    if let Some((_, spring_globals, spring_pose)) = spring_result {
        if info.animation_type == AnimationType::Node {
            compute_node_global_transforms(nodes, skeleton, spring_pose);
        }
        return Some(spring_globals.clone());
    }

    let mut pose = evaluate_entity_blend(info, inputs.assets)?;

    if let Some(constraints) = inputs.constraints {
        apply_constraints(constraints, skeleton, &mut pose);
    }

    if !inputs.pose_overrides.is_empty() {
        apply_pose_overrides(&mut pose, inputs.pose_overrides);
    }

    if info.animation_type == AnimationType::Node {
        compute_node_global_transforms(nodes, skeleton, &pose);
    }

    Some(compute_pose_global_transforms(skeleton, &pose))
}

#[inline]
fn should_skip_skinned(cache: &PoseApplyCache, mesh_idx: usize, globals: &[Matrix4<f32>]) -> bool {
    if let Some(cached) = cache.skinned_cache.get(&mesh_idx) {
        if cached == &globals {
            return true;
        }
    }
    false
}

#[inline]
fn should_skip_node(
    cache: &PoseApplyCache,
    mesh_idx: usize,
    current_value: (Matrix4<f32>, f32),
) -> bool {
    if let Some(cached) = cache.node_cache.get(&mesh_idx) {
        if cached == &current_value {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::resource::PoseApplyCache;
    use cgmath::SquareMatrix;

    fn identity_matrices(count: usize) -> Vec<Matrix4<f32>> {
        (0..count).map(|_| Matrix4::identity()).collect()
    }

    fn translated_matrices(offset: f32, count: usize) -> Vec<Matrix4<f32>> {
        (0..count)
            .map(|i| {
                Matrix4::from_translation(cgmath::Vector3::new(offset * (i as f32 + 1.0), 0.0, 0.0))
            })
            .collect()
    }

    #[test]
    fn test_should_skip_skinned_same_globals() {
        let mut cache = PoseApplyCache::default();
        let globals: Vec<Matrix4<f32>> = identity_matrices(4);
        cache.skinned_cache.insert(0, globals.clone());

        assert!(should_skip_skinned(&cache, 0, &globals));
    }

    #[test]
    fn test_should_skip_skinned_different_globals() {
        let mut cache = PoseApplyCache::default();
        let cached_globals: Vec<Matrix4<f32>> = identity_matrices(4);
        let new_globals: Vec<Matrix4<f32>> = translated_matrices(1.0, 4);
        cache.skinned_cache.insert(0, cached_globals);

        assert!(!should_skip_skinned(&cache, 0, &new_globals));
    }

    #[test]
    fn test_should_skip_skinned_uncached_mesh() {
        let cache = PoseApplyCache::default();
        let globals: Vec<Matrix4<f32>> = identity_matrices(4);

        assert!(!should_skip_skinned(&cache, 0, &globals));
    }

    #[test]
    fn test_should_skip_node_same_value() {
        let mut cache = PoseApplyCache::default();
        let value: (Matrix4<f32>, f32) = (Matrix4::identity(), 1.0);
        cache.node_cache.insert(0, value);

        assert!(should_skip_node(&cache, 0, value));
    }

    #[test]
    fn test_should_skip_node_different_value() {
        let mut cache = PoseApplyCache::default();
        let cached_value: (Matrix4<f32>, f32) = (Matrix4::identity(), 1.0);
        let new_value: (Matrix4<f32>, f32) = (
            Matrix4::from_translation(cgmath::Vector3::new(1.0, 0.0, 0.0)),
            1.0,
        );
        cache.node_cache.insert(0, cached_value);

        assert!(!should_skip_node(&cache, 0, new_value));
    }

    #[test]
    fn test_should_skip_node_uncached_mesh() {
        let cache = PoseApplyCache::default();
        let value: (Matrix4<f32>, f32) = (Matrix4::identity(), 1.0);

        assert!(!should_skip_node(&cache, 0, value));
    }
}
