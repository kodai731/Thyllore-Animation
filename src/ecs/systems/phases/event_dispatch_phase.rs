use crate::asset::AssetStorage;
use crate::ecs::events::{apply_queued_ui_commands, UIEvent};
use crate::ecs::world::World;
use crate::ecs::UIEventQueue;
use crate::hooks::dispatch_prep::run_dispatch_prep_hooks;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

use super::event_dispatch::overlay::dispatch_overlay_events;

pub fn run_event_dispatch_phase(
    world: &mut World,
    assets: &mut AssetStorage,
    graphics: &GraphicsResources,
) {
    run_dispatch_prep_hooks(world, assets);

    let effect_events: Vec<UIEvent> = match world.get_resource_mut::<UIEventQueue>() {
        Some(mut ui_events) => ui_events.drain().collect(),
        None => Vec::new(),
    };
    dispatch_overlay_events(&effect_events, world);

    apply_queued_ui_commands(world, assets, graphics);
}
