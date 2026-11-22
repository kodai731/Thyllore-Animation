use std::path::{Path, PathBuf};

use anyhow::Result;

use super::entities::{model_display_name, GENERATED_MODEL_NAME};
use crate::asset::AssetStorage;
use crate::ecs::component::{GeneratedGlb, ModelSource};
use crate::ecs::resource::{SceneLoadCommand, SceneLoadQueue, SceneState};
use crate::ecs::world::{Children, Entity, World};
use crate::hooks::scene::{entities_with, spawn_scene_owner};

/// Adds the entity of a model file to the scene; its meshes are loaded by the next dispatch.
pub fn spawn_model_source(world: &mut World, model_file: &str) -> Entity {
    let assets_dir = find_scene_assets_dir(world);
    let source_path = express_relative_to_assets(Path::new(model_file), &assets_dir);

    spawn_scene_owner(
        world,
        &model_display_name(model_file, "model"),
        ModelSource::new(&source_path.to_string_lossy()),
    )
}

/// Has the next dispatch load the entity's file again; the entity keeps its name and scheduled clip.
/// A generated model has no file to read again and stays as it is.
pub fn request_model_reload(world: &mut World, model_entity: Entity) {
    if world.get_component::<ModelSource>(model_entity).is_none() {
        return;
    }
    world.remove_component::<Children>(model_entity);
}

/// The file a model entity loads: its path is relative to the scene's assets directory or absolute.
pub(super) fn resolve_model_file(world: &World, model_entity: Entity) -> Option<PathBuf> {
    let source = world.get_component::<ModelSource>(model_entity)?;
    let assets_dir = find_scene_assets_dir(world);

    Some(assets_dir.join(&source.path))
}

fn find_scene_assets_dir(world: &World) -> PathBuf {
    world
        .get_resource::<SceneState>()
        .map(|scene_state| scene_state.assets_dir())
        .unwrap_or_else(|| SceneState::new().assets_dir())
}

/// The model whose meshes are in the scene: a file model that finished loading, or a generated one.
pub fn find_loaded_model_entity(world: &World) -> Option<Entity> {
    entities_with::<ModelSource>(world)
        .into_iter()
        .find(|entity| world.get_component::<Children>(*entity).is_some())
        .or_else(|| entities_with::<GeneratedGlb>(world).into_iter().next())
}

/// The path the loaded model's sidecar files sit beside; a generated model uses its name.
pub fn find_loaded_model_file(world: &World) -> Option<PathBuf> {
    let model_entity = find_loaded_model_entity(world)?;

    resolve_model_file(world, model_entity).or_else(|| Some(PathBuf::from(GENERATED_MODEL_NAME)))
}

pub fn has_model_glb(world: &World, model_entity: Entity) -> bool {
    world.get_component::<ModelSource>(model_entity).is_some()
        || world.get_component::<GeneratedGlb>(model_entity).is_some()
}

/// `None` when the entity is not a model.
pub fn read_model_glb(world: &World, model_entity: Entity) -> Option<Result<Vec<u8>>> {
    if let Some(generated) = world.get_component::<GeneratedGlb>(model_entity) {
        return Some(Ok(generated.bytes.clone()));
    }

    let model_file = resolve_model_file(world, model_entity)?;
    Some(std::fs::read(model_file).map_err(Into::into))
}

fn express_relative_to_assets(model_file: &Path, assets_dir: &Path) -> PathBuf {
    let Some((located_file, located_assets_dir)) = locate_from_root(model_file, assets_dir) else {
        return model_file.to_path_buf();
    };

    match located_file.strip_prefix(&located_assets_dir) {
        Ok(relative) => relative.to_path_buf(),
        Err(_) => located_file,
    }
}

/// Both paths from the filesystem root: through the links when the file exists, by name when it is missing.
fn locate_from_root(model_file: &Path, assets_dir: &Path) -> Option<(PathBuf, PathBuf)> {
    if let (Ok(file), Ok(dir)) = (model_file.canonicalize(), assets_dir.canonicalize()) {
        return Some((file, dir));
    }

    let file = std::path::absolute(model_file).ok()?;
    let dir = std::path::absolute(assets_dir).ok()?;
    Some((file, dir))
}

fn find_pending_model_entities(world: &World) -> Vec<Entity> {
    entities_with::<ModelSource>(world)
        .into_iter()
        .filter(|entity| world.get_component::<Children>(*entity).is_none())
        .collect()
}

/// True while a model entity still waits for its meshes, so work that needs the model can wait too.
pub fn is_model_load_pending(world: &World) -> bool {
    !find_pending_model_entities(world).is_empty()
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
pub(crate) fn spawn_loaded_model_for_test(world: &mut World, model_file: &str) -> Entity {
    let model_entity = spawn_scene_owner(world, "model", ModelSource::new(model_file));
    world
        .entity()
        .with_name("mesh")
        .with_parent(model_entity)
        .build();
    model_entity
}

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
    fn reload_request_makes_a_loaded_model_pending_again() {
        let mut world = world_with_load_queue();
        let mut assets = AssetStorage::new();
        let model_entity = spawn_loaded_model_for_test(&mut world, "models/avatar/avatar.fbx");

        request_model_reload(&mut world, model_entity);
        request_pending_model_loads(&mut world, &mut assets);

        assert_eq!(queued_load_entities(&world), vec![model_entity]);
        assert_eq!(entities_with::<ModelSource>(&world), vec![model_entity]);
    }

    #[test]
    fn model_source_with_mesh_children_requests_nothing() {
        let mut world = world_with_load_queue();
        let mut assets = AssetStorage::new();
        spawn_loaded_model_for_test(&mut world, "models/avatar/avatar.fbx");

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
    fn missing_model_file_inside_the_assets_directory_is_stored_relative_to_it() {
        let assets_dir = Path::new("thyllore_missing_assets");
        let model_file = assets_dir.join("models/avatar/missing.fbx");

        let source_path = express_relative_to_assets(&model_file, assets_dir);

        assert_eq!(source_path, PathBuf::from("models/avatar/missing.fbx"));
        assert_eq!(assets_dir.join(&source_path), model_file);
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
