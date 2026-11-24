use crate::asset::AssetStorage;
use crate::ecs::events::UiCommand;
use crate::ecs::resource::SceneState;
use crate::ecs::world::World;
use crate::scene::default_scene_path;
use thyllore_vulkan_core::resource::GraphicsResources;

#[derive(Clone, Debug)]
pub enum SceneEvent {
    SaveScene,
}

impl UiCommand for SceneEvent {
    fn apply(self: Box<Self>, world: &mut World, _: &mut AssetStorage, _: &GraphicsResources) {
        match *self {
            SceneEvent::SaveScene => save_open_scene(world),
        }
    }
}

/// Asset paths in the world are relative to the open scene's assets directory, so the scene is
/// written back to its own file; without an open scene that is the default scene.
fn save_open_scene(world: &World) {
    let scene_path = world
        .get_resource::<SceneState>()
        .and_then(|scene_state| scene_state.current_scene_path.clone())
        .unwrap_or_else(default_scene_path);

    match crate::scene::save_scene(&scene_path, world) {
        Ok(()) => {
            msg_info!("Scene saved to {:?}", scene_path);
        }
        Err(e) => {
            msg_error!("Failed to save scene: {:?}", e);
        }
    }
}
