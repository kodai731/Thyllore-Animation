use super::view::SceneSettingsView;
use crate::platform::ui::ui_rows::UiRow;
use thyllore_render_core::ExposureMetering;

pub(super) fn dof_group(view: &SceneSettingsView) -> UiRow<SceneSettingsView> {
    let mut rows: Vec<UiRow<SceneSettingsView>> = Vec::new();

    rows.push(UiRow::Switch {
        label: "DOF Enabled",
        get: |v: &SceneSettingsView| v.dof.as_ref().map_or(false, |d| d.enabled),
        set: |v: &mut SceneSettingsView, value: bool| {
            if let Some(d) = v.dof.as_mut() {
                d.enabled = value;
            }
        },
    });

    rows.push(UiRow::Slider {
        label: "Focus Distance",
        min: 0.1,
        max: 100.0,
        format: "%.3f",
        get: |v: &SceneSettingsView| v.dof.as_ref().map_or(10.0, |d| d.focus_distance),
        set: |v: &mut SceneSettingsView, value: f32| {
            if let Some(d) = v.dof.as_mut() {
                d.focus_distance = value;
            }
        },
    });

    rows.push(UiRow::Slider {
        label: "Max Blur Radius",
        min: 1.0,
        max: 32.0,
        format: "%.3f",
        get: |v: &SceneSettingsView| v.dof.as_ref().map_or(8.0, |d| d.max_blur_radius),
        set: |v: &mut SceneSettingsView, value: f32| {
            if let Some(d) = v.dof.as_mut() {
                d.max_blur_radius = value;
            }
        },
    });

    if view.camera.is_some() {
        rows.push(UiRow::Slider {
            label: "Aperture (f-stops)",
            min: 1.0,
            max: 22.0,
            format: "%.3f",
            get: |v: &SceneSettingsView| v.camera.as_ref().map_or(16.0, |c| c.aperture_f_stops),
            set: |v: &mut SceneSettingsView, value: f32| {
                if let Some(c) = v.camera.as_mut() {
                    c.aperture_f_stops = value;
                }
            },
        });

        rows.push(UiRow::Slider {
            label: "Focal Length (mm)",
            min: 10.0,
            max: 200.0,
            format: "%.3f",
            get: |v: &SceneSettingsView| v.camera.as_ref().map_or(35.0, |c| c.focal_length_mm),
            set: |v: &mut SceneSettingsView, value: f32| {
                if let Some(c) = v.camera.as_mut() {
                    c.focal_length_mm = value;
                }
            },
        });
    }

    UiRow::Group {
        title: "Depth of Field".into(),
        initially_open: false,
        rows,
    }
}

pub(super) fn auto_exposure_group(view: &SceneSettingsView) -> UiRow<SceneSettingsView> {
    let mut rows: Vec<UiRow<SceneSettingsView>> = Vec::new();

    rows.push(UiRow::Switch {
        label: "Auto Exposure Enabled",
        get: |v: &SceneSettingsView| v.auto_exposure.as_ref().map_or(false, |a| a.enabled),
        set: |v: &mut SceneSettingsView, value: bool| {
            if let Some(a) = v.auto_exposure.as_mut() {
                a.enabled = value;
            }
        },
    });

    rows.push(UiRow::Choice {
        label: "Metering",
        options: |_world, v: &SceneSettingsView| {
            let current = v
                .auto_exposure
                .as_ref()
                .map_or(ExposureMetering::Light, |a| a.metering);
            ExposureMetering::ALL
                .iter()
                .map(|m| crate::platform::ui::ui_rows::ChoiceOption {
                    label: m.name().to_string(),
                    selected: *m == current,
                })
                .collect()
        },
        pick: |v: &mut SceneSettingsView, index: usize| {
            if let Some(a) = v.auto_exposure.as_mut() {
                a.metering = ExposureMetering::ALL[index];
            }
        },
    });

    rows.push(UiRow::Slider {
        label: "White Level",
        min: 0.05,
        max: 1.0,
        format: "%.3f",
        get: |v: &SceneSettingsView| v.auto_exposure.as_ref().map_or(0.36, |a| a.white_level),
        set: |v: &mut SceneSettingsView, value: f32| {
            if let Some(a) = v.auto_exposure.as_mut() {
                a.white_level = value;
            }
        },
    });

    rows.push(UiRow::Slider {
        label: "Min EV",
        min: -10.0,
        max: 10.0,
        format: "%.3f",
        get: |v: &SceneSettingsView| v.auto_exposure.as_ref().map_or(-1.0, |a| a.min_ev),
        set: |v: &mut SceneSettingsView, value: f32| {
            if let Some(a) = v.auto_exposure.as_mut() {
                a.min_ev = value;
            }
        },
    });

    rows.push(UiRow::Slider {
        label: "Max EV",
        min: 0.0,
        max: 30.0,
        format: "%.3f",
        get: |v: &SceneSettingsView| v.auto_exposure.as_ref().map_or(14.0, |a| a.max_ev),
        set: |v: &mut SceneSettingsView, value: f32| {
            if let Some(a) = v.auto_exposure.as_mut() {
                a.max_ev = value;
            }
        },
    });

    rows.push(UiRow::Slider {
        label: "Speed Up",
        min: 0.1,
        max: 10.0,
        format: "%.3f",
        get: |v: &SceneSettingsView| {
            v.auto_exposure
                .as_ref()
                .map_or(3.0, |a| a.adaptation_speed_up)
        },
        set: |v: &mut SceneSettingsView, value: f32| {
            if let Some(a) = v.auto_exposure.as_mut() {
                a.adaptation_speed_up = value;
            }
        },
    });

    rows.push(UiRow::Slider {
        label: "Speed Down",
        min: 0.1,
        max: 10.0,
        format: "%.3f",
        get: |v: &SceneSettingsView| {
            v.auto_exposure
                .as_ref()
                .map_or(1.0, |a| a.adaptation_speed_down)
        },
        set: |v: &mut SceneSettingsView, value: f32| {
            if let Some(a) = v.auto_exposure.as_mut() {
                a.adaptation_speed_down = value;
            }
        },
    });

    rows.push(UiRow::Slider {
        label: "Low Percent",
        min: 0.0,
        max: 0.5,
        format: "%.3f",
        get: |v: &SceneSettingsView| v.auto_exposure.as_ref().map_or(0.4, |a| a.low_percent),
        set: |v: &mut SceneSettingsView, value: f32| {
            if let Some(a) = v.auto_exposure.as_mut() {
                a.low_percent = value;
            }
        },
    });

    rows.push(UiRow::Slider {
        label: "High Percent",
        min: 0.5,
        max: 1.0,
        format: "%.3f",
        get: |v: &SceneSettingsView| v.auto_exposure.as_ref().map_or(0.9, |a| a.high_percent),
        set: |v: &mut SceneSettingsView, value: f32| {
            if let Some(a) = v.auto_exposure.as_mut() {
                a.high_percent = value;
            }
        },
    });

    if view.exposure_readout.is_some() {
        rows.push(UiRow::Canvas(|ui, _world, v: &SceneSettingsView| {
            if let Some((exposure, ev100)) = v.exposure_readout {
                ui.text(format!("Current Exposure: {:.4}", exposure));
                ui.text(format!("Current EV100: {:.2}", ev100));
            }
        }));
    }

    UiRow::Group {
        title: "Auto Exposure".into(),
        initially_open: false,
        rows,
    }
}
