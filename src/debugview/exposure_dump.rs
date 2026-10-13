use crate::app::App;
use crate::ecs::resource::{Exposure, ExposureDumpSink};

impl App {
    pub(crate) fn record_exposure_dump(&self, frame: u64, adapted: Option<f32>) {
        let Some(sink) = self.data.ecs_world.get_resource::<ExposureDumpSink>() else {
            return;
        };
        let exposure = self
            .data
            .ecs_world
            .get_resource::<Exposure>()
            .map(|e| e.clone())
            .unwrap_or_default();

        let line = serde_json::json!({
            "frame": frame,
            "adapted": adapted.unwrap_or(0.0),
            "exposure_value": exposure.exposure_value,
            "ev100": exposure.ev100,
            "metered_luminance": exposure.metered_luminance,
            "ae_enabled": adapted.is_some(),
        });
        append_jsonl(&sink.path, &format!("{line}\n"));
    }
}

fn append_jsonl(path: &str, line: &str) {
    use std::fs::OpenOptions;
    use std::io::Write;
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(line.as_bytes());
    }
}
