use std::path::{Path, PathBuf};

use crate::animation::editable::EditableAnimationClip;
use crate::app::{App, AppData};
use crate::ecs::component::ClipSchedule;
use crate::ecs::systems::model_load_systems::build_initial_clip_schedule;
use crate::ecs::world::World;
use crate::ecs::{SceneState, TimelineState};
use crate::scene::{find_default_scene, load_scene, LoadedScene};

use anyhow::Result;

const SCREENSHOT_FILE_PREFIX: &str = "screenshot_";
const BATCH_SCENE_FLAG: &str = "--batch-scene";

type StartupScene = (PathBuf, LoadedScene, Vec<EditableAnimationClip>);

pub fn cleanup_old_screenshots() -> Result<()> {
    let log_dir = PathBuf::from("log");
    if !log_dir.exists() {
        return Ok(());
    }

    let mut deleted_count = 0;
    for entry in std::fs::read_dir(&log_dir)? {
        let path = entry?.path();
        if !is_screenshot_file(&path) {
            continue;
        }

        std::fs::remove_file(&path)?;
        deleted_count += 1;
        log!("Deleted old screenshot: {}", path.display());
    }

    if deleted_count > 0 {
        log!("Cleaned up {} old screenshot(s)", deleted_count);
    }

    Ok(())
}

fn is_screenshot_file(path: &Path) -> bool {
    let has_screenshot_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with(SCREENSHOT_FILE_PREFIX));

    has_screenshot_name && path.is_file()
}

impl App {
    pub(super) fn apply_loaded_scene(
        data: &mut AppData,
        loaded_scene: Option<StartupScene>,
    ) -> Result<()> {
        let mut scene_state = SceneState::new();

        match loaded_scene {
            Some((scene_path, scene, clips)) => {
                crate::ecs::systems::clip_library_systems::clip_library_register_loaded(
                    &mut data.ecs_world,
                    &mut data.ecs_assets,
                    clips,
                );
                crate::scene::apply_loaded_scene_to_world(
                    &scene,
                    &mut data.ecs_world,
                    &mut data.ecs_assets,
                );
                assign_active_clip_schedule(&mut data.ecs_world);
                scene_state.set_from_loaded(scene_path, scene.scene.metadata.clone());
            }
            None => crate::hooks::effect_spawn::spawn_empty_scene_defaults(
                &mut data.ecs_world,
                &mut data.ecs_assets,
            ),
        }

        data.ecs_world.insert_resource(scene_state);
        Ok(())
    }

    pub(super) fn determine_startup_model() -> (String, Option<StartupScene>) {
        let batch_scene =
            find_batch_scene_path().and_then(|path| load_startup_scene(path, "batch"));
        let startup_scene = batch_scene
            .or_else(|| find_default_scene().and_then(|path| load_startup_scene(path, "default")));

        match startup_scene {
            Some((model_path, scene)) => (model_path, Some(scene)),
            None => (String::new(), None),
        }
    }
}

fn assign_active_clip_schedule(world: &mut World) {
    let Some(clip_id) = world.resource::<TimelineState>().current_clip_id else {
        return;
    };

    let schedule = build_initial_clip_schedule(Some(clip_id), world);
    for (_, existing) in world.iter_components_mut::<ClipSchedule>() {
        *existing = schedule.clone();
    }
}

fn find_batch_scene_path() -> Option<PathBuf> {
    let args: Vec<String> = std::env::args().collect();
    let flag_position = args.iter().position(|arg| arg == BATCH_SCENE_FLAG)?;

    let Some(path_argument) = args.get(flag_position + 1) else {
        log_error!("{} flag is missing the path argument", BATCH_SCENE_FLAG);
        return None;
    };

    let scene_path = PathBuf::from(path_argument);
    if !scene_path.exists() {
        log_error!("Batch scene path does not exist: {}", scene_path.display());
        return None;
    }

    Some(scene_path)
}

fn load_startup_scene(scene_path: PathBuf, origin: &str) -> Option<(String, StartupScene)> {
    let loaded = match load_scene(&scene_path) {
        Ok(loaded) => loaded,
        Err(error) => {
            log_error!("Failed to load {} scene: {:?}", origin, error);
            return None;
        }
    };

    // A generated mesh has no file, so its model path is empty. Saving uses loaded.scene.model.path.
    let model_path = loaded
        .model_path
        .as_ref()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_default();
    let clips = loaded.clips.clone();
    log!("Loaded {} scene from: {}", origin, scene_path.display());

    Some((model_path, (scene_path, loaded, clips)))
}
