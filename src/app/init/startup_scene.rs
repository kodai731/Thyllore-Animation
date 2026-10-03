use crate::app::{App, AppData};

use crate::ecs::{SceneState, TimelineState};
use crate::vulkanr::command::*;
use crate::vulkanr::device::*;
use crate::vulkanr::swapchain::*;
use crate::vulkanr::vulkan::*;

use anyhow::Result;
use std::rc::Rc;

/// Clean up old screenshot files from the log directory
pub fn cleanup_old_screenshots() -> Result<()> {
    use std::fs;
    use std::path::PathBuf;

    let log_dir = PathBuf::from("log");

    if !log_dir.exists() {
        return Ok(());
    }

    let entries = fs::read_dir(&log_dir)?;

    let mut deleted_count = 0;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(filename) = path.file_name() {
                if let Some(filename_str) = filename.to_str() {
                    if filename_str.starts_with("screenshot_") {
                        fs::remove_file(&path)?;
                        deleted_count += 1;
                        log!("Deleted old screenshot: {:?}", filename_str);
                    }
                }
            }
        }
    }

    if deleted_count > 0 {
        log!("Cleaned up {} old screenshot(s)", deleted_count);
    }

    Ok(())
}

impl App {
    unsafe fn load_startup_model(
        instance: &Instance,
        rrdevice: &RRDevice,
        rrcommand_pool: &Rc<RRCommandPool>,
        rrswapchain: &RRSwapchain,
        data: &mut AppData,
        model_path: &str,
        has_scene: bool,
    ) {
        if let Err(e) = Self::load_model_from_path_with_resources(
            instance,
            rrdevice,
            data,
            rrcommand_pool,
            rrswapchain,
            model_path,
            has_scene,
        ) {
            eprintln!("Failed to load model: {:?}", e);
            log_error!("Failed to load model: {:?}", e);
        }
        log!("loaded initial model: {}", model_path);
    }

    pub(super) fn apply_loaded_scene(
        data: &mut AppData,
        loaded_scene: Option<(
            std::path::PathBuf,
            crate::scene::LoadedScene,
            Vec<crate::animation::editable::EditableAnimationClip>,
        )>,
    ) -> anyhow::Result<()> {
        let mut scene_state = SceneState::new();
        if let Some((scene_path, scene, clips)) = loaded_scene {
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

            let active_clip_id = {
                let timeline = data.ecs_world.resource::<TimelineState>();
                timeline.current_clip_id
            };

            if let Some(clip_id) = active_clip_id {
                let schedule =
                    crate::app::model::build_initial_clip_schedule(Some(clip_id), &data.ecs_world);
                for (_, existing) in data
                    .ecs_world
                    .iter_components_mut::<crate::ecs::component::ClipSchedule>()
                {
                    *existing = schedule.clone();
                }
            }

            scene_state.set_from_loaded(scene_path, scene.scene.metadata.clone());
        } else {
            crate::hooks::effect_spawn::spawn_empty_scene_defaults(
                &mut data.ecs_world,
                &mut data.ecs_assets,
            );
        }
        data.ecs_world.insert_resource(scene_state);
        Ok(())
    }

    pub(super) fn determine_startup_model() -> (
        String,
        Option<(
            std::path::PathBuf,
            crate::scene::LoadedScene,
            Vec<crate::animation::editable::EditableAnimationClip>,
        )>,
    ) {
        use crate::scene::{find_default_scene, load_scene};

        // let default_model_path = "assets/models/stickman/stickman.glb".to_string();

        // Check for --batch-scene flag in command line arguments
        let args: Vec<String> = std::env::args().collect();
        if let Some(pos) = args.iter().position(|a| a == "--batch-scene") {
            if let Some(path_str) = args.get(pos + 1) {
                let scene_path = std::path::PathBuf::from(path_str);
                if scene_path.exists() {
                    match load_scene(&scene_path) {
                        Ok(loaded) => {
                            let model_path = loaded
                                .model_path
                                .as_ref()
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or_default();
                            let clips = loaded.clips.clone();
                            log!("Loaded batch scene from: {}", scene_path.display());
                            return (model_path, Some((scene_path, loaded, clips)));
                        }
                        Err(e) => {
                            log_error!("Failed to load batch scene: {:?}", e);
                        }
                    }
                } else {
                    log_error!("Batch scene path does not exist: {}", scene_path.display());
                }
            } else {
                log_error!("--batch-scene flag is missing the path argument");
            }
        }

        if let Some(scene_path) = find_default_scene() {
            match load_scene(&scene_path) {
                Ok(loaded) => {
                    // Loading uses the resolved assets path from LoadedScene.model_path (None for
                    // a generated mesh with no file). Saving still uses loaded.scene.model.path.
                    let model_path = loaded
                        .model_path
                        .as_ref()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let clips = loaded.clips.clone();
                    log!("Loaded default scene from: {}", scene_path.display());
                    return (model_path, Some((scene_path, loaded, clips)));
                }
                Err(e) => {
                    log_error!("Failed to load default scene: {:?}", e);
                }
            }
        }

        // (default_model_path, None)
        ("".to_string(), None)
    }
}
