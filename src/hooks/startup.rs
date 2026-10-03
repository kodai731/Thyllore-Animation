use crate::ecs::world::World;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StartupPhase {
    CoreResources,
    Editor,
    PostProcessing,
    Ml,
}

impl StartupPhase {
    pub fn label(self) -> &'static str {
        match self {
            StartupPhase::CoreResources => "core_resources",
            StartupPhase::Editor => "editor",
            StartupPhase::PostProcessing => "post_processing",
            StartupPhase::Ml => "ml",
        }
    }
}

pub type StartupRunFn = fn(&mut World);

#[derive(Clone, Copy)]
pub struct StartupHook {
    pub name: &'static str,
    pub phase: StartupPhase,
    pub run: StartupRunFn,
}

inventory::collect!(StartupHook);

#[macro_export]
macro_rules! startup_resource {
    ($resource:ty, $phase:ident) => {
        inventory::submit! {
            $crate::hooks::startup::StartupHook {
                name: stringify!($resource),
                phase: $crate::hooks::startup::StartupPhase::$phase,
                run: $crate::hooks::startup::insert_default::<$resource>,
            }
        }
    };
}

#[macro_export]
macro_rules! startup_hook {
    ($name:literal, $phase:ident, $run:path) => {
        inventory::submit! {
            $crate::hooks::startup::StartupHook {
                name: $name,
                phase: $crate::hooks::startup::StartupPhase::$phase,
                run: $run,
            }
        }
    };
}

pub fn insert_default<T: Default + 'static>(world: &mut World) {
    if !world.contains_resource::<T>() {
        world.insert_resource(T::default());
    }
}

pub fn run_startup_phase(world: &mut World, phase: StartupPhase) {
    let mut hooks: Vec<StartupHook> = inventory::iter::<StartupHook>
        .into_iter()
        .copied()
        .filter(|hook| hook.phase == phase)
        .collect();
    hooks.sort_by(|a, b| a.name.cmp(b.name));
    for hook in hooks {
        (hook.run)(world);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct StartupProbe {
        value: u32,
    }

    crate::startup_resource!(StartupProbe, Ml);

    #[test]
    fn test_run_startup_phase_runs_only_hooks_of_that_phase() {
        let mut world = World::new();

        run_startup_phase(&mut world, StartupPhase::PostProcessing);
        assert!(!world.contains_resource::<StartupProbe>());

        run_startup_phase(&mut world, StartupPhase::Ml);
        assert_eq!(world.get_resource::<StartupProbe>().unwrap().value, 0);
    }

    #[test]
    fn test_run_startup_phase_keeps_existing_resource() {
        let mut world = World::new();
        world.insert_resource(StartupProbe { value: 7 });

        run_startup_phase(&mut world, StartupPhase::Ml);

        assert_eq!(world.get_resource::<StartupProbe>().unwrap().value, 7);
    }
}
