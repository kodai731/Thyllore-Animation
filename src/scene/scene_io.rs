use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::clip_io::{load_animation_clip, save_animation_clip};
use super::entities::{apply_scene_entities, capture_scene_entities};
use super::error::{SceneError, SceneResult};
use super::file::{AnimationClipRef, ModelReference, SceneFile, SCENE_FORMAT_VERSION};
use crate::ecs::resource::{ClipLibrary, ModelState, SceneState};
use crate::ecs::world::World;
use crate::hooks::scene::SceneValue;
use crate::hooks::scene_resource::SceneResourceHooks;

pub fn save_scene(scene_path: &Path, world: &World) -> SceneResult<()> {
    let animations_dir = scene_path
        .parent()
        .unwrap_or(Path::new("."))
        .parent()
        .unwrap_or(Path::new("."))
        .join("animations");
    fs::create_dir_all(&animations_dir)?;
    let animation_clips = save_animation_clips(world, &animations_dir)?;

    let scene = build_scene_file(animation_clips, world);
    write_scene_file(scene_path, &scene)?;

    log!("Saved scene to: {}", scene_path.display());
    Ok(())
}

fn build_scene_file(animation_clips: Vec<AnimationClipRef>, world: &World) -> SceneFile {
    let previous_metadata = world
        .get_resource::<SceneState>()
        .and_then(|s| s.previous_metadata.clone());
    let scene_name = previous_metadata
        .as_ref()
        .map(|m| m.name.clone())
        .unwrap_or_else(|| "Untitled Scene".to_string());
    let model_path = world
        .get_resource::<ModelState>()
        .map(|s| s.model_path.clone())
        .unwrap_or_default();

    let mut scene = SceneFile::new(&scene_name, &model_path);
    scene.animation_clips = animation_clips;
    scene.resources = capture_scene_resources(world);
    scene.entities = capture_scene_entities(world);

    if let Some(prev) = previous_metadata {
        scene.metadata.created_at = prev.created_at;
    }
    scene.metadata.update_modified();
    scene
}

/// Every registered resource present in the world, keyed by type key.
pub fn capture_scene_resources(world: &World) -> BTreeMap<String, SceneValue> {
    let Some(hooks) = world.get_resource::<SceneResourceHooks>() else {
        log_warn!("SceneResourceHooks resource missing: no scene resources captured");
        return BTreeMap::new();
    };
    hooks
        .iter()
        .filter_map(|hook| (hook.capture)(world).map(|value| (hook.type_key.to_string(), value)))
        .collect()
}

pub fn apply_scene_resources(world: &mut World, resources: &BTreeMap<String, SceneValue>) {
    let hooks: Vec<_> = match world.get_resource::<SceneResourceHooks>() {
        Some(hooks) => hooks.iter().copied().collect(),
        None => {
            log_warn!("SceneResourceHooks resource missing: scene resources not applied");
            return;
        }
    };

    for (type_key, value) in resources {
        match hooks.iter().find(|hook| hook.type_key == type_key) {
            Some(hook) => {
                if let Err(error) = (hook.apply)(world, value) {
                    log_warn!("Failed to apply scene resource {}: {:#}", type_key, error);
                }
            }
            None => log_warn!("Unknown scene resource type key: {}", type_key),
        }
    }
}

fn save_animation_clips(
    world: &World,
    animations_dir: &Path,
) -> SceneResult<Vec<AnimationClipRef>> {
    let clip_library = match world.get_resource::<ClipLibrary>() {
        Some(cm) => cm,
        None => return Ok(Vec::new()),
    };

    let mut animation_clips = Vec::new();
    let mut saved_paths = std::collections::HashSet::new();

    for (clip_id, clip_name) in
        crate::ecs::systems::clip_library_systems::clip_library_clip_names(&clip_library)
    {
        if let Some(clip) = clip_library.get(clip_id) {
            let clip_filename = sanitize_filename(&clip_name);
            let relative_path = format!("animations/{}.anim.ron", clip_filename);

            let clip_path = animations_dir.join(format!("{}.anim.ron", clip_filename));
            save_animation_clip(&clip_path, clip)?;

            if saved_paths.insert(relative_path.clone()) {
                animation_clips.push(AnimationClipRef::new(&relative_path));
            }
        }
    }

    Ok(animation_clips)
}

fn write_scene_file(scene_path: &Path, scene: &SceneFile) -> SceneResult<()> {
    let config = ron::ser::PrettyConfig::new()
        .depth_limit(8)
        .separate_tuple_members(true)
        .enumerate_arrays(false);

    let content = ron::ser::to_string_pretty(scene, config)?;

    if let Some(parent) = scene_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(scene_path, content)?;
    Ok(())
}

pub fn load_scene(scene_path: &Path) -> SceneResult<LoadedScene> {
    if !scene_path.exists() {
        return Err(SceneError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Scene file not found: {}", scene_path.display()),
        )));
    }

    let content = fs::read_to_string(scene_path)?;
    let scene: SceneFile = ron::from_str(&content)?;
    if scene.version != SCENE_FORMAT_VERSION {
        return Err(SceneError::VersionMismatch {
            expected: SCENE_FORMAT_VERSION,
            found: scene.version,
        });
    }

    let assets_dir = scene_path
        .parent()
        .unwrap_or(Path::new("."))
        .parent()
        .unwrap_or(Path::new("."));

    let model_path = resolve_model_path(assets_dir, &scene.model)?;

    let mut clips = Vec::new();
    let mut loaded_paths = std::collections::HashSet::new();
    for clip_ref in &scene.animation_clips {
        if !loaded_paths.insert(clip_ref.path.clone()) {
            continue;
        }
        let clip_path = assets_dir.join(&clip_ref.path);
        let mut clip = load_animation_clip(&clip_path)?;
        clip.source_path = Some(clip_path.to_string_lossy().to_string());
        clips.push(clip);
    }

    log!("Loaded scene from: {}", scene_path.display());

    Ok(LoadedScene {
        scene,
        model_path,
        clips,
    })
}

pub fn find_default_scene() -> Option<PathBuf> {
    let path = PathBuf::from("assets/scenes/default.scene.ron");
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

pub struct LoadedScene {
    pub scene: SceneFile,
    /// `None` when the scene's mesh was generated in-app and has no file to load.
    pub model_path: Option<PathBuf>,
    pub clips: Vec<crate::animation::editable::EditableAnimationClip>,
}

fn resolve_model_path(assets_dir: &Path, model: &ModelReference) -> SceneResult<Option<PathBuf>> {
    if model.is_generated_mesh() {
        return Ok(None);
    }

    let model_path = assets_dir.join(&model.path);
    if !model_path.exists() {
        return Err(SceneError::ModelNotFound(model_path));
    }
    Ok(Some(model_path))
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Clips referenced by the scene must already be registered in the `ClipLibrary`.
pub fn apply_loaded_scene_to_world(
    loaded: &LoadedScene,
    world: &mut World,
    assets: &mut crate::asset::AssetStorage,
) {
    apply_scene_resources(world, &loaded.scene.resources);
    apply_scene_entities(world, assets, &loaded.scene.entities);
}

#[cfg(test)]
mod tests {
    use super::super::entities::test_support::ProbeOwner;
    use super::super::entities::world_with_scene_hooks;
    use super::*;
    use crate::ecs::component::MotionPath;
    use crate::ecs::events::DebugPrimitiveKind;
    use crate::ecs::resource::{Camera, PanelLayout, TimelineState};
    use crate::ecs::world::{Name, Transform};
    use crate::hooks::scene::{decode_scene_value, SceneOwner};
    use cgmath::Vector3;
    use thyllore_scene_core::SceneComponent;

    fn write_scene(dir: &Path, model_path: &str) -> PathBuf {
        let scenes_dir = dir.join("scenes");
        fs::create_dir_all(&scenes_dir).unwrap();
        let scene = SceneFile::new("test", model_path);
        let scene_path = scenes_dir.join("test.scene.ron");
        write_scene_file(&scene_path, &scene).unwrap();
        scene_path
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("thyllore_scene_{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn scene_path_in(name: &str) -> PathBuf {
        let scenes_dir = temp_dir(name).join("scenes");
        fs::create_dir_all(&scenes_dir).unwrap();
        scenes_dir.join("test.scene.ron")
    }

    fn world_with_resources() -> World {
        let mut world = world_with_scene_hooks();
        world.insert_resource(Camera::default());
        world.insert_resource(PanelLayout::default());
        world.insert_resource(TimelineState::default());
        world
    }

    #[test]
    fn generated_mesh_scene_loads_without_a_model_file() {
        let dir = temp_dir("generated");
        let scene_path = write_scene(&dir, ModelReference::GENERATED_MESH);

        let loaded = load_scene(&scene_path).expect("generated mesh is not a file path");

        assert!(loaded.model_path.is_none());
        assert!(loaded.scene.model.is_generated_mesh());
    }

    #[test]
    fn scene_pointing_at_a_missing_model_file_is_rejected() {
        let dir = temp_dir("missing");
        let scene_path = write_scene(&dir, "models/does_not_exist.glb");

        assert!(matches!(
            load_scene(&scene_path),
            Err(SceneError::ModelNotFound(_))
        ));
    }

    #[test]
    fn older_format_versions_are_rejected() {
        let scene_path = scene_path_in("old_version");
        fs::write(
            &scene_path,
            r#"(version: 6, model: (path: "Generated Mesh"))"#,
        )
        .unwrap();

        assert!(matches!(
            load_scene(&scene_path),
            Err(SceneError::VersionMismatch {
                expected: SCENE_FORMAT_VERSION,
                found: 6
            })
        ));
    }

    #[test]
    fn scene_resolves_an_existing_model_file() {
        let dir = temp_dir("present");
        fs::create_dir_all(dir.join("models")).unwrap();
        fs::write(dir.join("models/mesh.glb"), b"stub").unwrap();
        let scene_path = write_scene(&dir, "models/mesh.glb");

        let loaded = load_scene(&scene_path).expect("model file exists");

        assert_eq!(loaded.model_path, Some(dir.join("models/mesh.glb")));
    }

    #[test]
    fn registered_resources_roundtrip_through_the_file() {
        let scene_path = scene_path_in("resources");
        let mut world = world_with_resources();
        {
            let mut camera = world.resource_mut::<Camera>();
            camera.distance = 12.5;
            camera.pivot = Vector3::new(1.0, 2.0, 3.0);
        }
        world.resource_mut::<PanelLayout>().debug_height = 123.0;
        world.resource_mut::<TimelineState>().looping = false;

        save_scene(&scene_path, &world).unwrap();
        let loaded = load_scene(&scene_path).unwrap();
        let keys: Vec<&String> = loaded.scene.resources.keys().collect();
        assert!(keys.contains(&&"camera".to_string()), "{keys:?}");
        assert!(keys.contains(&&"timeline".to_string()), "{keys:?}");

        let mut restored = world_with_resources();
        let mut assets = crate::asset::AssetStorage::new();
        apply_loaded_scene_to_world(&loaded, &mut restored, &mut assets);

        let camera = restored.resource::<Camera>();
        assert_eq!(camera.distance, 12.5);
        assert_eq!(camera.initial_distance, 12.5);
        assert_eq!(camera.pivot, Vector3::new(1.0, 2.0, 3.0));
        assert_eq!(restored.resource::<PanelLayout>().debug_height, 123.0);
        assert!(!restored.resource::<TimelineState>().looping);
    }

    #[test]
    fn owner_entities_with_attachments_roundtrip_through_the_file() {
        let scene_path = scene_path_in("entities");
        let mut world = world_with_resources();
        let entity = crate::hooks::scene::spawn_scene_owner(
            &mut world,
            "probe",
            ProbeOwner {
                position: [4.0, 5.0, 6.0],
                level: 0.75,
            },
        );
        world.insert_component(
            entity,
            MotionPath {
                radius: 2.0,
                ..MotionPath::default()
            },
        );

        save_scene(&scene_path, &world).unwrap();
        let loaded = load_scene(&scene_path).unwrap();
        assert_eq!(loaded.scene.entities.len(), 1);
        let components = &loaded.scene.entities[0].components;
        assert!(components.contains_key(ProbeOwner::TYPE_KEY));
        let probe: ProbeOwner = decode_scene_value(&components[ProbeOwner::TYPE_KEY]).unwrap();
        assert_eq!(probe.level, 0.75);

        let mut restored = world_with_resources();
        let mut assets = crate::asset::AssetStorage::new();
        apply_loaded_scene_to_world(&loaded, &mut restored, &mut assets);

        let probes: Vec<_> = restored.iter_components::<ProbeOwner>().collect();
        assert_eq!(probes.len(), 1);
        let (restored_entity, probe) = probes[0];
        assert_eq!(probe.level, 0.75);
        assert_eq!(
            restored
                .get_component::<Name>(restored_entity)
                .map(|n| n.0.as_str()),
            Some("probe")
        );
        assert_eq!(
            restored
                .get_component::<Transform>(restored_entity)
                .map(|t| t.translation),
            Some(Vector3::new(4.0, 5.0, 6.0))
        );
        assert_eq!(
            restored
                .get_component::<MotionPath>(restored_entity)
                .map(|m| m.radius),
            Some(2.0)
        );
        assert_eq!(ProbeOwner::ICON, crate::ecs::component::EntityIcon::Empty);
    }

    fn spawn_tagged_debug_primitive(
        world: &mut World,
        kind: DebugPrimitiveKind,
        position: [f32; 3],
    ) -> crate::ecs::world::Entity {
        let entity = world
            .entity()
            .with_name("primitive")
            .with_transform(Transform {
                translation: position.into(),
                ..Default::default()
            })
            .build();
        world.insert_component(entity, crate::ecs::component::DebugPrimitiveTag { kind });
        entity
    }

    #[test]
    fn debug_primitives_roundtrip_and_wait_for_their_mesh() {
        let scene_path = scene_path_in("debug_primitives");
        let mut world = world_with_resources();
        spawn_tagged_debug_primitive(&mut world, DebugPrimitiveKind::Cube, [1.0, 2.0, 3.0]);
        spawn_tagged_debug_primitive(&mut world, DebugPrimitiveKind::Floor, [0.0, -1.6, 0.0]);

        save_scene(&scene_path, &world).unwrap();
        let loaded = load_scene(&scene_path).unwrap();
        assert_eq!(loaded.scene.entities.len(), 2);

        let mut respawned = world_with_resources();
        let mut assets = crate::asset::AssetStorage::new();
        apply_loaded_scene_to_world(&loaded, &mut respawned, &mut assets);

        let awaiting = crate::ecs::systems::take_debug_primitives_awaiting_mesh(&mut respawned);
        assert_eq!(awaiting.len(), 2);
        assert_eq!(awaiting[0].kind, "cube");
        assert_eq!(awaiting[0].position, [1.0, 2.0, 3.0]);
        assert_eq!(awaiting[1].kind, "floor");
        assert_eq!(awaiting[1].position, [0.0, -1.6, 0.0]);
        assert!(respawned
            .iter_components::<crate::ecs::component::DebugPrimitiveTag>()
            .next()
            .is_none());
    }

    #[test]
    fn default_scene_asset_parses() {
        let content = fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/scenes/default.scene.ron"),
        )
        .expect("default scene asset readable");
        let scene: SceneFile = ron::from_str(&content).expect("default scene asset parses");

        assert_eq!(scene.version, SCENE_FORMAT_VERSION);
        assert!(!scene.resources.is_empty());
        assert!(!scene.entities.is_empty());
    }

    #[test]
    fn every_scene_asset_parses_at_the_current_version() {
        let scenes_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/scenes");
        let mut checked = 0;
        for entry in fs::read_dir(&scenes_dir).expect("scene assets readable") {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("ron") {
                continue;
            }
            let content = fs::read_to_string(&path).unwrap();
            let scene: SceneFile = ron::from_str(&content)
                .unwrap_or_else(|e| panic!("{} does not parse: {e}", path.display()));
            assert_eq!(scene.version, SCENE_FORMAT_VERSION, "{}", path.display());
            checked += 1;
        }
        assert!(checked > 0);
    }
}
