use crate::ecs::resource::{ComputeStageTimings, CpuFrameTimings, WaitStageTimings};

pub fn sum_compute_ms(compute: &ComputeStageTimings) -> f32 {
    let ComputeStageTimings {
        imgui_build_ms,
        event_dispatch_ms,
        frame_begin_ms,
        update_ms,
        render_cpu_ms,
        last_phase_ms,
    } = *compute;

    imgui_build_ms + event_dispatch_ms + frame_begin_ms + update_ms + render_cpu_ms + last_phase_ms
}

pub fn sum_wait_ms(wait: &WaitStageTimings) -> f32 {
    let WaitStageTimings {
        gpu_wait_ms,
        present_ms,
        device_idle_ms,
    } = *wait;

    gpu_wait_ms + present_ms + device_idle_ms
}

pub fn label_stage_timings(timings: &CpuFrameTimings) -> [(&'static str, f32); 9] {
    let ComputeStageTimings {
        imgui_build_ms,
        event_dispatch_ms,
        frame_begin_ms,
        update_ms,
        render_cpu_ms,
        last_phase_ms,
    } = timings.compute;
    let WaitStageTimings {
        gpu_wait_ms,
        present_ms,
        device_idle_ms,
    } = timings.wait;

    [
        ("imgui_build", imgui_build_ms),
        ("event_dispatch", event_dispatch_ms),
        ("frame_begin", frame_begin_ms),
        ("gpu_wait", gpu_wait_ms),
        ("update", update_ms),
        ("render_cpu", render_cpu_ms),
        ("present", present_ms),
        ("last_phase", last_phase_ms),
        ("device_idle", device_idle_ms),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_timings() -> CpuFrameTimings {
        CpuFrameTimings {
            compute: ComputeStageTimings {
                imgui_build_ms: 1.0,
                event_dispatch_ms: 0.5,
                frame_begin_ms: 0.25,
                update_ms: 2.0,
                render_cpu_ms: 3.0,
                last_phase_ms: 0.5,
            },
            wait: WaitStageTimings {
                gpu_wait_ms: 4.0,
                present_ms: 8.0,
                device_idle_ms: 16.0,
            },
            ..Default::default()
        }
    }

    #[test]
    fn test_sum_compute_ms_excludes_wait_stages() {
        assert!((sum_compute_ms(&sample_timings().compute) - 7.25).abs() < 1e-6);
    }

    #[test]
    fn test_sum_wait_ms_excludes_compute_stages() {
        assert!((sum_wait_ms(&sample_timings().wait) - 28.0).abs() < 1e-6);
    }

    #[test]
    fn test_sums_of_default_are_zero() {
        let timings = CpuFrameTimings::default();
        assert_eq!(sum_compute_ms(&timings.compute), 0.0);
        assert_eq!(sum_wait_ms(&timings.wait), 0.0);
    }

    #[test]
    fn test_labelled_stages_cover_every_stage_once() {
        let timings = sample_timings();
        let labelled = label_stage_timings(&timings);

        let labelled_total_ms: f32 = labelled.iter().map(|(_, ms)| ms).sum();
        let expected_total_ms = sum_compute_ms(&timings.compute) + sum_wait_ms(&timings.wait);
        assert!((labelled_total_ms - expected_total_ms).abs() < 1e-6);

        let unique_labels: std::collections::HashSet<_> =
            labelled.iter().map(|(label, _)| *label).collect();
        assert_eq!(unique_labels.len(), labelled.len());
    }
}
