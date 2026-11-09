use thyllore_animation::ecs::resource::{ClipLibrary, TimelineState};
use thyllore_animation::scratch::job::{ScratchContext, ScratchJob, ScratchStep};

pub fn make(_args: serde_json::Value) -> anyhow::Result<Box<dyn ScratchJob>> {
    Ok(Box::new(TimelineJob))
}

#[derive(Debug)]
struct TimelineJob;

impl ScratchJob for TimelineJob {
    fn step(&mut self, ctx: &mut ScratchContext) -> anyhow::Result<ScratchStep> {
        let timeline = ctx.world.get_resource::<TimelineState>();
        let current_time = timeline.as_ref().map(|t| t.current_time).unwrap_or(-1.0);
        let playing = timeline.as_ref().map(|t| t.playing).unwrap_or(false);

        let clip_count = ctx
            .world
            .get_resource::<ClipLibrary>()
            .map(|lib| lib.all_clip_ids().count())
            .unwrap_or(0);

        let data = serde_json::json!({
            "current_time": current_time,
            "playing": playing,
            "clip_count": clip_count,
        });

        Ok(ScratchStep::Done(data))
    }
}
