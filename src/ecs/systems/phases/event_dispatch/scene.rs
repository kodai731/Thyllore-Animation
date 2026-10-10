use crate::asset::AssetStorage;
use crate::ecs::events::UiCommand;
use crate::ecs::world::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

#[derive(Clone, Debug)]
pub enum SceneEvent {
    SaveScene,
}

impl UiCommand for SceneEvent {
    fn apply(self: Box<Self>, world: &mut World, _: &mut AssetStorage, _: &GraphicsResources) {
        match *self {
            SceneEvent::SaveScene => save_default_scene(world),
        }
    }
}

fn save_default_scene(world: &World) {
    let scene_path = std::path::PathBuf::from("assets/scenes/default.scene.ron");

    match crate::scene::save_scene(&scene_path, world) {
        Ok(()) => {
            msg_info!("Scene saved to {:?}", scene_path);
        }
        Err(e) => {
            msg_error!("Failed to save scene: {:?}", e);
        }
    }
}
