use crate::ecs::world::World;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StartupPhase {
    CoreResources,
    Editor,
    PostProcessing,
    Ml,
}

pub type StartupNameFn = fn() -> &'static str;
pub type StartupRunFn = fn(&mut World);

#[derive(Clone, Copy)]
pub struct StartupHook {
    pub name: StartupNameFn,
    pub phase: StartupPhase,
    pub run: StartupRunFn,
}

inventory::collect!(StartupHook);

#[macro_export]
macro_rules! startup_resource {
    ($resource:ty, $phase:ident) => {
        inventory::submit! {
            $crate::hooks::startup::StartupHook::resource::<$resource>(
                $crate::hooks::startup::StartupPhase::$phase,
            )
        }
    };
}

#[macro_export]
macro_rules! startup_hook {
    ($name:literal, $phase:ident, $run:path) => {
        inventory::submit! {
            $crate::hooks::startup::StartupHook {
                name: || $name,
                phase: $crate::hooks::startup::StartupPhase::$phase,
                run: $run,
            }
        }
    };
}

impl StartupHook {
    pub const fn resource<T: Default + 'static>(phase: StartupPhase) -> Self {
        Self {
            name: std::any::type_name::<T>,
            phase,
            run: insert_default::<T>,
        }
    }
}

fn insert_default<T: Default + 'static>(world: &mut World) {
    if !world.contains_resource::<T>() {
        world.insert_resource(T::default());
    }
}

/// Every startup hook submitted at link time, sorted by name.
pub struct StartupHooks {
    entries: Vec<StartupHook>,
}

impl StartupHooks {
    pub fn collect() -> anyhow::Result<Self> {
        Self::from_hooks(
            inventory::iter::<StartupHook>
                .into_iter()
                .copied()
                .collect(),
        )
    }

    pub fn from_hooks(mut entries: Vec<StartupHook>) -> anyhow::Result<Self> {
        entries.sort_by_key(|hook| (hook.name)());
        for pair in entries.windows(2) {
            anyhow::ensure!(
                (pair[0].name)() != (pair[1].name)(),
                "startup hook {} registered twice",
                (pair[0].name)()
            );
        }
        Ok(Self { entries })
    }

    pub fn run_phase(&self, world: &mut World, phase: StartupPhase) {
        for hook in self.entries.iter().filter(|hook| hook.phase == phase) {
            (hook.run)(world);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct StartupProbe {
        value: u32,
    }

    fn probe_hooks() -> StartupHooks {
        StartupHooks::from_hooks(vec![StartupHook::resource::<StartupProbe>(
            StartupPhase::Ml,
        )])
        .unwrap()
    }

    #[test]
    fn test_run_phase_runs_only_hooks_of_that_phase() {
        let hooks = probe_hooks();
        let mut world = World::new();

        hooks.run_phase(&mut world, StartupPhase::PostProcessing);
        assert!(!world.contains_resource::<StartupProbe>());

        hooks.run_phase(&mut world, StartupPhase::Ml);
        assert_eq!(world.get_resource::<StartupProbe>().unwrap().value, 0);
    }

    #[test]
    fn test_run_phase_keeps_existing_resource() {
        let hooks = probe_hooks();
        let mut world = World::new();
        world.insert_resource(StartupProbe { value: 7 });

        hooks.run_phase(&mut world, StartupPhase::Ml);

        assert_eq!(world.get_resource::<StartupProbe>().unwrap().value, 7);
    }

    #[test]
    fn test_from_hooks_rejects_a_name_registered_twice() {
        let duplicated = vec![
            StartupHook::resource::<StartupProbe>(StartupPhase::Editor),
            StartupHook::resource::<StartupProbe>(StartupPhase::Ml),
        ];

        assert!(StartupHooks::from_hooks(duplicated).is_err());
    }

    #[test]
    fn test_collect_accepts_the_registered_hooks() {
        assert!(StartupHooks::collect().is_ok());
    }
}
