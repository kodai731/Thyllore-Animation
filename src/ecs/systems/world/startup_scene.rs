use std::path::PathBuf;

use crate::asset::AssetStorage;
use crate::ecs::component::ClipSchedule;
use crate::ecs::resource::{SceneLoadCommand, SceneLoadQueue, SceneState, TimelineState};
use crate::ecs::systems::clip_library_systems::clip_library_register_loaded;
use crate::ecs::systems::model_load_systems::build_initial_clip_schedule;
use crate::ecs::world::World;
use crate::hooks::effect_spawn::spawn_empty_scene_defaults;
use crate::scene::{apply_loaded_scene_to_world, LoadedScene};

pub struct StartupScene {
    pub path: PathBuf,
    pub loaded: LoadedScene,
}

pub fn apply_startup_scene(
    world: &mut World,
    assets: &mut AssetStorage,
    startup_scene: Option<StartupScene>,
) {
    let mut scene_state = SceneState::new();

    match startup_scene {
        Some(StartupScene { path, loaded }) => {
            restore_loaded_scene(world, assets, &loaded);
            scene_state.set_from_loaded(path, loaded.scene.metadata);
        }
        None => spawn_empty_scene_defaults(world, assets),
    }

    world.insert_resource(scene_state);
}

fn restore_loaded_scene(world: &mut World, assets: &mut AssetStorage, loaded: &LoadedScene) {
    clip_library_register_loaded(world, assets, loaded.clips.clone());
    apply_loaded_scene_to_world(loaded, world, assets);
    assign_active_clip_schedule(world);

    if let Some(model_path) = &loaded.model_path {
        world
            .resource_mut::<SceneLoadQueue>()
            .push(SceneLoadCommand::LoadModel {
                path: model_path.to_string_lossy().into_owned(),
            });
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
