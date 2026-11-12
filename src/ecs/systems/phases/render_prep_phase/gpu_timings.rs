use crate::ecs::resource::{GpuPassTimings, GpuTimingsSink};

pub(super) fn gpu_timings_write(world: &mut crate::ecs::World) {
    let timings = match world.get_resource::<GpuPassTimings>() {
        Some(t) => t,
        None => return,
    };
    let frame = timings.frame;
    let frame_total_ms = timings.frame_total_ms;
    let passes: Vec<(String, f32)> = timings.passes.clone();
    if passes.is_empty() {
        return;
    }
    let mut sink = match world.get_resource_mut::<GpuTimingsSink>() {
        Some(s) => s,
        None => return,
    };
    if frame == sink.last_frame {
        return;
    }
    let passes_map: serde_json::Map<String, serde_json::Value> = passes
        .into_iter()
        .map(|(label, ms)| (label, serde_json::json!(ms)))
        .collect();

    let mut obj: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
    obj.insert("frame".to_string(), serde_json::json!(frame));
    obj.insert("passes".to_string(), serde_json::Value::Object(passes_map));
    if let Some(ms) = frame_total_ms {
        obj.insert("frame_total_ms".to_string(), serde_json::json!(ms));
    }

    if let Some(cpu) = world.get_resource::<crate::ecs::resource::CpuFrameTimings>() {
        obj.insert("cpu_dt_ms".to_string(), serde_json::json!(cpu.dt_ms));
        obj.insert("imgui_vtx".to_string(), serde_json::json!(cpu.imgui_vtx));
        obj.insert("imgui_idx".to_string(), serde_json::json!(cpu.imgui_idx));
        let cpu_map: serde_json::Map<String, serde_json::Value> = cpu
            .stages
            .iter()
            .map(|(label, ms)| (label.clone(), serde_json::json!(ms)))
            .collect();
        obj.insert("cpu".to_string(), serde_json::Value::Object(cpu_map));
    }
    if let Some(up) = world.get_resource::<crate::ecs::resource::UpdatePhaseTimings>() {
        let up_map: serde_json::Map<String, serde_json::Value> = up
            .stages
            .iter()
            .map(|(label, ms)| (label.clone(), serde_json::json!(ms)))
            .collect();
        obj.insert(
            "update_phases".to_string(),
            serde_json::Value::Object(up_map),
        );
    }
    if let Some(phase_timings) = world.get_resource::<crate::ecs::resource::PhaseSubTimings>() {
        for (phase_name, timings) in &phase_timings.phases {
            let map: serde_json::Map<String, serde_json::Value> = timings
                .iter()
                .map(|(label, ms)| (label.clone(), serde_json::json!(ms)))
                .collect();
            obj.insert(
                format!("{}_sub", phase_name),
                serde_json::Value::Object(map),
            );
        }
    }

    let line = serde_json::Value::Object(obj);
    if let Err(e) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&sink.path)
        .and_then(|mut f| {
            use std::io::Write;
            writeln!(f, "{}", line)
        })
    {
        eprintln!("gpu timings write failed: {}", e);
    }
    sink.last_frame = frame;
}
