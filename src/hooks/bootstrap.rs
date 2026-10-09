use std::any::Any;
use std::collections::HashMap;

use anyhow::{anyhow, ensure, Result};
use clap::{Args, Command};

use crate::asset::AssetStorage;
use crate::ecs::world::World;

/// A subsystem's startup configuration: a `clap::Args` struct whose every field is one flag,
/// resolved from the command line before the window exists and applied to the world once the
/// app is constructed. The app never learns what it contains.
pub trait BootstrapOverrides: Args + Sized + 'static {
    const NAME: &'static str;

    fn apply(&self, world: &mut World, assets: &mut AssetStorage) -> Result<()>;

    fn command() -> Command {
        Self::augment_args(
            Command::new(Self::NAME)
                .no_binary_name(true)
                .disable_help_flag(true)
                .disable_version_flag(true),
        )
    }

    /// Parses only this subsystem's flags out of `args`; tokens of other subsystems and of the
    /// engine's own hand-parsed flags are skipped.
    fn resolve(args: &[String]) -> Result<Self> {
        let mut command = Self::command();
        command.build();
        let own_tokens = select_tokens_of(&command, args);
        let matches = command.try_get_matches_from(own_tokens)?;
        Ok(Self::from_arg_matches(&matches)?)
    }
}

pub type BootstrapResolveFn = fn(&[String]) -> Result<Box<dyn Any>>;
pub type BootstrapApplyFn = fn(&mut World, &mut AssetStorage, &dyn Any) -> Result<()>;
pub type BootstrapCommandFn = fn() -> Command;

#[derive(Clone, Copy)]
pub struct BootstrapHook {
    pub name: &'static str,
    pub command: BootstrapCommandFn,
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
                command: <$overrides as $crate::hooks::bootstrap::BootstrapOverrides>::command,
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

    ensure_flags_are_unique(&hooks)?;
    Ok(hooks)
}

fn ensure_flags_are_unique(hooks: &[BootstrapHook]) -> Result<()> {
    let mut owner_by_flag: HashMap<String, &'static str> = HashMap::new();
    for hook in hooks {
        for flag in long_flags_of(&(hook.command)()) {
            if let Some(other) = owner_by_flag.insert(flag.clone(), hook.name) {
                return Err(anyhow!(
                    "flag --{flag} is declared by both bootstrap hooks {other} and {}",
                    hook.name
                ));
            }
        }
    }
    Ok(())
}

fn long_flags_of(command: &Command) -> Vec<String> {
    command
        .get_arguments()
        .filter_map(|arg| arg.get_long())
        .map(str::to_string)
        .collect()
}

/// Keeps the tokens `command` declares (`--flag`, `--flag=value`, and the values that follow a
/// `--flag`), dropping everything else so a hook can be parsed in isolation.
fn select_tokens_of(command: &Command, args: &[String]) -> Vec<String> {
    let mut selected = Vec::new();
    let mut remaining = args.iter();
    while let Some(token) = remaining.next() {
        let Some(flag) = token.strip_prefix("--") else {
            continue;
        };
        let (name, inline_value) = match flag.split_once('=') {
            Some((name, value)) => (name, Some(value)),
            None => (flag, None),
        };
        let Some(arg) = command
            .get_arguments()
            .find(|arg| arg.get_long() == Some(name))
        else {
            continue;
        };

        selected.push(token.clone());
        if inline_value.is_some() {
            continue;
        }
        let value_count = arg.get_num_args().map_or(0, |range| range.max_values());
        let values = remaining
            .clone()
            .take(value_count)
            .take_while(|value| !value.starts_with("--"));
        for value in values {
            selected.push(value.clone());
            remaining.next();
        }
    }
    selected
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

    #[derive(Args)]
    struct ProbeOverrides {
        #[arg(long = "probe")]
        label: Option<String>,
        #[arg(long = "probe-count", value_parser = clap::value_parser!(u32).range(1..))]
        count: Option<u32>,
        #[arg(long = "probe-tag")]
        tags: Vec<String>,
    }

    struct ProbeApplied(String);

    impl BootstrapOverrides for ProbeOverrides {
        const NAME: &'static str = "probe";

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

    #[test]
    fn foreign_flags_are_skipped_and_own_flags_take_both_forms() {
        let resolved = ProbeOverrides::resolve(&args(&[
            "bin",
            "--batch-screenshot",
            "/tmp/out.png",
            "--probe=hello",
            "--probe-tag",
            "a",
            "--probe-tag=b",
            "--other",
        ]))
        .unwrap();
        assert_eq!(resolved.label.as_deref(), Some("hello"));
        assert_eq!(resolved.tags, ["a", "b"]);
    }

    #[test]
    fn own_flag_errors_still_fail() {
        assert!(ProbeOverrides::resolve(&args(&["bin", "--probe"])).is_err());
        assert!(ProbeOverrides::resolve(&args(&["bin", "--probe", "--other"])).is_err());
        assert!(ProbeOverrides::resolve(&args(&["bin", "--probe-count", "0"])).is_err());
        assert!(ProbeOverrides::resolve(&args(&["bin", "--probe-count", "x"])).is_err());
    }
}
