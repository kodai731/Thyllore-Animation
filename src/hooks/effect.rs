use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use crate::app::AppData;
use crate::ecs::EffectContext;
use crate::hooks::pass::{PassGraph, RenderPassNode};
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::render::RRRender;

pub type EffectSetupHook = unsafe fn(&Instance, &RRDevice, &mut AppData, &RRRender) -> Result<()>;
pub type EffectHookFn = unsafe fn(&mut EffectContext) -> Result<()>;

#[derive(Clone, Copy)]
pub struct EffectHook {
    pub name: &'static str,
    pub setup: Option<EffectSetupHook>,
    pub on_viewport_resize: Option<EffectHookFn>,
    pub destroy: Option<EffectHookFn>,
    pub passes: &'static [&'static dyn RenderPassNode],
}

#[derive(Default)]
pub struct EffectHooks {
    entries: Vec<EffectHook>,
}

impl std::fmt::Debug for EffectHooks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EffectHooks")
            .field("entries", &self.names())
            .finish()
    }
}

impl EffectHooks {
    pub fn register(&mut self, hook: EffectHook) {
        match self
            .entries
            .iter_mut()
            .find(|entry| entry.name == hook.name)
        {
            Some(entry) => *entry = hook,
            None => self.entries.push(hook),
        }
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.entries.iter().map(|entry| entry.name).collect()
    }

    pub fn register_passes(&self, graph: &mut PassGraph) {
        for hook in &self.entries {
            graph.register_all(hook.passes);
        }
    }

    pub(crate) fn snapshot(&self) -> Vec<EffectHook> {
        self.entries.clone()
    }

    pub unsafe fn run_setup(
        instance: &Instance,
        rrdevice: &RRDevice,
        data: &mut AppData,
        rrrender: &RRRender,
    ) -> Result<()> {
        for hook in data.effect_hooks.snapshot() {
            if let Some(setup) = hook.setup {
                setup(instance, rrdevice, data, rrrender)?;
            }
        }
        Ok(())
    }
}
