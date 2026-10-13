use std::path::Path;
use std::sync::mpsc::Sender;

use serde_json::{json, Value};

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
        let job = entry_fn(job, args)?;
        self.libraries.push(lib);
        self.running.push(RunningJob { job, reply });
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset::AssetStorage;
    use crate::ecs::World;
    use crate::vulkanr::resource::graphics_resource::GraphicsResources;
    use std::sync::mpsc;

    #[derive(Debug)]
    struct DummyDoneJob {
        data: Value,
    }
    impl ScratchJob for DummyDoneJob {
        fn step(&mut self, _ctx: &mut ScratchContext) -> anyhow::Result<ScratchStep> {
            Ok(ScratchStep::Done(self.data.clone()))
        }
    }

    #[derive(Debug)]
    struct DummyContinueJob;
    impl ScratchJob for DummyContinueJob {
        fn step(&mut self, _ctx: &mut ScratchContext) -> anyhow::Result<ScratchStep> {
            Ok(ScratchStep::Continue)
        }
    }

    #[derive(Debug)]
    struct DummyErrorJob;
    impl ScratchJob for DummyErrorJob {
        fn step(&mut self, _ctx: &mut ScratchContext) -> anyhow::Result<ScratchStep> {
            Err(anyhow::anyhow!("dummy error"))
        }
    }

    fn make_ctx() -> ScratchContext<'static> {
        let world = World::new();
        let assets = AssetStorage::default();
        let graphics = Box::leak(Box::new(GraphicsResources::default()));
        ScratchContext {
            world: Box::leak(Box::new(world)),
            assets: Box::leak(Box::new(assets)),
            graphics,
        }
    }

    #[test]
    fn done_job_replies_and_removes() {
        let (tx, rx) = mpsc::channel();
        let mut host = ScratchHost::default();
        host.running.push(RunningJob {
            job: Box::new(DummyDoneJob {
                data: Value::String("hello".to_string()),
            }),
            reply: tx,
        });

        let mut ctx = make_ctx();
        host.step_running_jobs(&mut ctx);

        assert!(host.running.is_empty());
        let msg = rx.recv().unwrap();
        assert_eq!(msg["ok"], true);
        assert_eq!(msg["data"], "hello");
    }

    #[test]
    fn continue_job_stays() {
        let (tx, _rx) = mpsc::channel();
        let mut host = ScratchHost::default();
        host.running.push(RunningJob {
            job: Box::new(DummyContinueJob),
            reply: tx,
        });

        let mut ctx = make_ctx();
        host.step_running_jobs(&mut ctx);

        assert_eq!(host.running.len(), 1);
    }

    #[test]
    fn error_job_replies_false_and_removes() {
        let (tx, rx) = mpsc::channel();
        let mut host = ScratchHost::default();
        host.running.push(RunningJob {
            job: Box::new(DummyErrorJob),
            reply: tx,
        });

        let mut ctx = make_ctx();
        host.step_running_jobs(&mut ctx);

        assert!(host.running.is_empty());
        let msg = rx.recv().unwrap();
        assert_eq!(msg["ok"], false);
        assert!(msg["error"].is_string());
    }
}
