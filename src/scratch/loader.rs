use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use serde_json::{json, Value};

use crate::asset::AssetStorage;
use crate::ecs::events::UiCommand;
use crate::ecs::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

use super::job::{ScratchContext, ScratchEntryFn, ScratchJob, ScratchStep, SCRATCH_ENTRY_SYMBOL};

#[derive(Debug)]
pub struct RunningJob {
    pub job: Box<dyn ScratchJob>,
    pub reply: Sender<Value>,
}

#[derive(Debug, Default)]
pub struct ScratchHost {
    // `running` before `libraries`: Rust drops in declaration order, so jobs (which may call
    // into their library code) must drop before the libraries are dlclose'd.
    pub running: Vec<RunningJob>,
    pub libraries: Vec<libloading::Library>,
}

impl ScratchHost {
    pub fn load_job(
        &mut self,
        library_path: &Path,
        job: &str,
        args: Value,
        reply: Sender<Value>,
    ) -> anyhow::Result<()> {
        let lib = unsafe { libloading::Library::new(library_path)? };
        let entry_fn: ScratchEntryFn = unsafe {
            let symbol: libloading::Symbol<ScratchEntryFn> = lib.get(SCRATCH_ENTRY_SYMBOL)?;
            *symbol
        };
        match entry_fn(job, args) {
            Ok(job) => {
                self.libraries.push(lib);
                self.running.push(RunningJob { job, reply });
                Ok(())
            }
            Err(e) => {
                let message = format!("{e:#}");
                drop(e);
                Err(anyhow::anyhow!(message))
            }
        }
    }

    pub fn step_running_jobs(&mut self, ctx: &mut ScratchContext) {
        let mut i = 0;
        while i < self.running.len() {
            match self.running[i].job.step(ctx) {
                Ok(ScratchStep::Done(data)) => {
                    let running_job = self.running.remove(i);
                    let _ = running_job.reply.send(json!({"ok": true, "data": data}));
                }
                Ok(ScratchStep::Continue) => {
                    i += 1;
                }
                Err(e) => {
                    let running_job = self.running.remove(i);
                    let _ = running_job
                        .reply
                        .send(json!({"ok": false, "error": e.to_string()}));
                }
            }
        }
    }
}

pub fn step_scratch_jobs(
    world: &mut World,
    assets: &mut AssetStorage,
    graphics: &GraphicsResources,
) {
    let Some(mut host) = world.remove_resource::<ScratchHost>() else {
        return;
    };
    let mut ctx = ScratchContext {
        world,
        assets,
        graphics,
    };
    host.step_running_jobs(&mut ctx);
    world.insert_resource(host);
}

#[derive(Debug)]
pub struct LoadScratchJob {
    pub library: PathBuf,
    pub job: String,
    pub args: Value,
    pub reply: Sender<Value>,
}

impl UiCommand for LoadScratchJob {
    fn apply(self: Box<Self>, world: &mut World, _: &mut AssetStorage, _: &GraphicsResources) {
        let Some(mut host) = world.get_resource_mut::<ScratchHost>() else {
            let _ = self
                .reply
                .send(json!({"ok": false, "error": "ScratchHost is not registered"}));
            return;
        };
        if let Err(e) = host.load_job(&self.library, &self.job, self.args, self.reply.clone()) {
            let _ = self
                .reply
                .send(json!({"ok": false, "error": format!("{e:#}")}));
        }
    }
}

#[cfg(test)]
mod tests;
