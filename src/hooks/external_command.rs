use winit::event_loop::EventLoopProxy;

use crate::ecs::events::UiCommand;

/// A command produced outside the main thread (a socket listener, a watcher) that the event loop
/// applies through the ordinary `UiCommandQueue` once it is woken up.
pub type ExternalCommand = Box<dyn UiCommand + Send>;

/// Wakes the event loop with a command; cloned into every source thread.
#[derive(Clone)]
pub struct ExternalCommandSender {
    proxy: EventLoopProxy<ExternalCommand>,
}

impl ExternalCommandSender {
    pub fn new(proxy: EventLoopProxy<ExternalCommand>) -> Self {
        Self { proxy }
    }

    pub fn send(&self, command: ExternalCommand) -> anyhow::Result<()> {
        self.proxy
            .send_event(command)
            .map_err(|_| anyhow::anyhow!("event loop is closed"))
    }
}

pub type ExternalCommandSourceStartFn = fn(ExternalCommandSender) -> anyhow::Result<()>;

#[derive(Clone, Copy)]
pub struct ExternalCommandSourceHook {
    pub name: &'static str,
    pub start: ExternalCommandSourceStartFn,
}

inventory::collect!(ExternalCommandSourceHook);

/// Registers a source that runs for the whole session and pushes commands into the event loop.
#[macro_export]
macro_rules! external_command_source {
    ($name:literal, $start:path) => {
        inventory::submit! {
            $crate::hooks::external_command::ExternalCommandSourceHook {
                name: $name,
                start: $start,
            }
        }
    };
}

/// Starts every registered source once, in name order; a source that fails to start is logged and
/// skipped so the engine still runs.
pub fn start_external_command_sources(sender: &ExternalCommandSender) {
    let mut hooks: Vec<ExternalCommandSourceHook> = inventory::iter::<ExternalCommandSourceHook>
        .into_iter()
        .copied()
        .collect();
    hooks.sort_by_key(|hook| hook.name);
    for hook in hooks {
        if let Err(error) = (hook.start)(sender.clone()) {
            log_warn!(
                "external command source {} not started: {error:#}",
                hook.name
            );
        }
    }
}
