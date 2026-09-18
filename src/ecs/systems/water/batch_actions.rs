use crate::ecs::events::{UIEvent, UIEventQueue};
use crate::ecs::resource::BatchDumpPlan;
use crate::ecs::systems::{unit_action_parse, BatchAction};
use crate::ecs::world::World;

/// In a batch run the dump waits for the screenshot frame; interactively it is the debug window's event.
#[derive(Debug, Default)]
pub struct WaterDebugDump;

impl BatchAction for WaterDebugDump {
    fn name(&self) -> &'static str {
        "dump_water_debug"
    }
    fn apply(&self, world: &World) {
        if let Some(mut plan) = world.get_resource_mut::<BatchDumpPlan>() {
            plan.dump_water_debug = true;
            return;
        }
        world
            .resource_mut::<UIEventQueue>()
            .send(UIEvent::DumpWaterDebug);
    }
}

crate::batch_action!("dump_water_debug", unit_action_parse::<WaterDebugDump>);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::systems::batch_apply_debug_actions;

    #[test]
    fn dump_marks_the_plan_inside_a_batch_run() {
        let mut world = World::new();
        world.insert_resource(BatchDumpPlan::default());
        world.insert_resource(UIEventQueue::new());

        batch_apply_debug_actions(&world, &[&WaterDebugDump as &dyn BatchAction]);

        assert!(world.resource::<BatchDumpPlan>().dump_water_debug);
        assert_eq!(world.resource_mut::<UIEventQueue>().drain().count(), 0);
    }

    #[test]
    fn dump_queues_its_event_outside_a_batch_run() {
        let mut world = World::new();
        world.insert_resource(UIEventQueue::new());

        batch_apply_debug_actions(&world, &[&WaterDebugDump as &dyn BatchAction]);

        let events: Vec<UIEvent> = world.resource_mut::<UIEventQueue>().drain().collect();
        assert!(matches!(events[0], UIEvent::DumpWaterDebug));
    }
}
