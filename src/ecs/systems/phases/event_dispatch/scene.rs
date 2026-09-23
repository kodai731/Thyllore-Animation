use crate::ecs::events::UIEvent;
use crate::ecs::world::World;

pub fn dispatch_scene_events(events: &[UIEvent], world: &mut World) {
    for event in events {
        if let UIEvent::SaveScene = event {
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
    }
}
