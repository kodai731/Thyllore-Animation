use super::clips::{build_initial_clip_schedule, register_loaded_clips};
use crate::asset::{AssetStorage, MeshAsset};
use crate::ecs::component::{AnimationMeta, EntityIcon};
use crate::ecs::resource::AnimationType;
use crate::ecs::world::{Animator, Entity, Transform, World};
use crate::loader::ModelLoadResult;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

pub(super) fn spawn_model_entities(
    model_name: &str,
    graphics: &GraphicsResources,
    world: &mut World,
    assets: &mut AssetStorage,
    load_result: &ModelLoadResult,
    scene_will_provide_clips: bool,
) -> Entity {
    let name = model_display_name(model_name, "model");
    let animation_type = determine_animation_type(load_result);
    log!(
        "[ModelLoad] type={:?}, has_skinned={}, node_anim_scale={}",
        animation_type,
        load_result.has_skinned_meshes,
        load_result.node_animation_scale
    );

    let first_clip_id =
        register_loaded_clips(world, assets, &load_result.clips, scene_will_provide_clips);
    let initial_schedule = build_initial_clip_schedule(first_clip_id, world);

    let mut parent_builder = world
        .entity()
        .with_name(&name)
        .with_transform(Transform::default())
        .with_visible(true)
        .with_editor_display(EntityIcon::Model, true);
    if first_clip_id.is_some() {
        parent_builder = parent_builder
            .with_animator(Animator::new())
            .with_clip_schedule(initial_schedule)
            .with_animation_meta(AnimationMeta {
                animation_type,
                node_animation_scale: load_result.node_animation_scale,
            });
    }
    let parent_entity = parent_builder.build();
    log!(
        "Created parent entity '{}': entity_id={}",
        name,
        parent_entity
    );

    spawn_mesh_entities(
        &name,
        graphics,
        world,
        assets,
        parent_entity,
        0..graphics.meshes.len(),
    );

    log!(
        "Created {} ECS entities, {} mesh assets, {} skeletons, {} clips, {} nodes",
        world.entity_count(),
        assets.meshes.len(),
        assets.skeletons.len(),
        assets.animation_clips.len(),
        assets.nodes.len()
    );

    parent_entity
}

pub(super) fn spawn_model_part_entity(part_name: &str, world: &mut World) -> Entity {
    let parent_entity = world
        .entity()
        .with_name(part_name)
        .with_transform(Transform::default())
        .with_visible(true)
        .with_editor_display(EntityIcon::Model, true)
        .build();

    log!(
        "Created additive parent entity '{}': entity_id={}",
        part_name,
        parent_entity
    );
    parent_entity
}

pub(super) fn spawn_mesh_entities(
    name: &str,
    graphics: &GraphicsResources,
    world: &mut World,
    assets: &mut AssetStorage,
    parent_entity: Entity,
    mesh_range: std::ops::Range<usize>,
) {
    for mesh_idx in mesh_range {
        let mesh = &graphics.meshes[mesh_idx];
        let entity_name = format!("{}_{:02}", name, mesh_idx + 1);

        let asset_id = assets.add_mesh(MeshAsset {
            id: 0,
            name: entity_name.clone(),
            graphics_mesh_index: mesh_idx,
            object_index: mesh.object_index,
            material_id: graphics.mesh_material_ids.get(mesh_idx).copied(),
            skeleton_id: mesh.skeleton_id,
            node_index: mesh.node_index,
            render_to_gbuffer: mesh.render_to_gbuffer,
        });

        let entity = world
            .entity()
            .with_name(&entity_name)
            .with_global_transform()
            .with_visible(true)
            .with_parent(parent_entity)
            .with_editor_display(EntityIcon::Mesh, false)
            .with_mesh(asset_id, mesh.object_index)
            .build();

        log!(
            "Created mesh entity {} (asset_id={}) for mesh {}: entity_id={}, parent={}",
            entity_name,
            asset_id,
            mesh_idx,
            entity,
            parent_entity
        );
    }
}

pub(super) fn model_display_name(model_path: &str, fallback: &str) -> String {
    std::path::Path::new(model_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(fallback)
        .to_string()
}

fn determine_animation_type(load_result: &ModelLoadResult) -> AnimationType {
    if load_result.has_skinned_meshes {
        AnimationType::Skeletal
    } else if !load_result.clips.is_empty() {
        AnimationType::Node
    } else {
        AnimationType::None
    }
}
