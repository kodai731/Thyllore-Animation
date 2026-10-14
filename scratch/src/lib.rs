mod examples;

use thyllore_animation::scratch::job::ScratchJob;

#[no_mangle]
pub fn thyllore_scratch_entry(
    name: &str,
    args: serde_json::Value,
) -> anyhow::Result<Box<dyn ScratchJob>> {
    match name {
        "timeline" => examples::timeline::make(args),
        other => Err(anyhow::anyhow!("unknown scratch job: {}", other)),
    }
}
