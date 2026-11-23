use crate::asset::AssetStorage;
use crate::ecs::world::World;

pub type DispatchPrepRunFn = fn(&mut World, &mut AssetStorage);

/// Per-frame work that must land in `World` before the UI commands of the frame apply (polling a worker).
#[derive(Clone, Copy)]
pub struct DispatchPrepHook {
    pub name: &'static str,
    pub run: DispatchPrepRunFn,
}

inventory::collect!(DispatchPrepHook);

/// Registers a function that runs at the start of the event dispatch phase, every frame.
#[macro_export]
macro_rules! dispatch_prep_hook {
    ($name:literal, $run:path) => {
        inventory::submit! {
            $crate::hooks::dispatch_prep::DispatchPrepHook {
                name: $name,
                run: $run,
            }
        }
    };
}

/// Every dispatch prep hook submitted at link time, sorted by name.
pub struct DispatchPrepHooks {
    entries: Vec<DispatchPrepHook>,
}

impl DispatchPrepHooks {
    pub fn collect() -> anyhow::Result<Self> {
        Self::from_hooks(
            inventory::iter::<DispatchPrepHook>
                .into_iter()
                .copied()
                .collect(),
        )
    }

    pub fn from_hooks(mut entries: Vec<DispatchPrepHook>) -> anyhow::Result<Self> {
        entries.sort_by_key(|hook| hook.name);
        for pair in entries.windows(2) {
            anyhow::ensure!(
                pair[0].name != pair[1].name,
                "dispatch prep hook {} registered twice",
                pair[0].name
            );
        }
        Ok(Self { entries })
    }

    pub fn ordered(&self) -> Vec<DispatchPrepHook> {
        self.entries.clone()
    }
}

pub fn run_dispatch_prep_hooks(world: &mut World, assets: &mut AssetStorage) {
    let hooks = match world.get_resource::<DispatchPrepHooks>() {
        Some(hooks) => hooks.ordered(),
        None => return,
    };
    for hook in hooks {
        (hook.run)(world, assets);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct ProbePrepRuns(Vec<&'static str>);

    fn record_run(world: &mut World, name: &'static str) {
        if !world.contains_resource::<ProbePrepRuns>() {
            world.insert_resource(ProbePrepRuns::default());
        }
        world.resource_mut::<ProbePrepRuns>().0.push(name);
    }

    fn record_early_run(world: &mut World, _: &mut AssetStorage) {
        record_run(world, "early");
    }

    fn record_late_run(world: &mut World, _: &mut AssetStorage) {
        record_run(world, "late");
    }

    const EARLY_PROBE: DispatchPrepHook = DispatchPrepHook {
        name: "a_probe",
        run: record_early_run,
    };
    const LATE_PROBE: DispatchPrepHook = DispatchPrepHook {
        name: "b_probe",
        run: record_late_run,
    };

    crate::dispatch_prep_hook!("probe_prep", record_early_run);

    #[test]
    fn test_registered_hooks_are_collected_sorted_by_name() {
        let hooks = DispatchPrepHooks::collect().expect("dispatch prep hooks collected");
        let names: Vec<&str> = hooks.ordered().iter().map(|hook| hook.name).collect();
        let mut sorted = names.clone();
        sorted.sort();

        assert_eq!(names, sorted);
        assert!(names.contains(&"probe_prep"));
    }

    #[test]
    fn test_hooks_run_in_name_order_once_per_call() {
        let hooks = DispatchPrepHooks::from_hooks(vec![LATE_PROBE, EARLY_PROBE]).expect("hooks");
        let mut world = World::new();
        world.insert_resource(hooks);
        let mut assets = AssetStorage::new();

        run_dispatch_prep_hooks(&mut world, &mut assets);
        run_dispatch_prep_hooks(&mut world, &mut assets);

        assert_eq!(
            world.resource::<ProbePrepRuns>().0,
            vec!["early", "late", "early", "late"]
        );
    }

    #[test]
    fn test_hook_registered_twice_is_refused() {
        assert!(DispatchPrepHooks::from_hooks(vec![EARLY_PROBE, EARLY_PROBE]).is_err());
    }
}
