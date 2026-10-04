use crate::asset::AssetStorage;
use crate::ecs::events::apply_queued_ui_commands;
use crate::ecs::world::World;
use crate::hooks::dispatch_prep::run_dispatch_prep_hooks;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

pub fn run_event_dispatch_phase(
    world: &mut World,
    assets: &mut AssetStorage,
    graphics: &GraphicsResources,
) {
    run_dispatch_prep_hooks(world, assets);

    apply_queued_ui_commands(world, assets, graphics);
}
