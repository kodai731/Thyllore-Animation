use std::path::{Path, PathBuf};

use super::entities::model_display_name;
use crate::asset::AssetStorage;
use crate::ecs::component::ModelSource;
use crate::ecs::resource::{SceneLoadCommand, SceneLoadQueue, SceneState};
use crate::ecs::world::{Children, Entity, World};
use crate::hooks::scene::{entities_with, spawn_scene_owner};

/// Adds the entity of a model file to the scene; its meshes are loaded by the next dispatch.
pub fn spawn_model_source(world: &mut World, model_file: &str) -> Entity {
    let assets_dir = world.resource::<SceneState>().assets_dir();
    let source_path = express_relative_to_assets(Path::new(model_file), &assets_dir);

    spawn_scene_owner(
        world,
        &model_display_name(model_file, "model"),
        ModelSource::new(&source_path.to_string_lossy()),
    )
}

/// The file a model entity loads: its path is relative to the scene's assets directory or absolute.
pub fn resolve_model_file(world: &World, model_entity: Entity) -> Option<PathBuf> {
    let source = world.get_component::<ModelSource>(model_entity)?;
    let assets_dir = world.resource::<SceneState>().assets_dir();

    Some(assets_dir.join(&source.path))
}

fn express_relative_to_assets(model_file: &Path, assets_dir: &Path) -> PathBuf {
    let (Ok(canonical_file), Ok(canonical_assets_dir)) =
        (model_file.canonicalize(), assets_dir.canonicalize())
    else {
        return model_file.to_path_buf();
    };

    match canonical_file.strip_prefix(&canonical_assets_dir) {
        Ok(relative) => relative.to_path_buf(),
        Err(_) => canonical_file,
    }
}

fn find_pending_model_entities(world: &World) -> Vec<Entity> {
    entities_with::<ModelSource>(world)
        .into_iter()
        .filter(|entity| world.get_component::<Children>(*entity).is_none())
        .collect()
}

fn request_pending_model_loads(world: &mut World, _: &mut AssetStorage) {
    let pending = find_pending_model_entities(world);
    if pending.is_empty() {
        return;
    }

    let mut scene_loads = world.resource_mut::<SceneLoadQueue>();
    for entity in pending {
        scene_loads.push(SceneLoadCommand::LoadModel { entity });
    }
}

crate::dispatch_prep_hook!("model_load", request_pending_model_loads);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::world_with_scene_hooks;

    fn world_with_load_queue() -> World {
        let mut world = world_with_scene_hooks();
        world.insert_resource(SceneState::new());
        world.insert_resource(SceneLoadQueue::default());
        world
    }

    fn queued_load_entities(world: &World) -> Vec<Entity> {
        world
            .resource_mut::<SceneLoadQueue>()
            .take()
            .into_iter()
            .filter_map(|command| match command {
                SceneLoadCommand::LoadModel { entity } => Some(entity),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn model_source_without_mesh_children_requests_a_load() {
        let mut world = world_with_load_queue();
        let mut assets = AssetStorage::new();
        let model_entity = spawn_model_source(&mut world, "models/avatar/avatar.fbx");

        request_pending_model_loads(&mut world, &mut assets);

        assert_eq!(queued_load_entities(&world), vec![model_entity]);
    }

    #[test]
    fn model_source_with_mesh_children_requests_nothing() {
        let mut world = world_with_load_queue();
        let mut assets = AssetStorage::new();
        let model_entity = spawn_model_source(&mut world, "models/avatar/avatar.fbx");
        world
            .entity()
            .with_name("mesh")
            .with_parent(model_entity)
            .build();

        request_pending_model_loads(&mut world, &mut assets);

        assert!(queued_load_entities(&world).is_empty());
    }

    #[test]
    fn model_file_inside_the_assets_directory_is_stored_relative_to_it() {
        let assets_dir = std::env::temp_dir().join("thyllore_model_source_inside");
        let model_file = assets_dir.join("models/avatar/avatar.fbx");
        std::fs::create_dir_all(model_file.parent().unwrap()).unwrap();
        std::fs::write(&model_file, b"stub").unwrap();

        let source_path = express_relative_to_assets(&model_file, &assets_dir);

        assert_eq!(source_path, PathBuf::from("models/avatar/avatar.fbx"));
        assert_eq!(
            assets_dir.join(&source_path).canonicalize().unwrap(),
            model_file.canonicalize().unwrap()
        );
    }

    #[test]
    fn model_file_outside_the_assets_directory_is_stored_absolute() {
        let root = std::env::temp_dir().join("thyllore_model_source_outside");
        let assets_dir = root.join("assets");
        let model_file = root.join("elsewhere/avatar.fbx");
        std::fs::create_dir_all(&assets_dir).unwrap();
        std::fs::create_dir_all(model_file.parent().unwrap()).unwrap();
        std::fs::write(&model_file, b"stub").unwrap();

        let source_path = express_relative_to_assets(&model_file, &assets_dir);

        assert!(source_path.is_absolute());
        assert_eq!(assets_dir.join(&source_path), source_path);
    }
}
