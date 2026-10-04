use crate::asset::AssetStorage;
use crate::ecs::events::EventQueue;
use crate::ecs::world::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

/// A UI command applies itself to the world during the event dispatch phase, in the order it was sent.
pub trait UiCommand: std::fmt::Debug {
    fn apply(
        self: Box<Self>,
        world: &mut World,
        assets: &mut AssetStorage,
        graphics: &GraphicsResources,
    );
}

pub type UiCommandQueue = EventQueue<Box<dyn UiCommand>>;

pub fn apply_queued_ui_commands(
    world: &mut World,
    assets: &mut AssetStorage,
    graphics: &GraphicsResources,
) {
    let commands: Vec<Box<dyn UiCommand>> = match world.get_resource_mut::<UiCommandQueue>() {
        Some(mut queue) => queue.drain().collect(),
        None => return,
    };
    for command in commands {
        command.apply(world, assets, graphics);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct ProbeCommand(u32);

    #[derive(Default)]
    struct AppliedProbes(Vec<u32>);

    impl UiCommand for ProbeCommand {
        fn apply(self: Box<Self>, world: &mut World, _: &mut AssetStorage, _: &GraphicsResources) {
            if !world.contains_resource::<AppliedProbes>() {
                world.insert_resource(AppliedProbes::default());
            }
            world.resource_mut::<AppliedProbes>().0.push(self.0);
        }
    }

    fn applied(world: &World) -> Vec<u32> {
        world
            .get_resource::<AppliedProbes>()
            .map(|applied| applied.0.clone())
            .unwrap_or_default()
    }

    #[test]
    fn test_commands_apply_in_send_order_once() {
        let mut world = World::new();
        world.insert_resource(UiCommandQueue::default());
        let mut assets = AssetStorage::default();
        let graphics = GraphicsResources::default();
        world.send_command(ProbeCommand(1));
        world.send_command(ProbeCommand(2));

        apply_queued_ui_commands(&mut world, &mut assets, &graphics);
        apply_queued_ui_commands(&mut world, &mut assets, &graphics);

        assert_eq!(applied(&world), vec![1, 2]);
        assert!(world.resource::<UiCommandQueue>().is_empty());
    }

    #[test]
    fn test_apply_without_queue_is_a_no_op() {
        let mut world = World::new();
        let mut assets = AssetStorage::default();
        let graphics = GraphicsResources::default();

        apply_queued_ui_commands(&mut world, &mut assets, &graphics);

        assert!(applied(&world).is_empty());
    }
}
