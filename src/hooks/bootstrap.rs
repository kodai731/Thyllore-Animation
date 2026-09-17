use std::any::Any;

use anyhow::{anyhow, ensure, Result};

use crate::asset::AssetStorage;
use crate::ecs::world::World;

/// A subsystem's startup configuration: resolved from the command line before the window exists
/// and applied to the world once the app is constructed. The app never learns what it contains.
pub trait BootstrapOverrides: Sized + 'static {
    const NAME: &'static str;

    fn resolve(args: &[String]) -> Result<Self>;

    fn apply(&self, world: &mut World, assets: &mut AssetStorage) -> Result<()>;
}

pub type BootstrapResolveFn = fn(&[String]) -> Result<Box<dyn Any>>;
pub type BootstrapApplyFn = fn(&mut World, &mut AssetStorage, &dyn Any) -> Result<()>;

#[derive(Clone, Copy)]
pub struct BootstrapHook {
    pub name: &'static str,
    pub resolve: BootstrapResolveFn,
    pub apply: BootstrapApplyFn,
}

inventory::collect!(BootstrapHook);

pub fn resolve_erased<T: BootstrapOverrides>(args: &[String]) -> Result<Box<dyn Any>> {
    Ok(Box::new(T::resolve(args)?))
}

pub fn apply_erased<T: BootstrapOverrides>(
    world: &mut World,
    assets: &mut AssetStorage,
    overrides: &dyn Any,
) -> Result<()> {
    let overrides = overrides.downcast_ref::<T>().ok_or_else(|| {
        anyhow!(
            "bootstrap hook {} received overrides of another type",
            T::NAME
        )
    })?;
    overrides.apply(world, assets)
}

/// Registers a `BootstrapOverrides` implementation so the app resolves and applies it without
/// naming the type.
#[macro_export]
macro_rules! bootstrap_hook {
    ($overrides:ty) => {
        inventory::submit! {
            $crate::hooks::bootstrap::BootstrapHook {
                name: <$overrides as $crate::hooks::bootstrap::BootstrapOverrides>::NAME,
                resolve: $crate::hooks::bootstrap::resolve_erased::<$overrides>,
                apply: $crate::hooks::bootstrap::apply_erased::<$overrides>,
            }
        }
    };
}

/// Every bootstrap hook submitted at link time with its overrides resolved from one argument
/// list, sorted by name.
pub struct ResolvedBootstrap {
    entries: Vec<(BootstrapHook, Box<dyn Any>)>,
}

impl ResolvedBootstrap {
    pub fn resolve(args: &[String]) -> Result<Self> {
        let hooks = collect_hooks()?;

        let mut entries = Vec::with_capacity(hooks.len());
        for hook in hooks {
            entries.push((hook, (hook.resolve)(args)?));
        }
        Ok(Self { entries })
    }

    pub fn apply(&self, world: &mut World, assets: &mut AssetStorage) -> Result<()> {
        for (hook, overrides) in &self.entries {
            (hook.apply)(world, assets, overrides.as_ref())?;
        }
        Ok(())
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.entries.iter().map(|(hook, _)| hook.name).collect()
    }
}

fn collect_hooks() -> Result<Vec<BootstrapHook>> {
    let mut hooks: Vec<BootstrapHook> = inventory::iter::<BootstrapHook>
        .into_iter()
        .copied()
        .collect();
    hooks.sort_by_key(|hook| hook.name);
    for pair in hooks.windows(2) {
        ensure!(
            pair[0].name != pair[1].name,
            "bootstrap hook {} registered twice",
            pair[0].name
        );
    }
    Ok(hooks)
}

impl std::fmt::Debug for ResolvedBootstrap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedBootstrap")
            .field("names", &self.names())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ProbeOverrides {
        label: Option<String>,
    }

    struct ProbeApplied(String);

    impl BootstrapOverrides for ProbeOverrides {
        const NAME: &'static str = "probe";

        fn resolve(args: &[String]) -> Result<Self> {
            let label = args
                .iter()
                .position(|arg| arg == "--probe")
                .and_then(|position| args.get(position + 1).cloned());
            Ok(Self { label })
        }

        fn apply(&self, world: &mut World, _assets: &mut AssetStorage) -> Result<()> {
            if let Some(label) = &self.label {
                world.insert_resource(ProbeApplied(label.clone()));
            }
            Ok(())
        }
    }

    bootstrap_hook!(ProbeOverrides);

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn registered_hooks_are_resolved_and_applied_by_name() {
        let resolved = ResolvedBootstrap::resolve(&args(&["bin", "--probe", "hello"])).unwrap();
        assert!(resolved.names().contains(&"probe"));

        let mut world = World::new();
        let mut assets = AssetStorage::new();
        resolved.apply(&mut world, &mut assets).unwrap();
        assert_eq!(world.resource::<ProbeApplied>().0, "hello");
    }

    #[test]
    fn hook_without_its_flag_leaves_the_world_untouched() {
        let resolved = ResolvedBootstrap::resolve(&args(&["bin"])).unwrap();

        let mut world = World::new();
        let mut assets = AssetStorage::new();
        resolved.apply(&mut world, &mut assets).unwrap();
        assert!(world.get_resource::<ProbeApplied>().is_none());
    }
}
