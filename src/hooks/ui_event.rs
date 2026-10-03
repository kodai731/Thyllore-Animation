use crate::asset::AssetStorage;
use crate::ecs::events::EventQueue;
use crate::ecs::world::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DispatchStage {
    Early,
    Normal,
    Late,
}

pub type UiEventRunFn = fn(&mut World, &mut AssetStorage, &GraphicsResources);

pub type UiEventDispatchFn<E> = fn(Vec<E>, &mut World, &mut AssetStorage, &GraphicsResources);

#[derive(Clone, Copy)]
pub struct UiEventHook {
    pub name: &'static str,
    pub stage: DispatchStage,
    pub run: UiEventRunFn,
}

inventory::collect!(UiEventHook);

pub fn dispatch_queued_events<E: 'static>(
    world: &mut World,
    assets: &mut AssetStorage,
    graphics: &GraphicsResources,
    dispatch: UiEventDispatchFn<E>,
) {
    let events: Vec<E> = match world.get_resource_mut::<EventQueue<E>>() {
        Some(mut queue) => queue.drain().collect(),
        None => return,
    };
    if events.is_empty() {
        return;
    }
    dispatch(events, world, assets, graphics);
}

/// Registers `dispatch_fn` to receive the drained `EventQueue<E>` every event dispatch phase.
#[macro_export]
macro_rules! ui_event {
    ($event:ty => $dispatch:path, $stage:ident) => {
        inventory::submit! {
            $crate::hooks::ui_event::UiEventHook {
                name: stringify!($event),
                stage: $crate::hooks::ui_event::DispatchStage::$stage,
                run: |world, assets, graphics| {
                    $crate::hooks::ui_event::dispatch_queued_events::<$event>(
                        world, assets, graphics, $dispatch,
                    )
                },
            }
        }
    };
}

fn collect_ui_event_hooks() -> Vec<UiEventHook> {
    let mut hooks: Vec<UiEventHook> = inventory::iter::<UiEventHook>
        .into_iter()
        .copied()
        .collect();
    hooks.sort_by_key(|hook| (hook.stage, hook.name));
    hooks
}

pub fn run_ui_event_hooks(
    world: &mut World,
    assets: &mut AssetStorage,
    graphics: &GraphicsResources,
) {
    for hook in collect_ui_event_hooks() {
        (hook.run)(world, assets, graphics);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct ProbeEvent(u32);

    #[derive(Default)]
    struct ReceivedProbeEvents(Vec<Vec<u32>>);

    fn record_probe_events(
        events: Vec<ProbeEvent>,
        world: &mut World,
        _assets: &mut AssetStorage,
        _graphics: &GraphicsResources,
    ) {
        if !world.contains_resource::<ReceivedProbeEvents>() {
            world.insert_resource(ReceivedProbeEvents::default());
        }
        let values = events.into_iter().map(|event| event.0).collect();
        world.resource_mut::<ReceivedProbeEvents>().0.push(values);
    }

    crate::ui_event!(ProbeEvent => record_probe_events, Normal);

    fn received_batches(world: &World) -> Vec<Vec<u32>> {
        world
            .get_resource::<ReceivedProbeEvents>()
            .map(|received| received.0.clone())
            .unwrap_or_default()
    }

    #[test]
    fn test_registered_hook_receives_sent_events_once() {
        let mut world = World::new();
        let mut assets = AssetStorage::default();
        let graphics = GraphicsResources::default();
        world.send_event(ProbeEvent(1));
        world.send_event(ProbeEvent(2));

        run_ui_event_hooks(&mut world, &mut assets, &graphics);
        run_ui_event_hooks(&mut world, &mut assets, &graphics);

        assert_eq!(received_batches(&world), vec![vec![1, 2]]);
        assert!(world.resource::<EventQueue<ProbeEvent>>().is_empty());
    }

    #[test]
    fn test_hook_skips_dispatch_without_events() {
        let mut world = World::new();
        let mut assets = AssetStorage::default();
        let graphics = GraphicsResources::default();

        run_ui_event_hooks(&mut world, &mut assets, &graphics);
        world.insert_resource(EventQueue::<ProbeEvent>::default());
        run_ui_event_hooks(&mut world, &mut assets, &graphics);

        assert!(received_batches(&world).is_empty());
    }

    #[test]
    fn test_hooks_are_ordered_by_stage_then_name() {
        let hooks = collect_ui_event_hooks();

        let order_keys: Vec<(DispatchStage, &str)> =
            hooks.iter().map(|hook| (hook.stage, hook.name)).collect();
        let mut sorted_keys = order_keys.clone();
        sorted_keys.sort();

        assert_eq!(order_keys, sorted_keys);
        assert!(DispatchStage::Early < DispatchStage::Normal);
        assert!(DispatchStage::Normal < DispatchStage::Late);
    }
}
