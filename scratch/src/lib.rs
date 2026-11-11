mod examples;

include!(concat!(env!("OUT_DIR"), "/local_jobs.rs"));

use thyllore_animation::scratch::job::ScratchJob;

#[no_mangle]
pub fn thyllore_scratch_entry(
    name: &str,
    args: serde_json::Value,
) -> anyhow::Result<Box<dyn ScratchJob>> {
    match name {
        "timeline" => examples::timeline::make(args),
        other => make_local_job(other, args)
            .unwrap_or_else(|| Err(anyhow::anyhow!("unknown scratch job: {}", other))),
    }
}
