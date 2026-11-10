use std::path::PathBuf;

use crate::ecs::systems::{scene_path_resolve_from_args, StartupScene};
use crate::scene::{find_default_scene, load_scene};

pub(super) fn load_startup_scene() -> Option<StartupScene> {
    find_batch_scene_path()
        .and_then(|path| load_scene_from(path, "batch"))
        .or_else(|| find_default_scene().and_then(|path| load_scene_from(path, "default")))
}

fn find_batch_scene_path() -> Option<PathBuf> {
    let args: Vec<String> = std::env::args().collect();
    let scene_path = scene_path_resolve_from_args(&args).ok().flatten()?;
    Some(PathBuf::from(scene_path))
}

fn load_scene_from(path: PathBuf, origin: &str) -> Option<StartupScene> {
    match load_scene(&path) {
        Ok(loaded) => Some(StartupScene { path, loaded }),
        Err(error) => {
            log_error!("Failed to load {} scene: {:?}", origin, error);
            None
        }
    }
}
