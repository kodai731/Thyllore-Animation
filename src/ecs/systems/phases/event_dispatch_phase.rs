use crate::asset::AssetStorage;
use crate::ecs::events::{apply_queued_ui_commands, UIEvent};
use crate::ecs::world::World;
use crate::ecs::UIEventQueue;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

use super::event_dispatch::overlay::dispatch_overlay_events;

pub fn run_event_dispatch_phase(
    world: &mut World,
    assets: &mut AssetStorage,
    graphics: &GraphicsResources,
) {
    #[cfg(feature = "text-to-motion")]
    super::event_dispatch::ml::text_to_motion::drain_grpc_responses(world, assets);

    #[cfg(feature = "auto-rig")]
    super::event_dispatch::ml::auto_rig::poll_mesh_server_status(world);
    #[cfg(feature = "auto-rig")]
    super::event_dispatch::ml::auto_rig::poll_rigging_server_status(world);

    let effect_events: Vec<UIEvent> = match world.get_resource_mut::<UIEventQueue>() {
        Some(mut ui_events) => ui_events.drain().collect(),
        None => Vec::new(),
    };
    dispatch_overlay_events(&effect_events, world);

    apply_queued_ui_commands(world, assets, graphics);
}
