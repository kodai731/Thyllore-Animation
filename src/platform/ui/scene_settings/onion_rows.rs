use super::view::SceneSettingsView;
use crate::platform::ui::ui_rows::UiRow;

pub(super) fn onion_skinning_group() -> UiRow<SceneSettingsView> {
    let mut rows: Vec<UiRow<SceneSettingsView>> = Vec::new();

    rows.push(UiRow::Switch {
        label: "Onion Skin Enabled",
        get: |v: &SceneSettingsView| v.onion.as_ref().map_or(false, |o| o.enabled),
        set: |v: &mut SceneSettingsView, value: bool| {
            if let Some(o) = v.onion.as_mut() {
                o.enabled = value;
            }
        },
    });

    rows.push(UiRow::IntSlider {
        label: "Past Frames",
        min: 0,
        max: 4,
        get: |v: &SceneSettingsView| v.onion.as_ref().map_or(2, |o| o.past_count as i32),
        set: |v: &mut SceneSettingsView, value: i32| {
            if let Some(o) = v.onion.as_mut() {
                o.past_count = value.max(0) as u32;
            }
        },
    });

    rows.push(UiRow::IntSlider {
        label: "Future Frames",
        min: 0,
        max: 4,
        get: |v: &SceneSettingsView| v.onion.as_ref().map_or(2, |o| o.future_count as i32),
        set: |v: &mut SceneSettingsView, value: i32| {
            if let Some(o) = v.onion.as_mut() {
                o.future_count = value.max(0) as u32;
            }
        },
    });

    rows.push(UiRow::Slider {
        label: "Frame Step",
        min: 0.001,
        max: 0.2,
        format: "%.3f",
        get: |v: &SceneSettingsView| v.onion.as_ref().map_or(1.0 / 30.0, |o| o.frame_step),
        set: |v: &mut SceneSettingsView, value: f32| {
            if let Some(o) = v.onion.as_mut() {
                o.frame_step = value;
            }
        },
    });

    rows.push(UiRow::Slider {
        label: "Ghost Opacity",
        min: 0.0,
        max: 1.0,
        format: "%.3f",
        get: |v: &SceneSettingsView| v.onion.as_ref().map_or(0.2, |o| o.opacity),
        set: |v: &mut SceneSettingsView, value: f32| {
            if let Some(o) = v.onion.as_mut() {
                o.opacity = value;
            }
        },
    });

    rows.push(UiRow::Color3 {
        label: "Past Color",
        get: |v: &SceneSettingsView| v.onion.as_ref().map_or([0.2, 0.4, 1.0], |o| o.past_color),
        set: |v: &mut SceneSettingsView, value: [f32; 3]| {
            if let Some(o) = v.onion.as_mut() {
                o.past_color = value;
            }
        },
    });

    rows.push(UiRow::Color3 {
        label: "Future Color",
        get: |v: &SceneSettingsView| v.onion.as_ref().map_or([1.0, 0.4, 0.2], |o| o.future_color),
        set: |v: &mut SceneSettingsView, value: [f32; 3]| {
            if let Some(o) = v.onion.as_mut() {
                o.future_color = value;
            }
        },
    });

    rows.push(UiRow::Canvas(|ui, _world, v: &SceneSettingsView| {
        if let Some(o) = v.onion.as_ref() {
            ui.text(format!(
                "Total ghosts: {}",
                crate::ecs::compute_total_ghost_count(o)
            ));
        }
    }));

    UiRow::Group {
        title: "Onion Skinning".into(),
        initially_open: false,
        rows,
    }
}
