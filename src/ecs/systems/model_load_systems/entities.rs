use crate::asset::{AssetStorage, MeshAsset};
use crate::ecs::component::{
    AnimationMeta, ClipSchedule, EditorDisplay, EntityIcon, ModelSource, MorphWeights,
};
use crate::ecs::resource::AnimationType;
use crate::ecs::systems::{
    attach_morph_channel_catalogs, build_initial_clip_schedule, despawn_entity_with_clip,
};
use crate::ecs::world::{Animator, Entity, Name, Transform, Visibility, Visible, World};
use crate::hooks::scene::entities_with;
use thyllore_anim_core::editable::SourceClipId;
use thyllore_anim_core::{classify_imported_animation, ImportedAnimationTraits};
use thyllore_importer_core::{LoadedMesh, ModelLoadResult};
use thyllore_vulkan_core::resource::GraphicsResources;

pub(super) const GENERATED_MODEL_NAME: &str = "Generated Mesh";

/// One scene holds one model: the entity being loaded replaces every other model entity.
pub(super) fn despawn_other_model_source_entities(world: &mut World, kept_entity: Entity) {
    for entity in entities_with::<ModelSource>(world) {
        if entity != kept_entity {
            despawn_entity_with_clip(world, entity);
        }
    }
}

pub(super) fn despawn_model_source_entities(world: &mut World) {
    for entity in entities_with::<ModelSource>(world) {
        despawn_entity_with_clip(world, entity);
    }
}

pub(super) fn find_model_source_entity(world: &World) -> Option<Entity> {
    entities_with::<ModelSource>(world).into_iter().next()
}

#[cfg(feature = "auto-rig")]
pub(super) fn spawn_model_root_entity(name: &str, world: &mut World) -> Entity {
    world
        .entity()
        .with_name(name)
        .with_transform(Transform::default())
        .build()
}

/// Turns `model_entity` into the parent of the uploaded meshes and gives it the model's animation.
pub(super) fn attach_model_entities(
    model_entity: Entity,
    first_clip_id: Option<SourceClipId>,
    graphics: &GraphicsResources,
    world: &mut World,
    assets: &mut AssetStorage,
    load_result: &ModelLoadResult,
) {
    let name = find_entity_name(world, model_entity);
    let animation_type = classify_imported_animation(ImportedAnimationTraits {
        has_skinned_meshes: load_result.has_skinned_meshes,
        has_clips: !load_result.clips.is_empty(),
    });
    log!(
        "[ModelLoad] type={:?}, has_skinned={}, node_anim_scale={}",
        animation_type,
        load_result.has_skinned_meshes,
        load_result.node_animation_scale
    );

    world.insert_component(model_entity, Visible(Visibility::from(true)));
    world.insert_component(
        model_entity,
        EditorDisplay::new(EntityIcon::Model).with_expanded(true),
    );
    if first_clip_id.is_some() || animation_type == AnimationType::Skeletal {
        let schedule = world
            .get_component::<ClipSchedule>(model_entity)
            .cloned()
            .unwrap_or_else(|| build_initial_clip_schedule(first_clip_id, world));
        world.insert_component(model_entity, Animator::new());
        world.insert_component(model_entity, schedule);
        world.insert_component(
            model_entity,
            AnimationMeta {
                animation_type,
                node_animation_scale: load_result.node_animation_scale,
            },
        );
    }
    log!(
        "Attached model to entity '{}': entity_id={}",
        name,
        model_entity
    );

    spawn_mesh_entities(
        &mesh_entity_names(&name, &load_result.meshes, 0),
        graphics,
        world,
        assets,
        model_entity,
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
}

fn find_entity_name(world: &World, entity: Entity) -> String {
    world
        .get_component::<Name>(entity)
        .map(|name| name.0.clone())
        .unwrap_or_default()
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

pub(super) fn mesh_entity_names(
    model_name: &str,
    meshes: &[LoadedMesh],
    first_mesh_index: usize,
) -> Vec<String> {
    meshes
        .iter()
        .enumerate()
        .map(|(offset, mesh)| {
            mesh.name
                .clone()
                .unwrap_or_else(|| format!("{}_{:02}", model_name, first_mesh_index + offset + 1))
        })
        .collect()
}

pub(super) fn spawn_mesh_entities(
    entity_names: &[String],
    graphics: &GraphicsResources,
    world: &mut World,
    assets: &mut AssetStorage,
    parent_entity: Entity,
    mesh_range: std::ops::Range<usize>,
) {
    let first_mesh_index = mesh_range.start;
    let mut morph_entities = Vec::new();
    for mesh_idx in mesh_range {
        let mesh = &graphics.meshes[mesh_idx];
        let entity_name = entity_names[mesh_idx - first_mesh_index].clone();

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

        let morph = &graphics.meshes[mesh_idx].morph;
        if !morph.channels.is_empty() {
            world.insert_component(entity, MorphWeights::from_defaults(morph));
            morph_entities.push(entity);
        }

        log!(
            "Created mesh entity {} (asset_id={}) for mesh {}: entity_id={}, parent={}",
            entity_name,
            asset_id,
            mesh_idx,
            entity,
            parent_entity
        );
    }

    attach_morph_channel_catalogs(world, assets, graphics, &morph_entities);
}

pub(super) fn model_display_name(model_path: &str, fallback: &str) -> String {
    std::path::Path::new(model_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(fallback)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::resource::ClipLibrary;
    use crate::ecs::systems::{
        clip_library_register_and_activate, find_entity_clip_id, schedule_entity_clip,
    };
    use crate::hooks::scene::spawn_scene_owner;
    use thyllore_anim_core::editable::EditableAnimationClip;

    fn spawn_model_with_clip(world: &mut World, assets: &mut AssetStorage, name: &str) -> Entity {
        let entity = spawn_scene_owner(world, name, ModelSource::new(name));
        let clip_id = clip_library_register_and_activate(
            &mut world.resource_mut::<ClipLibrary>(),
            assets,
            EditableAnimationClip::new(0, name.to_string()),
        );
        schedule_entity_clip(world, entity, clip_id);
        entity
    }

    fn world_with_two_models() -> (World, Entity, Entity) {
        let mut world = World::new();
        let mut assets = AssetStorage::new();
        world.insert_resource(ClipLibrary::new());
        let previous = spawn_model_with_clip(&mut world, &mut assets, "previous");
        let loading = spawn_model_with_clip(&mut world, &mut assets, "loading");
        (world, previous, loading)
    }

    #[test]
    fn replaced_model_entity_leaves_with_its_scheduled_clip() {
        let (mut world, previous, loading) = world_with_two_models();
        let previous_clip = find_entity_clip_id(&world, previous).expect("previous clip");
        let loading_clip = find_entity_clip_id(&world, loading).expect("loading clip");

        despawn_other_model_source_entities(&mut world, loading);

        assert_eq!(entities_with::<ModelSource>(&world), vec![loading]);
        assert_eq!(find_entity_clip_id(&world, loading), Some(loading_clip));
        let library = world.resource::<ClipLibrary>();
        assert!(library.get(previous_clip).is_none());
        assert!(library.get(loading_clip).is_some());
    }

    #[test]
    fn despawning_every_model_entity_removes_their_scheduled_clips() {
        let (mut world, _, _) = world_with_two_models();

        despawn_model_source_entities(&mut world);

        assert!(entities_with::<ModelSource>(&world).is_empty());
        assert_eq!(world.resource::<ClipLibrary>().clip_count(), 0);
    }
}
