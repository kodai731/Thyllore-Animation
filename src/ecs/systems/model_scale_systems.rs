use cgmath::Vector3;
use thyllore_anim_core::{
    scale_animation_clip_translations, scale_editable_clip_translations, scale_spring_setup,
    ConstraintType,
};
use thyllore_file_format_core::fbx::scale_fbx_model;
use thyllore_math_core::scale_translation;
use thyllore_model_core::{scale_morph, scale_skeleton, scale_skin, scale_vertex_positions};

use crate::animation::BoneId;
use crate::asset::AssetStorage;
use crate::ecs::component::{ConstraintSet, SpringBoneSetup};
use crate::ecs::resource::{ClipLibrary, FbxModelCache, HumanoidRigState, NodeAssets};
use crate::ecs::systems::humanoid_bake_systems::request_bake_scan;
use crate::ecs::systems::{invalidate_humanoid_rig, refresh_after_skeleton_edit};
use crate::ecs::world::{Entity, MeshRef, Parent, Transform, World};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

const UNIFORM_SCALE_TOLERANCE: f32 = 1e-4;

/// The one uniform factor an entity scale stands for, or why it cannot be baked.
pub fn uniform_scale_factor(scale: Vector3<f32>) -> anyhow::Result<f32> {
    let spread = (scale.x - scale.y).abs().max((scale.x - scale.z).abs());
    if spread > UNIFORM_SCALE_TOLERANCE * scale.x.abs().max(1.0) {
        anyhow::bail!(
            "scale must be uniform to bake into the model, got ({}, {}, {})",
            scale.x,
            scale.y,
            scale.z
        );
    }
    validate_scale_factor(scale.x)?;
    Ok(scale.x)
}

pub fn validate_scale_factor(factor: f32) -> anyhow::Result<()> {
    if !factor.is_finite() || factor <= 0.0 {
        anyhow::bail!("scale factor must be a positive finite number, got {factor}");
    }
    Ok(())
}

/// Bake `factor` into every length of the loaded model and reset the model roots' entity scale to 1.
pub fn apply_model_scale(
    world: &mut World,
    assets: &mut AssetStorage,
    graphics: &mut GraphicsResources,
    factor: f32,
) -> anyhow::Result<()> {
    validate_scale_factor(factor)?;
    if assets.meshes.is_empty() && assets.skeletons.is_empty() {
        anyhow::bail!("no model is loaded");
    }

    scale_assets(assets, factor);
    scale_graphics_meshes(graphics, factor);
    scale_world_data(world, factor);
    reset_model_root_scales(world);

    invalidate_humanoid_rig(world);
    request_bake_scan(world);
    refresh_after_skeleton_edit(world, assets, graphics);
    Ok(())
}

fn scale_assets(assets: &mut AssetStorage, factor: f32) {
    for skeleton_asset in assets.skeletons.values_mut() {
        scale_skeleton(&mut skeleton_asset.skeleton, factor);
    }
    for clip_asset in assets.animation_clips.values_mut() {
        scale_animation_clip_translations(&mut clip_asset.clip, factor);
    }
    for node_asset in assets.nodes.values_mut() {
        node_asset.local_transform = scale_translation(&node_asset.local_transform, factor);
    }
}

fn scale_graphics_meshes(graphics: &mut GraphicsResources, factor: f32) {
    for mesh in &mut graphics.meshes {
        scale_vertex_positions(&mut mesh.vertex_data.vertices, factor);
        scale_vertex_positions(&mut mesh.base_vertices, factor);
        if let Some(skin) = mesh.skin_data.as_mut() {
            scale_skin(skin, factor);
        }
        scale_morph(&mut mesh.morph, factor);
    }
}

fn scale_world_data(world: &mut World, factor: f32) {
    let model_space_bones = model_space_bone_filter(world);

    if let Some(mut cache) = world.get_resource_mut::<FbxModelCache>() {
        if let Some(fbx_model) = cache.fbx_model_mut() {
            scale_fbx_model(fbx_model, factor);
        }
    }
    if let Some(mut node_assets) = world.get_resource_mut::<NodeAssets>() {
        for node in &mut node_assets.nodes {
            node.local_transform = scale_translation(&node.local_transform, factor);
            node.global_transform = scale_translation(&node.global_transform, factor);
        }
    }
    if let Some(mut clip_library) = world.get_resource_mut::<ClipLibrary>() {
        for skeleton in &mut clip_library.animation.skeletons {
            scale_skeleton(skeleton, factor);
        }
        for source in clip_library.source_clips.values_mut() {
            scale_editable_clip_translations(&mut source.editable_clip, factor, |bone| {
                model_space_bones(bone)
            });
        }
    }

    for (_, setup) in world.iter_components_mut::<SpringBoneSetup>() {
        scale_spring_setup(setup, factor);
    }
    for (_, constraint_set) in world.iter_components_mut::<ConstraintSet>() {
        for entry in &mut constraint_set.constraints {
            if let ConstraintType::Position(position) = &mut entry.constraint {
                position.offset *= factor;
            }
        }
    }
}

/// Humanoid tracks hold normalised values that do not depend on the model's size; every other bone is model space.
fn model_space_bone_filter(world: &World) -> impl Fn(BoneId) -> bool {
    let humanoid_bones: Vec<BoneId> = world
        .get_resource::<HumanoidRigState>()
        .and_then(|state| state.rig.as_ref().map(|rig| rig.mapping.clone()))
        .map(|mapping| mapping.by_role.values().map(|&b| b as BoneId).collect())
        .unwrap_or_default();
    move |bone| !humanoid_bones.contains(&bone)
}

/// Entities whose `Transform` scales the loaded meshes: the parents of the mesh entities.
pub fn model_root_entities(world: &World) -> Vec<Entity> {
    let mut roots: Vec<Entity> = world
        .iter_components::<MeshRef>()
        .filter_map(|(entity, _)| world.get_component::<Parent>(entity).map(|parent| parent.0))
        .collect();
    roots.sort_unstable();
    roots.dedup();
    roots
}

/// The entity scale an export has to bake first, or None when the model is already at scale 1.
/// Baking scales every loaded asset by one factor, so model roots of different scales are an error.
pub fn pending_entity_scale(world: &World) -> anyhow::Result<Option<f32>> {
    let mut shared: Option<f32> = None;
    for root in model_root_entities(world) {
        let Some(scale) = world
            .get_component::<Transform>(root)
            .map(|transform| transform.scale)
        else {
            continue;
        };
        let factor = uniform_scale_factor(scale)?;
        match shared {
            None => shared = Some(factor),
            Some(first) if is_same_scale(first, factor) => {}
            Some(first) => anyhow::bail!(
                "model roots have different scales ({first} and {factor}); set them to one scale before baking"
            ),
        }
    }
    Ok(shared.filter(|factor| !is_same_scale(*factor, 1.0)))
}

fn is_same_scale(a: f32, b: f32) -> bool {
    (a - b).abs() <= UNIFORM_SCALE_TOLERANCE * a.abs().max(b.abs()).max(1.0)
}

fn reset_model_root_scales(world: &mut World) {
    for root in model_root_entities(world) {
        if let Some(transform) = world.get_component_mut::<Transform>(root) {
            transform.scale = Vector3::new(1.0, 1.0, 1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_scale_accepts_equal_components_and_rejects_the_rest() {
        assert_eq!(
            uniform_scale_factor(Vector3::new(0.01, 0.01, 0.01)).unwrap(),
            0.01
        );
        assert!(uniform_scale_factor(Vector3::new(1.0, 2.0, 1.0)).is_err());
        assert!(uniform_scale_factor(Vector3::new(0.0, 0.0, 0.0)).is_err());
        assert!(uniform_scale_factor(Vector3::new(-1.0, -1.0, -1.0)).is_err());
    }

    #[test]
    fn pending_entity_scale_reads_the_mesh_parent_and_ignores_scale_one() {
        let mut world = World::new();
        let root = world
            .entity()
            .with_name("model")
            .with_transform(Transform::default())
            .build();
        world
            .entity()
            .with_name("mesh")
            .with_parent(root)
            .with_mesh(0, 0)
            .build();

        assert_eq!(pending_entity_scale(&world).unwrap(), None);

        world.get_component_mut::<Transform>(root).unwrap().scale = Vector3::new(0.01, 0.01, 0.01);
        assert_eq!(pending_entity_scale(&world).unwrap(), Some(0.01));

        world.get_component_mut::<Transform>(root).unwrap().scale = Vector3::new(1.0, 2.0, 1.0);
        assert!(pending_entity_scale(&world).is_err());
    }

    #[test]
    fn pending_entity_scale_rejects_model_roots_of_different_scales() {
        let mut world = World::new();
        for (name, scale) in [("a", 1.0), ("b", 2.0)] {
            let root = world
                .entity()
                .with_name(name)
                .with_transform(Transform {
                    scale: Vector3::new(scale, scale, scale),
                    ..Default::default()
                })
                .build();
            world
                .entity()
                .with_name("mesh")
                .with_parent(root)
                .with_mesh(0, 0)
                .build();
        }

        assert!(pending_entity_scale(&world).is_err());
    }

    #[test]
    fn apply_without_a_model_is_an_error() {
        let mut world = World::new();
        let mut assets = AssetStorage::new();
        let mut graphics = GraphicsResources::default();
        assert!(apply_model_scale(&mut world, &mut assets, &mut graphics, 2.0).is_err());
    }
}
