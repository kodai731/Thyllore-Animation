use std::path::{Path, PathBuf};

use anyhow::Result;

use super::entities::{find_entity_name, model_display_name, GENERATED_MODEL_NAME};
use crate::asset::AssetStorage;
use crate::ecs::component::{GeneratedGlb, ModelLoadMark, ModelSource};
use crate::ecs::resource::{SceneLoadCommand, SceneLoadQueue, SceneState};
use crate::ecs::world::{Children, Entity, World};
use crate::hooks::scene::{entities_with, spawn_scene_owner};

/// Adds the entity of a model file to the scene; its meshes are loaded by the next dispatch.
pub fn spawn_model_source(world: &mut World, model_file: &str) -> Entity {
    let source_path = express_for_scene(world, Path::new(model_file));

    let model_entity = spawn_scene_owner(
        world,
        &model_display_name(model_file, "model"),
        ModelSource::new(&source_path.to_string_lossy()),
    );
    world.insert_component(model_entity, ModelLoadMark::Added);

    model_entity
}

/// Whether the load serves a model the user just added or one the scene already holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ModelLoadOrigin {
    Added,
    InScene,
}

/// Has the next dispatch read the entity's file again; a generated model has no file and stays as it is.
pub fn request_model_reload(world: &mut World, model_entity: Entity) {
    if world.get_component::<ModelSource>(model_entity).is_none() {
        return;
    }
    world.insert_component(model_entity, ModelLoadMark::ReloadRequested);
}

/// The loader answers the request it is about to serve, whatever the outcome.
pub(super) fn begin_model_load(world: &mut World, model_entity: Entity) -> ModelLoadOrigin {
    let served_mark = world.get_component::<ModelLoadMark>(model_entity).copied();
    world.remove_component::<ModelLoadMark>(model_entity);

    match served_mark {
        Some(ModelLoadMark::Added) => ModelLoadOrigin::Added,
        Some(ModelLoadMark::ReloadRequested) | Some(ModelLoadMark::Failed) | None => {
            ModelLoadOrigin::InScene
        }
    }
}

/// Only the model a scene file names, alone in the scene, keeps its entity when its file does not load.
pub(super) fn fail_model_load(
    world: &mut World,
    model_entity: Entity,
    load_origin: ModelLoadOrigin,
) {
    if world.get_component::<Children>(model_entity).is_some() {
        return;
    }
    if load_origin == ModelLoadOrigin::Added || has_other_model(world, model_entity) {
        world.despawn(model_entity);
        return;
    }

    world.insert_component(model_entity, ModelLoadMark::Failed);
}

fn has_other_model(world: &World, model_entity: Entity) -> bool {
    let has_other_model_file = entities_with::<ModelSource>(world)
        .into_iter()
        .any(|entity| entity != model_entity);
    let has_generated_model = !entities_with::<GeneratedGlb>(world).is_empty();

    has_other_model_file || has_generated_model
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

/// Relative to the scene's assets tree when the file lies inside it, absolute otherwise.
fn express_for_scene(world: &World, model_file: &Path) -> PathBuf {
    let scene_state = world.get_resource::<SceneState>();
    let assets_tree = match scene_state.as_deref() {
        Some(scene_state) => scene_state.assets_tree(),
        None => SceneState::new().assets_tree(),
    };

    match assets_tree {
        Some(assets_dir) => express_relative_to_assets(model_file, &assets_dir),
        None => locate_file_from_root(model_file),
    }
}

fn locate_file_from_root(model_file: &Path) -> PathBuf {
    model_file
        .canonicalize()
        .or_else(|_| std::path::absolute(model_file))
        .unwrap_or_else(|_| model_file.to_path_buf())
}

fn express_relative_to_assets(model_file: &Path, assets_dir: &Path) -> PathBuf {
    let named = locate_by_name(model_file, assets_dir);
    let linked = locate_through_links(model_file, assets_dir);

    let relative_file = [&named, &linked]
        .into_iter()
        .flatten()
        .find_map(|(file, dir)| file.strip_prefix(dir).ok());
    if let Some(relative_file) = relative_file {
        return relative_file.to_path_buf();
    }

    match linked.or(named) {
        Some((located_file, _)) => located_file,
        None => model_file.to_path_buf(),
    }
}

/// Both paths from the filesystem root as they are named, so a link inside the assets tree stays inside it.
fn locate_by_name(model_file: &Path, assets_dir: &Path) -> Option<(PathBuf, PathBuf)> {
    let file = std::path::absolute(model_file).ok()?;
    let dir = std::path::absolute(assets_dir).ok()?;

    Some((file, dir))
}

/// Both paths from the filesystem root with every link followed; `None` when either does not exist.
fn locate_through_links(model_file: &Path, assets_dir: &Path) -> Option<(PathBuf, PathBuf)> {
    let file = model_file.canonicalize().ok()?;
    let dir = assets_dir.canonicalize().ok()?;

    Some((file, dir))
}

fn find_pending_model_entities(world: &World) -> Vec<Entity> {
    entities_with::<ModelSource>(world)
        .into_iter()
        .filter(|entity| is_model_load_wanted(world, *entity))
        .collect()
}

fn is_model_load_wanted(world: &World, model_entity: Entity) -> bool {
    match world.get_component::<ModelLoadMark>(model_entity).copied() {
        Some(ModelLoadMark::Added) | Some(ModelLoadMark::ReloadRequested) => true,
        Some(ModelLoadMark::Failed) => false,
        None => world.get_component::<Children>(model_entity).is_none(),
    }
}

/// True while a model entity still waits for its meshes, so work that needs the model can wait too.
pub fn is_model_load_pending(world: &World) -> bool {
    !find_pending_model_entities(world).is_empty()
}

/// One scene holds one model, so a dispatch loads one: the model added last, then a reload, then an unloaded one.
fn select_model_to_load(world: &World) -> Option<Entity> {
    let pending = find_pending_model_entities(world);
    let find_marked = |mark: ModelLoadMark| {
        pending.iter().copied().filter(move |entity| {
            world.get_component::<ModelLoadMark>(*entity).copied() == Some(mark)
        })
    };

    find_marked(ModelLoadMark::Added)
        .max()
        .or_else(|| find_marked(ModelLoadMark::ReloadRequested).next())
        .or_else(|| pending.first().copied())
}

/// A request names the model chosen when it was queued; one overtaken since then is logged and not served.
pub fn confirm_model_load_request(world: &World, model_entity: Entity) -> bool {
    let chosen_entity = select_model_to_load(world);
    if chosen_entity == Some(model_entity) {
        return true;
    }

    match chosen_entity {
        Some(chosen_entity) => log!(
            "Model load request for '{}' skipped: '{}' is the model to load now",
            find_entity_name(world, model_entity),
            find_entity_name(world, chosen_entity)
        ),
        None => log!(
            "Model load request for '{}' skipped: no model waits for a load",
            find_entity_name(world, model_entity)
        ),
    }
    false
}

fn request_pending_model_loads(world: &mut World, _: &mut AssetStorage) {
    let Some(entity) = select_model_to_load(world) else {
        return;
    };

    world
        .resource_mut::<SceneLoadQueue>()
        .push(SceneLoadCommand::LoadModel { entity });
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
    fn models_added_together_request_only_the_last_one() {
        let mut world = world_with_load_queue();
        let mut assets = AssetStorage::new();
        spawn_model_source(&mut world, "models/avatar/first.fbx");
        let last_added = spawn_model_source(&mut world, "models/avatar/last.fbx");

        request_pending_model_loads(&mut world, &mut assets);

        assert_eq!(queued_load_entities(&world), vec![last_added]);
    }

    #[test]
    fn added_model_is_requested_before_the_unloaded_model_of_the_scene() {
        let mut world = world_with_load_queue();
        let mut assets = AssetStorage::new();
        let added_entity = spawn_model_source(&mut world, "models/avatar/added.fbx");
        spawn_scene_owner(
            &mut world,
            "model",
            ModelSource::new("models/avatar/avatar.fbx"),
        );

        request_pending_model_loads(&mut world, &mut assets);

        assert_eq!(queued_load_entities(&world), vec![added_entity]);
    }

    #[test]
    fn model_added_after_a_request_was_queued_takes_the_load_from_it() {
        let mut world = world_with_load_queue();
        let mut assets = AssetStorage::new();
        let scene_model = spawn_scene_owner(
            &mut world,
            "model",
            ModelSource::new("models/avatar/avatar.fbx"),
        );
        request_pending_model_loads(&mut world, &mut assets);
        assert_eq!(queued_load_entities(&world), vec![scene_model]);

        let added_entity = spawn_model_source(&mut world, "models/avatar/added.fbx");

        assert!(!confirm_model_load_request(&world, scene_model));
        assert!(confirm_model_load_request(&world, added_entity));
        assert_eq!(
            world.get_component::<ModelLoadMark>(added_entity).copied(),
            Some(ModelLoadMark::Added)
        );
        assert_eq!(
            entities_with::<ModelSource>(&world),
            vec![scene_model, added_entity]
        );
    }

    #[test]
    fn reload_is_chosen_before_an_unloaded_model_whatever_their_entity_order() {
        let mut world = world_with_load_queue();
        let unloaded_entity = spawn_scene_owner(
            &mut world,
            "unloaded",
            ModelSource::new("models/avatar/unloaded.fbx"),
        );
        let loaded_entity = spawn_loaded_model_for_test(&mut world, "models/avatar/avatar.fbx");
        assert!(unloaded_entity < loaded_entity);

        request_model_reload(&mut world, loaded_entity);

        assert_eq!(select_model_to_load(&world), Some(loaded_entity));
    }

    #[test]
    fn request_for_a_model_that_already_loaded_is_no_longer_chosen() {
        let mut world = world_with_load_queue();
        let model_entity = spawn_loaded_model_for_test(&mut world, "models/avatar/avatar.fbx");

        assert!(!confirm_model_load_request(&world, model_entity));
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
    fn reload_request_keeps_the_meshes_until_the_file_is_read() {
        let mut world = world_with_load_queue();
        let model_entity = spawn_loaded_model_for_test(&mut world, "models/avatar/avatar.fbx");

        request_model_reload(&mut world, model_entity);

        assert_eq!(find_loaded_model_entity(&world), Some(model_entity));
    }

    #[test]
    fn failed_reload_leaves_the_loaded_model_as_it_was() {
        let mut world = world_with_load_queue();
        let mut assets = AssetStorage::new();
        let model_entity = spawn_loaded_model_for_test(&mut world, "models/avatar/avatar.fbx");
        request_model_reload(&mut world, model_entity);

        let load_origin = begin_model_load(&mut world, model_entity);
        fail_model_load(&mut world, model_entity, load_origin);
        request_pending_model_loads(&mut world, &mut assets);

        assert!(queued_load_entities(&world).is_empty());
        assert_eq!(find_loaded_model_entity(&world), Some(model_entity));
    }

    #[test]
    fn failed_load_keeps_the_model_entity_of_a_scene_file_without_requesting_it_again() {
        let mut world = world_with_load_queue();
        let mut assets = AssetStorage::new();
        let model_entity = spawn_scene_owner(
            &mut world,
            "model",
            ModelSource::new("models/avatar/missing.fbx"),
        );

        let load_origin = begin_model_load(&mut world, model_entity);
        fail_model_load(&mut world, model_entity, load_origin);
        request_pending_model_loads(&mut world, &mut assets);

        assert!(queued_load_entities(&world).is_empty());
        assert_eq!(entities_with::<ModelSource>(&world), vec![model_entity]);
        assert_eq!(find_loaded_model_entity(&world), None);
    }

    #[test]
    fn failed_load_removes_the_model_entity_the_user_added() {
        let mut world = world_with_load_queue();
        let added_entity = spawn_model_source(&mut world, "models/avatar/missing.fbx");

        let load_origin = begin_model_load(&mut world, added_entity);
        fail_model_load(&mut world, added_entity, load_origin);

        assert!(entities_with::<ModelSource>(&world).is_empty());
    }

    #[test]
    fn failed_load_beside_a_loaded_model_removes_the_other_model_entity() {
        let mut world = world_with_load_queue();
        let loaded_entity = spawn_loaded_model_for_test(&mut world, "models/avatar/avatar.fbx");
        let broken_entity = spawn_scene_owner(
            &mut world,
            "broken",
            ModelSource::new("models/avatar/missing.fbx"),
        );

        let load_origin = begin_model_load(&mut world, broken_entity);
        fail_model_load(&mut world, broken_entity, load_origin);

        assert_eq!(entities_with::<ModelSource>(&world), vec![loaded_entity]);
        assert_eq!(find_loaded_model_entity(&world), Some(loaded_entity));
    }

    #[test]
    fn failed_load_beside_an_unloaded_model_removes_the_failed_model_entity() {
        let mut world = world_with_load_queue();
        let broken_entity = spawn_scene_owner(
            &mut world,
            "broken",
            ModelSource::new("models/avatar/missing.fbx"),
        );
        let unloaded_entity = spawn_scene_owner(
            &mut world,
            "unloaded",
            ModelSource::new("models/avatar/avatar.fbx"),
        );

        let load_origin = begin_model_load(&mut world, broken_entity);
        fail_model_load(&mut world, broken_entity, load_origin);

        assert_eq!(entities_with::<ModelSource>(&world), vec![unloaded_entity]);
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

    #[cfg(unix)]
    #[test]
    fn model_file_under_a_linked_assets_subdirectory_is_stored_relative_to_it() {
        let root = std::env::temp_dir().join("thyllore_model_source_linked");
        let assets_dir = root.join("assets");
        let shared_models_dir = root.join("shared/models");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&assets_dir).unwrap();
        std::fs::create_dir_all(shared_models_dir.join("avatar")).unwrap();
        std::fs::write(shared_models_dir.join("avatar/avatar.fbx"), b"stub").unwrap();
        std::os::unix::fs::symlink(&shared_models_dir, assets_dir.join("models")).unwrap();

        let source_path =
            express_relative_to_assets(&assets_dir.join("models/avatar/avatar.fbx"), &assets_dir);

        assert_eq!(source_path, PathBuf::from("models/avatar/avatar.fbx"));
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
    fn model_added_to_a_scene_outside_an_assets_tree_is_stored_absolute() {
        let mut world = world_with_load_queue();
        world.resource_mut::<SceneState>().current_scene_path =
            Some(std::env::temp_dir().join("thyllore_loose.scene.ron"));
        let model_file = std::env::temp_dir().join("thyllore_loose_model/avatar.fbx");

        let model_entity = spawn_model_source(&mut world, &model_file.to_string_lossy());

        let source = world.get_component::<ModelSource>(model_entity).unwrap();
        assert!(Path::new(&source.path).is_absolute());
        assert_eq!(
            resolve_model_file(&world, model_entity),
            Some(PathBuf::from(&source.path))
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
