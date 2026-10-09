use crate::asset::AssetStorage;
use crate::ecs::events::apply_queued_ui_commands;
use crate::ecs::systems::validation_report_sync;
use crate::ecs::world::World;
use crate::hooks::dispatch_prep::run_dispatch_prep_hooks;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

pub fn run_event_dispatch_phase(
    world: &mut World,
    assets: &mut AssetStorage,
    graphics: &GraphicsResources,
) {
    if let Some(mut report) = world.get_resource_mut::<crate::ecs::resource::ValidationReport>() {
        validation_report_sync(
            &mut report,
            thyllore_log_core::validation_stats::validation_stats_snapshot(),
        );
    }

    run_dispatch_prep_hooks(world, assets);

    apply_queued_ui_commands(world, assets, graphics);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::events::{UiCommand, UiCommandQueue};

    #[derive(Debug)]
    enum ProbeUiCommand {
        Ping,
    }

    #[derive(Default)]
    struct ProbeDispatchCounter {
        count: usize,
    }

    impl UiCommand for ProbeUiCommand {
        fn apply(
            self: Box<Self>,
            world: &mut World,
            _assets: &mut AssetStorage,
            _graphics: &GraphicsResources,
        ) {
            let Some(mut counter) = world.get_resource_mut::<ProbeDispatchCounter>() else {
                return;
            };
            match *self {
                ProbeUiCommand::Ping => counter.count += 1,
            }
        }
    }

    #[test]
    fn test_probe_ui_command_is_applied_by_the_dispatch_phase() {
        let mut world = World::new();
        let mut assets = AssetStorage::default();
        world.insert_resource(UiCommandQueue::default());
        world.insert_resource(ProbeDispatchCounter::default());

        world.send_command(ProbeUiCommand::Ping);
        run_event_dispatch_phase(&mut world, &mut assets, &GraphicsResources::default());

        assert_eq!(world.resource::<ProbeDispatchCounter>().count, 1);
    }
}
