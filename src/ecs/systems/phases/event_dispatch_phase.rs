use crate::asset::AssetStorage;
use crate::ecs::events::apply_queued_ui_commands;
use crate::ecs::world::World;
use crate::hooks::dispatch_prep::run_dispatch_prep_hooks;
use crate::hooks::effect_ui_event::EffectUiEventDispatchHooks;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

pub fn run_event_dispatch_phase(
    world: &mut World,
    assets: &mut AssetStorage,
    graphics: &GraphicsResources,
) {
    run_dispatch_prep_hooks(world, assets);

    apply_queued_ui_commands(world, assets, graphics);
    dispatch_effect_ui_hooks(world, assets);
}

fn dispatch_effect_ui_hooks(world: &mut World, assets: &mut AssetStorage) {
    let hooks = world
        .get_resource::<EffectUiEventDispatchHooks>()
        .map(|hooks| hooks.entries())
        .unwrap_or_default();
    for hook in hooks {
        (hook.dispatch)(world, assets);
    }
}
