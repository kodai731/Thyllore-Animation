use crate::ecs::events::{UIEvent, UIEventQueue};
use crate::ecs::resource::{BatchRun, WindBatchCapture};
use crate::ecs::systems::{unit_action_parse, BatchAction};
use crate::ecs::world::World;

/// In a batch run the dump waits for the capture frame; interactively it is the debug window's event.
#[derive(Debug, Default)]
pub struct WindDebugDump;

impl BatchAction for WindDebugDump {
    fn name(&self) -> &'static str {
        "dump_wind_debug"
    }
    fn apply(&self, world: &mut World) {
        if world.contains_resource::<BatchRun>() {
            world.insert_resource(WindBatchCapture { debug_dump: true });
            return;
        }
        world
            .resource_mut::<UIEventQueue>()
            .send(UIEvent::DumpWindDebug);
    }
}

crate::batch_action!("dump_wind_debug", unit_action_parse::<WindDebugDump>);

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::ecs::resource::CaptureSchedule;
    use crate::ecs::systems::batch_apply_debug_actions;

    #[test]
    fn dump_requests_the_capture_inside_a_batch_run() {
        let mut world = World::new();
        world.insert_resource(BatchRun::new(CaptureSchedule::single(
            PathBuf::from("/tmp/out.png"),
            1,
        )));
        world.insert_resource(UIEventQueue::new());

        batch_apply_debug_actions(&mut world, &[&WindDebugDump as &dyn BatchAction]);

        assert!(world.resource::<WindBatchCapture>().debug_dump);
        assert_eq!(world.resource_mut::<UIEventQueue>().drain().count(), 0);
    }

    #[test]
    fn dump_queues_its_event_outside_a_batch_run() {
        let mut world = World::new();
        world.insert_resource(UIEventQueue::new());

        batch_apply_debug_actions(&mut world, &[&WindDebugDump as &dyn BatchAction]);

        let events: Vec<UIEvent> = world.resource_mut::<UIEventQueue>().drain().collect();
        assert!(matches!(events[0], UIEvent::DumpWindDebug));
        assert!(world.get_resource::<WindBatchCapture>().is_none());
    }
}
