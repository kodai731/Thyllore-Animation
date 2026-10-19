use thyllore_effect_core::FlameEffect;

use crate::ecs::resource::{FlameDebugView, FlameRenderSettings, FlameShadingMode};
use crate::ecs::systems::flame::FlameUiCommand;
use crate::ecs::systems::phases::event_dispatch::scalar_curve::ScalarCurveEvent;
use crate::ecs::World;

use super::super::param_widgets::rows_from_params;
use super::super::ui_rows::{ChoiceOption, UiRow};

const EMITTER_LABELS: [&str; 3] = ["Cylinder", "Ring", "Mesh SDF"];
const RING_EMITTER_KIND: u32 = 1;

#[derive(Clone)]
pub(super) struct FlameBodyView {
    pub effect: FlameEffect,
    pub trail_enabled: bool,
    pub trail_fade: f32,
    pub render: FlameRenderSettings,
    pub jitter: f32,
    pub jitter_freq: f32,
    pub viewport_size: [f32; 2],
    pub gpu_timings: Vec<(String, f32)>,
}

pub(super) fn emitter_rows(view: &FlameBodyView) -> Vec<UiRow<FlameBodyView>> {
    let mut rows: Vec<UiRow<FlameBodyView>> = vec![
        UiRow::Vector3 {
            label: "Position",
            speed: 0.1,
            format: "%.2f",
            get: |view| view.effect.position.into(),
            set: |view, position| {
                view.effect.position = position.into();
            },
        },
        UiRow::Choice {
            label: "Emitter",
            options: |_world, view| {
                EMITTER_LABELS
                    .iter()
                    .enumerate()
                    .map(|(kind, label)| ChoiceOption {
                        label: label.to_string(),
                        selected: view.effect.emitter.kind as usize == kind,
                    })
                    .collect()
            },
            pick: |view, index| {
                view.effect.emitter.kind = index as u32;
            },
        },
    ];

    if view.effect.emitter.kind == RING_EMITTER_KIND {
        rows.push(UiRow::Slider {
            label: "Ring Radius",
            min: 0.2,
            max: 5.0,
            format: "%.2f",
            get: |view| view.effect.emitter.ring_major_radius,
            set: |view, value| {
                view.effect.emitter.ring_major_radius = value;
            },
        });
        rows.push(UiRow::Slider {
            label: "Ring Speed",
            min: 0.0,
            max: 6.28,
            format: "%.2f",
            get: |view| view.effect.emitter.ring_angular_speed,
            set: |view, value| {
                view.effect.emitter.ring_angular_speed = value;
            },
        });
    }
    rows
}

pub(super) fn flame_param_rows() -> Vec<UiRow<FlameEffect>> {
    let mut rows = rows_from_params(
        &thyllore_effect_core::FLAME_UI_PARAMS,
        &thyllore_effect_core::FLAME_SCALAR_PARAMS,
        &[],
    );

    let advanced_rows = rows.iter_mut().find_map(|row| match row {
        UiRow::Group { title, rows, .. } if *title == "Advanced" => Some(rows),
        _ => None,
    });
    match advanced_rows {
        Some(advanced_rows) => advanced_rows.extend(manual_param_rows()),
        None => rows.push(UiRow::Group {
            title: "Advanced".into(),
            initially_open: false,
            rows: manual_param_rows(),
        }),
    }
    rows
}

fn manual_param_rows() -> Vec<UiRow<FlameEffect>> {
    vec![
        UiRow::Slider {
            label: "Noise Sharpness",
            min: 0.0,
            max: 1.0,
            format: "%.2f",
            get: |effect| {
                thyllore_effect_core::shaping_scale_to_noise_sharpness(effect.noise.shaping_scale)
            },
            set: |effect, sharpness| {
                effect.noise.shaping_scale =
                    thyllore_effect_core::noise_sharpness_to_shaping_scale(sharpness);
            },
        },
        UiRow::Tooltip(
            "Crispness of the noise pattern: log remap of the tanh shaping \
             scale, harder edges to the right (small scale saturates the \
             tanh into near-binary blobs; ~0.78 = scale 0.25, the measured \
             perceptual sweet spot). Stateless — noise_shaping_scale stays \
             the source of truth (0 = built-in 0.6)",
        ),
        UiRow::IntSlider {
            label: "Noise Segments",
            min: thyllore_effect_core::WAVE_SEGMENTS_MIN as i32,
            max: thyllore_effect_core::WAVE_SEGMENTS_MAX as i32,
            get: |effect| effect.wave_segments as i32,
            set: |effect, segments| {
                effect.wave_segments = segments as u32;
            },
        },
        UiRow::Tooltip(
            "Closed-form segments per ray: the noise grid aliases into a \
             pixel hatch above Noise Frequency ~2 at 64; 128 resolves \
             frequency ~4 at twice the cost",
        ),
        UiRow::Slider {
            label: "Vortex",
            min: 0.0,
            max: 1.0,
            format: "%.2f",
            get: |effect| {
                (effect.twist.gain / thyllore_effect_core::VORTEX_MACRO_MAX_GAIN).clamp(0.0, 1.0)
            },
            set: |effect, vortex| {
                let (gain, speed) = thyllore_effect_core::vortex_macro_parameters(vortex);
                effect.twist.gain = gain;
                effect.twist.speed = speed;
            },
        },
        UiRow::Tooltip(
            "Vortex macro: one knob writing both twist parameters along a \
             faster-and-deeper curve (stateless; the fine sliders \
             stay the source of truth)",
        ),
        UiRow::IntInput {
            label: "Branch Seed",
            get: |effect| effect.branch.seed as i32,
            set: |effect, seed| {
                effect.branch.seed = seed.max(0) as u32;
            },
        },
    ]
}

pub(super) fn body_action_rows() -> Vec<UiRow<FlameBodyView>> {
    vec![
        UiRow::Action {
            label: "Clear Flame Keys",
            enabled: |_world, _view| true,
            on_click: |_view| Box::new(ScalarCurveEvent::ClearScalarKeys),
        },
        UiRow::Action {
            label: "Random Keys (Debug)",
            enabled: |_world, _view| true,
            on_click: |_view| {
                let seed = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                Box::new(ScalarCurveEvent::InsertScalarDebugKeys { seed })
            },
        },
        UiRow::Action {
            label: "Curves",
            enabled: |_world, _view| true,
            on_click: |_view| Box::new(ScalarCurveEvent::OpenScalarCurveEditor),
        },
        UiRow::Toggle {
            label: "Trail",
            get: |view| view.trail_enabled,
            set: |view, enabled| {
                view.trail_enabled = enabled;
            },
        },
        UiRow::Slider {
            label: "Trail Fade",
            min: 0.1,
            max: 5.0,
            format: "%.2f",
            get: |view| view.trail_fade,
            set: |view, fade| {
                view.trail_fade = fade;
            },
        },
        UiRow::Canvas(draw_gpu_timings),
    ]
}

fn draw_gpu_timings(ui: &imgui::Ui, _world: &World, view: &FlameBodyView) {
    if view.gpu_timings.is_empty() {
        return;
    }
    ui.separator();
    ui.text("GPU Timings");
    for (label, ms) in &view.gpu_timings {
        ui.text(format!("  {} {:.3} ms", label, ms));
    }
}

pub(super) fn debug_rows(view: &FlameBodyView) -> Vec<UiRow<FlameBodyView>> {
    let mut rows: Vec<UiRow<FlameBodyView>> = vec![
        UiRow::Choice {
            label: "Shading Mode",
            options: |_world, view| {
                FlameShadingMode::ALL
                    .iter()
                    .map(|&mode| ChoiceOption {
                        label: mode.label().to_string(),
                        selected: mode == view.render.shading_mode,
                    })
                    .collect()
            },
            pick: |view, index| {
                if let Some(&mode) = FlameShadingMode::ALL.get(index) {
                    view.render.shading_mode = mode;
                }
            },
        },
        UiRow::Choice {
            label: "Debug View",
            options: |_world, view| {
                FlameDebugView::ALL
                    .iter()
                    .map(|&debug_view| ChoiceOption {
                        label: debug_view.label().to_string(),
                        selected: debug_view == view.render.debug_view,
                    })
                    .collect()
            },
            pick: |view, index| {
                if let Some(&debug_view) = FlameDebugView::ALL.get(index) {
                    view.render.debug_view = debug_view;
                }
            },
        },
        UiRow::Slider {
            label: "Jitter Depth",
            min: 0.0,
            max: 2.0,
            format: "%.3f",
            get: |view| view.jitter,
            set: |view, jitter| {
                view.jitter = jitter;
            },
        },
        UiRow::Slider {
            label: "Jitter Freq",
            min: 0.25,
            max: 6.0,
            format: "%.3f",
            get: |view| view.jitter_freq,
            set: |view, jitter_freq| {
                view.jitter_freq = jitter_freq;
            },
        },
    ];

    rows.extend(step_count_row(view.render.shading_mode));
    rows.push(UiRow::Action {
        label: "Dump Probe",
        enabled: |_world, _view| true,
        on_click: |view| {
            Box::new(FlameUiCommand::DumpWallProbe {
                viewport_size: view.viewport_size,
            })
        },
    });
    rows.push(UiRow::Tooltip(
        "Dump camera pose + wall-regime ray diagnostics to log/flame/",
    ));

    vec![UiRow::Group {
        title: "Flame Debug".into(),
        initially_open: false,
        rows,
    }]
}

fn step_count_row(shading_mode: FlameShadingMode) -> Option<UiRow<FlameBodyView>> {
    match shading_mode {
        FlameShadingMode::ReferenceRaymarch => Some(UiRow::IntSlider {
            label: "Reference Steps",
            min: 8,
            max: 512,
            get: |view| view.render.reference_step_count as i32,
            set: |view, steps| {
                view.render.reference_step_count = steps.max(1) as u32;
            },
        }),
        FlameShadingMode::NoiseRaymarch => Some(UiRow::IntSlider {
            label: "Noise Steps",
            min: 4,
            max: 64,
            get: |view| view.render.noise_step_count as i32,
            set: |view, steps| {
                view.render.noise_step_count = steps.max(1) as u32;
            },
        }),
        FlameShadingMode::Analytic
        | FlameShadingMode::DebugThickness
        | FlameShadingMode::DebugDepthClamp => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::platform::ui::ui_rows::visible_labels;

    fn body_view(kind: u32, shading_mode: FlameShadingMode) -> FlameBodyView {
        let mut effect = FlameEffect::default();
        effect.emitter.kind = kind;
        FlameBodyView {
            effect,
            trail_enabled: false,
            trail_fade: 0.8,
            render: FlameRenderSettings {
                shading_mode,
                ..FlameRenderSettings::default()
            },
            jitter: 1.0,
            jitter_freq: 3.0,
            viewport_size: [800.0, 600.0],
            gpu_timings: Vec::new(),
        }
    }

    fn assert_unique(labels: &[&str]) {
        let unique: HashSet<&&str> = labels.iter().collect();
        assert_eq!(unique.len(), labels.len(), "duplicate labels in {labels:?}");
    }

    fn group_rows<'a, C>(rows: &'a [UiRow<C>], group_title: &str) -> &'a [UiRow<C>] {
        rows.iter()
            .find_map(|row| match row {
                UiRow::Group { title, rows, .. } if *title == group_title => Some(rows.as_slice()),
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing group {group_title}"))
    }

    #[test]
    fn cylinder_emitter_has_no_ring_rows() {
        let rows = emitter_rows(&body_view(0, FlameShadingMode::Analytic));
        assert_eq!(visible_labels(&rows), ["Position", "Emitter"]);
    }

    #[test]
    fn ring_emitter_adds_radius_and_speed() {
        let rows = emitter_rows(&body_view(RING_EMITTER_KIND, FlameShadingMode::Analytic));
        let labels = visible_labels(&rows);
        assert_eq!(labels, ["Position", "Emitter", "Ring Radius", "Ring Speed"]);
        assert_unique(&labels);
    }

    #[test]
    fn advanced_group_ends_with_manual_params() {
        let rows = flame_param_rows();
        let advanced = visible_labels(group_rows(&rows, "Advanced"));
        assert_eq!(
            advanced[advanced.len() - 4..],
            ["Noise Sharpness", "Noise Segments", "Vortex", "Branch Seed"]
        );
        assert_unique(&advanced);
    }

    #[test]
    fn action_rows_keep_their_order() {
        let rows = body_action_rows();
        let labels = visible_labels(&rows);
        assert_eq!(
            labels,
            [
                "Clear Flame Keys",
                "Random Keys (Debug)",
                "Curves",
                "Trail",
                "Trail Fade"
            ]
        );
        assert_unique(&labels);
    }

    #[test]
    fn debug_rows_follow_the_shading_mode() {
        let cases = [
            (FlameShadingMode::Analytic, None),
            (FlameShadingMode::ReferenceRaymarch, Some("Reference Steps")),
            (FlameShadingMode::NoiseRaymarch, Some("Noise Steps")),
        ];
        for (shading_mode, step_label) in cases {
            let rows = debug_rows(&body_view(0, shading_mode));
            assert_eq!(rows.len(), 1);

            let mut expected = vec!["Shading Mode", "Debug View", "Jitter Depth", "Jitter Freq"];
            expected.extend(step_label);
            expected.push("Dump Probe");
            let debug_labels = visible_labels(group_rows(&rows, "Flame Debug"));
            assert_eq!(debug_labels, expected);
            assert_unique(&debug_labels);
        }
    }
}
