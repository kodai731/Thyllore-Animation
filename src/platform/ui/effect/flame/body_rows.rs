use thyllore_effect_core::flame::{FlameEffect, FlameEmitterKind};

use crate::ecs::resource::{FlameDebugView, FlameShadingMode};
use crate::ecs::systems::flame::FlameUiCommand;
use crate::ecs::systems::phases::event_dispatch::scalar_curve::ScalarCurveEvent;
use crate::ecs::world::World;

use super::section_view::FlameSectionView;
use crate::platform::ui::ui_rows::{ChoiceOption, UiRow};

fn emitter_rows() -> Vec<UiRow<FlameEffect>> {
    vec![
        UiRow::Vector3 {
            label: "Position",
            speed: 0.1,
            format: "%.2f",
            get: |effect| effect.position.into(),
            set: |effect, position| {
                effect.position = position.into();
            },
        },
        UiRow::Choice {
            label: "Emitter",
            options: |_world, effect| {
                FlameEmitterKind::ALL
                    .iter()
                    .map(|&kind| ChoiceOption {
                        label: kind.label().to_string(),
                        selected: kind == effect.emitter.kind,
                    })
                    .collect()
            },
            pick: |effect, index| {
                if let Some(&kind) = FlameEmitterKind::ALL.get(index) {
                    effect.emitter.kind = kind;
                }
            },
        },
    ]
}

pub(super) fn flame_param_rows(declared_rows: Vec<UiRow<FlameEffect>>) -> Vec<UiRow<FlameEffect>> {
    let mut rows = emitter_rows();
    rows.extend(declared_rows);
    rows.push(UiRow::Group {
        title: "Macros".into(),
        initially_open: false,
        rows: macro_rows(),
    });
    rows
}

fn macro_rows() -> Vec<UiRow<FlameEffect>> {
    vec![
        UiRow::Slider {
            label: "Noise Sharpness",
            min: 0.0,
            max: 1.0,
            format: "%.2f",
            get: |effect| {
                thyllore_effect_core::flame::shaping_scale_to_noise_sharpness(
                    effect.noise.shaping_scale,
                )
            },
            set: |effect, sharpness| {
                effect.noise.shaping_scale =
                    thyllore_effect_core::flame::noise_sharpness_to_shaping_scale(sharpness);
            },
        },
        UiRow::Tooltip(
            "Crispness of the noise pattern: log remap of the tanh shaping \
             scale, harder edges to the right (small scale saturates the \
             tanh into near-binary blobs; ~0.78 = scale 0.25, the measured \
             perceptual sweet spot). Stateless — noise_shaping_scale stays \
             the source of truth (0 = built-in 0.6)",
        ),
        UiRow::Slider {
            label: "Vortex",
            min: 0.0,
            max: 1.0,
            format: "%.2f",
            get: |effect| {
                (effect.twist.gain / thyllore_effect_core::flame::VORTEX_MACRO_MAX_GAIN)
                    .clamp(0.0, 1.0)
            },
            set: |effect, vortex| {
                let (gain, speed) = thyllore_effect_core::flame::vortex_macro_parameters(vortex);
                effect.twist.gain = gain;
                effect.twist.speed = speed;
            },
        },
        UiRow::Tooltip(
            "Vortex macro: one knob writing both twist parameters along a \
             faster-and-deeper curve (stateless; the fine sliders \
             stay the source of truth)",
        ),
    ]
}

fn key_rows() -> Vec<UiRow<FlameSectionView>> {
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
    ]
}

fn trail_rows() -> Vec<UiRow<FlameSectionView>> {
    vec![
        UiRow::Toggle {
            label: "Trail",
            get: |view| view.extra.trail_enabled,
            set: |view, enabled| {
                view.extra.trail_enabled = enabled;
            },
        },
        UiRow::Slider {
            label: "Trail Fade",
            min: 0.1,
            max: 5.0,
            format: "%.2f",
            get: |view| view.extra.trail_fade,
            set: |view, fade| {
                view.extra.trail_fade = fade;
            },
        },
    ]
}

fn shading_rows(shading_mode: FlameShadingMode) -> Vec<UiRow<FlameSectionView>> {
    let mut rows: Vec<UiRow<FlameSectionView>> = vec![
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
            get: |view| view.extra.jitter,
            set: |view, jitter| {
                view.extra.jitter = jitter;
            },
        },
        UiRow::Slider {
            label: "Jitter Freq",
            min: 0.25,
            max: 6.0,
            format: "%.3f",
            get: |view| view.extra.jitter_freq,
            set: |view, jitter_freq| {
                view.extra.jitter_freq = jitter_freq;
            },
        },
    ];
    rows.extend(step_count_row(shading_mode));
    rows
}

fn dump_rows() -> Vec<UiRow<FlameSectionView>> {
    vec![
        UiRow::Action {
            label: "Dump Probe",
            enabled: |_world, _view| true,
            on_click: |view| {
                Box::new(FlameUiCommand::DumpWallProbe {
                    viewport_size: view.extra.viewport_size,
                })
            },
        },
        UiRow::Tooltip("Dump camera pose + wall-regime ray diagnostics to log/flame/"),
        UiRow::Canvas(draw_gpu_timings),
    ]
}

fn draw_gpu_timings(ui: &imgui::Ui, _world: &World, view: &FlameSectionView) {
    if view.extra.gpu_timings.is_empty() {
        return;
    }
    ui.separator();
    ui.text("GPU Timings");
    for (label, ms) in &view.extra.gpu_timings {
        ui.text(format!("  {} {:.3} ms", label, ms));
    }
}

pub(super) fn flame_debug_rows(view: &FlameSectionView) -> Vec<UiRow<FlameSectionView>> {
    let mut rows = key_rows();
    rows.extend(trail_rows());
    rows.extend(shading_rows(view.render.shading_mode));
    rows.extend(dump_rows());
    rows
}

fn step_count_row(shading_mode: FlameShadingMode) -> Option<UiRow<FlameSectionView>> {
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
    use super::super::section_view::FlameExtra;
    use super::*;
    use crate::ecs::resource::FlameRenderSettings;
    use crate::platform::ui::ui_rows::{find_duplicate_labels, visible_labels};

    fn view_with_shading(shading_mode: FlameShadingMode) -> FlameSectionView {
        FlameSectionView {
            instances: vec!["Flame 1".to_string()],
            instance_index: 0,
            preset: None,
            selected: Some(0),
            effect: FlameEffect::default(),
            render: FlameRenderSettings {
                shading_mode,
                ..FlameRenderSettings::default()
            },
            extra: FlameExtra::default(),
        }
    }

    #[test]
    fn param_rows_wrap_the_declared_rows_between_emitter_and_macros() {
        let declared_rows = vec![UiRow::Heading("declared")];
        let rows = flame_param_rows(declared_rows);

        assert_eq!(visible_labels(&rows), ["Position", "Emitter"]);
        assert!(matches!(rows[2], UiRow::Heading("declared")));
        let Some(UiRow::Group { title, rows, .. }) = rows.last() else {
            panic!("last row is not the macro group");
        };
        assert_eq!(&**title, "Macros");
        assert_eq!(visible_labels(rows), ["Noise Sharpness", "Vortex"]);
    }

    #[test]
    fn debug_rows_follow_the_shading_mode() {
        let cases = [
            (FlameShadingMode::Analytic, None),
            (FlameShadingMode::ReferenceRaymarch, Some("Reference Steps")),
            (FlameShadingMode::NoiseRaymarch, Some("Noise Steps")),
        ];
        for (shading_mode, step_label) in cases {
            let rows = flame_debug_rows(&view_with_shading(shading_mode));

            let mut expected = vec![
                "Clear Flame Keys",
                "Random Keys (Debug)",
                "Trail",
                "Trail Fade",
                "Shading Mode",
                "Debug View",
                "Jitter Depth",
                "Jitter Freq",
            ];
            expected.extend(step_label);
            expected.push("Dump Probe");
            assert_eq!(visible_labels(&rows), expected);
            assert!(find_duplicate_labels(&rows).is_empty());
        }
    }
}
