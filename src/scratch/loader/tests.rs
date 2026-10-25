use super::*;
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

#[test]
fn load_scratch_job_replies_error_for_missing_library() {
    let mut world = World::new();
    world.insert_resource(ScratchHost::default());
    let mut assets = AssetStorage::default();
    let (reply, reply_rx) = mpsc::channel();

    Box::new(LoadScratchJob {
        library: PathBuf::from("/nonexistent/scratch.so"),
        job: "anim".to_string(),
        args: Value::Null,
        reply,
    })
    .apply(&mut world, &mut assets, &GraphicsResources::default());

    let response = reply_rx.recv().unwrap();
    assert_eq!(response["ok"], false);
    assert!(response["error"].is_string());
    assert!(world.resource::<ScratchHost>().running.is_empty());
}
