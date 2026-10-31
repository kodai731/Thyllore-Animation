use super::camera_rows::{auto_exposure_group, dof_group};
use super::onion_rows::onion_skinning_group;
use super::view::SceneSettingsView;
use crate::ecs::resource::TransformGizmoMode;
use crate::platform::ui::ui_rows::UiRow;

fn general_group() -> UiRow<SceneSettingsView> {
    UiRow::Group {
        title: "General".into(),
        initially_open: true,
        rows: vec![
            UiRow::FloatInput {
                label: "Brightness (EV)",
                format: "%.2f",
                get: |v: &SceneSettingsView| v.brightness.as_ref().map_or(0.0, |b| b.ev),
                set: |v: &mut SceneSettingsView, value: f32| {
                    if let Some(b) = v.brightness.as_mut() {
                        b.ev = value;
                    }
                },
            },
            UiRow::Tooltip("Scene View brightness on top of the exposure; +1 doubles it"),
        ],
    }
}

fn overlay_group(view: &SceneSettingsView) -> UiRow<SceneSettingsView> {
    let mut rows: Vec<UiRow<SceneSettingsView>> = Vec::new();

    if view.bones_visible.is_some() {
        rows.push(UiRow::Switch {
            label: "Show Bones",
            get: |v: &SceneSettingsView| v.bones_visible.unwrap_or(false),
            set: |v: &mut SceneSettingsView, value: bool| {
                if let Some(b) = v.bones_visible.as_mut() {
                    *b = value;
                }
            },
        });
    }

    if view.heatmap_enabled.is_some() {
        rows.push(UiRow::Switch {
            label: "Show Weight Heatmap (selected bone)",
            get: |v: &SceneSettingsView| v.heatmap_enabled.unwrap_or(false),
            set: |v: &mut SceneSettingsView, value: bool| {
                if let Some(h) = v.heatmap_enabled.as_mut() {
                    *h = value;
                }
            },
        });
    }

    UiRow::Group {
        title: "Overlay".into(),
        initially_open: true,
        rows,
    }
}

fn snap_slider(mode: TransformGizmoMode) -> UiRow<SceneSettingsView> {
    match mode {
        TransformGizmoMode::Translate => UiRow::Slider {
            label: "Snap Value",
            min: 0.01,
            max: 10.0,
            format: "%.3f",
            get: |v: &SceneSettingsView| v.gizmo.as_ref().map_or(0.5, |g| g.translate_snap_value),
            set: |v: &mut SceneSettingsView, value: f32| {
                if let Some(g) = v.gizmo.as_mut() {
                    g.translate_snap_value = value;
                }
            },
        },
        TransformGizmoMode::Rotate => UiRow::Slider {
            label: "Snap Degrees",
            min: 1.0,
            max: 90.0,
            format: "%.1f",
            get: |v: &SceneSettingsView| v.gizmo.as_ref().map_or(15.0, |g| g.rotate_snap_degrees),
            set: |v: &mut SceneSettingsView, value: f32| {
                if let Some(g) = v.gizmo.as_mut() {
                    g.rotate_snap_degrees = value;
                }
            },
        },
        TransformGizmoMode::Scale => UiRow::Slider {
            label: "Snap Value",
            min: 0.01,
            max: 1.0,
            format: "%.3f",
            get: |v: &SceneSettingsView| v.gizmo.as_ref().map_or(0.1, |g| g.scale_snap_value),
            set: |v: &mut SceneSettingsView, value: f32| {
                if let Some(g) = v.gizmo.as_mut() {
                    g.scale_snap_value = value;
                }
            },
        },
    }
}

fn transform_gizmo_group(view: &SceneSettingsView) -> UiRow<SceneSettingsView> {
    let mut rows: Vec<UiRow<SceneSettingsView>> = Vec::new();

    if let Some(gizmo) = view.gizmo.as_ref().filter(|g| g.snap_enabled) {
        rows.push(snap_slider(gizmo.mode));
    }

    rows.push(UiRow::Slider {
        label: "Gizmo Scale",
        min: 0.01,
        max: 0.3,
        format: "%.3f",
        get: |v: &SceneSettingsView| v.gizmo.as_ref().map_or(0.1, |g| g.gizmo_scale),
        set: |v: &mut SceneSettingsView, value: f32| {
            if let Some(g) = v.gizmo.as_mut() {
                g.gizmo_scale = value;
            }
        },
    });

    UiRow::Group {
        title: "Transform Gizmo".into(),
        initially_open: true,
        rows,
    }
}

pub(crate) fn scene_settings_rows(view: &SceneSettingsView) -> Vec<UiRow<SceneSettingsView>> {
    let mut rows = Vec::new();
    if view.brightness.is_some() {
        rows.push(general_group());
    }
    if view.bones_visible.is_some() || view.heatmap_enabled.is_some() {
        rows.push(overlay_group(view));
    }
    if view.gizmo.is_some() {
        rows.push(transform_gizmo_group(view));
    }
    if view.dof.is_some() {
        rows.push(dof_group(view));
    }
    if view.auto_exposure.is_some() {
        rows.push(auto_exposure_group(view));
    }
    if view.onion.is_some() {
        rows.push(onion_skinning_group());
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::resource::{
        AutoExposure, DepthOfField, OnionSkinningConfig, PhysicalCameraParameters, SceneBrightness,
        TransformGizmoState,
    };
    use crate::platform::ui::ui_rows::{find_duplicate_labels, visible_labels};

    fn full_view() -> SceneSettingsView {
        SceneSettingsView {
            brightness: Some(SceneBrightness { ev: 0.0 }),
            bones_visible: Some(true),
            heatmap_enabled: Some(false),
            gizmo: Some(TransformGizmoState {
                snap_enabled: true,
                ..TransformGizmoState::default()
            }),
            dof: Some(DepthOfField::default()),
            camera: Some(PhysicalCameraParameters::default()),
            auto_exposure: Some(AutoExposure::default()),
            exposure_readout: Some((1.0, 100.0)),
            onion: Some(OnionSkinningConfig::default()),
        }
    }

    fn empty_view() -> SceneSettingsView {
        SceneSettingsView {
            brightness: None,
            bones_visible: None,
            heatmap_enabled: None,
            gizmo: None,
            dof: None,
            camera: None,
            auto_exposure: None,
            exposure_readout: None,
            onion: None,
        }
    }

    #[test]
    fn empty_view_has_no_rows() {
        let view = empty_view();
        let rows = scene_settings_rows(&view);
        assert!(rows.is_empty(), "empty view should have no rows");
    }

    #[test]
    fn full_view_has_all_groups_in_order() {
        let view = full_view();
        let rows = scene_settings_rows(&view);
        assert_eq!(rows.len(), 6, "full view should have 6 groups");

        let titles: Vec<&str> = rows
            .iter()
            .enumerate()
            .map(|(i, row)| {
                if let UiRow::Group { title, .. } = row {
                    &**title
                } else {
                    panic!("row {} is not a Group", i);
                }
            })
            .collect();

        assert_eq!(titles[0], "General");
        assert_eq!(titles[1], "Overlay");
        assert_eq!(titles[2], "Transform Gizmo");
        assert_eq!(titles[3], "Depth of Field");
        assert_eq!(titles[4], "Auto Exposure");
        assert_eq!(titles[5], "Onion Skinning");
    }

    #[test]
    fn full_view_no_duplicate_labels() {
        let view = full_view();
        let rows = scene_settings_rows(&view);
        let duplicates: Vec<&str> = rows
            .iter()
            .flat_map(|row| match row {
                UiRow::Group { rows, .. } => find_duplicate_labels(rows),
                _ => Vec::new(),
            })
            .collect();
        assert!(
            duplicates.is_empty(),
            "full view has duplicate labels: {:?}",
            duplicates
        );
    }

    #[test]
    fn general_group_has_brightness_and_tooltip() {
        let view = full_view();
        let rows = scene_settings_rows(&view);
        let UiRow::Group {
            rows: group_rows, ..
        } = &rows[0]
        else {
            panic!("first row is not a Group");
        };
        let labels: Vec<Option<&str>> = group_rows
            .iter()
            .map(|row| match row {
                UiRow::FloatInput { label, .. } => Some(*label),
                UiRow::Tooltip(_) => None,
                _ => panic!("unexpected row type in General group"),
            })
            .collect();
        assert_eq!(labels[0], Some("Brightness (EV)"));
    }

    #[test]
    fn overlay_group_has_switches() {
        let view = full_view();
        let rows = scene_settings_rows(&view);
        let UiRow::Group {
            rows: group_rows, ..
        } = &rows[1]
        else {
            panic!("second row is not a Group");
        };
        assert_eq!(group_rows.len(), 2);
        let labels: Vec<&str> = group_rows
            .iter()
            .map(|row| {
                if let UiRow::Switch { label, .. } = row {
                    *label
                } else {
                    panic!("expected Switch in Overlay group");
                }
            })
            .collect();
        assert_eq!(labels[0], "Show Bones");
        assert_eq!(labels[1], "Show Weight Heatmap (selected bone)");
    }

    #[test]
    fn transform_gizmo_group_has_sliders() {
        let view = full_view();
        let rows = scene_settings_rows(&view);
        let UiRow::Group {
            rows: group_rows, ..
        } = &rows[2]
        else {
            panic!("third row is not a Group");
        };
        assert_eq!(group_rows.len(), 2);
        let labels: Vec<&str> = group_rows
            .iter()
            .map(|row| {
                if let UiRow::Slider { label, .. } = row {
                    *label
                } else {
                    panic!("expected Slider in Transform Gizmo group");
                }
            })
            .collect();
        assert_eq!(labels[0], "Snap Value");
        assert_eq!(labels[1], "Gizmo Scale");
    }

    #[test]
    fn dof_group_with_camera_has_all_sliders() {
        let view = full_view();
        let rows = scene_settings_rows(&view);
        let UiRow::Group {
            rows: group_rows, ..
        } = &rows[3]
        else {
            panic!("fourth row is not a Group");
        };
        assert_eq!(group_rows.len(), 5);
        let labels: Vec<&str> = group_rows
            .iter()
            .map(|row| match row {
                UiRow::Switch { label, .. } => *label,
                UiRow::Slider { label, .. } => *label,
                _ => panic!("unexpected row type in DOF group"),
            })
            .collect();
        assert_eq!(labels[0], "DOF Enabled");
        assert_eq!(labels[1], "Focus Distance");
        assert_eq!(labels[2], "Max Blur Radius");
        assert_eq!(labels[3], "Aperture (f-stops)");
        assert_eq!(labels[4], "Focal Length (mm)");
    }

    #[test]
    fn dof_group_without_camera_has_no_aperture_or_focal_length() {
        let mut view = full_view();
        view.camera = None;
        let rows = scene_settings_rows(&view);
        let UiRow::Group {
            rows: group_rows, ..
        } = &rows[3]
        else {
            panic!("fourth row is not a Group");
        };
        assert_eq!(group_rows.len(), 3);
    }

    #[test]
    fn auto_exposure_group_has_all_fields() {
        let view = full_view();
        let rows = scene_settings_rows(&view);
        let UiRow::Group {
            rows: group_rows, ..
        } = &rows[4]
        else {
            panic!("fifth row is not a Group");
        };
        assert_eq!(group_rows.len(), 10);
    }

    #[test]
    fn auto_exposure_group_without_readout_has_no_canvas() {
        let mut view = full_view();
        view.exposure_readout = None;
        let rows = scene_settings_rows(&view);
        let UiRow::Group {
            rows: group_rows, ..
        } = &rows[4]
        else {
            panic!("fifth row is not a Group");
        };
        assert_eq!(group_rows.len(), 9);
    }

    #[test]
    fn onion_skinning_group_has_all_fields() {
        let view = full_view();
        let rows = scene_settings_rows(&view);
        let UiRow::Group {
            rows: group_rows, ..
        } = &rows[5]
        else {
            panic!("sixth row is not a Group");
        };
        assert_eq!(group_rows.len(), 8);
    }

    #[test]
    fn onion_skinning_group_labels() {
        let view = full_view();
        let rows = scene_settings_rows(&view);
        let UiRow::Group {
            rows: group_rows, ..
        } = &rows[5]
        else {
            panic!("sixth row is not a Group");
        };
        let labels: Vec<&str> = visible_labels(group_rows);
        assert_eq!(labels[0], "Onion Skin Enabled");
        assert_eq!(labels[1], "Past Frames");
        assert_eq!(labels[2], "Future Frames");
        assert_eq!(labels[3], "Frame Step");
        assert_eq!(labels[4], "Ghost Opacity");
        assert_eq!(labels[5], "Past Color");
        assert_eq!(labels[6], "Future Color");
    }

    #[test]
    fn only_brightness_has_one_group() {
        let view = SceneSettingsView {
            brightness: Some(SceneBrightness { ev: 0.0 }),
            bones_visible: None,
            heatmap_enabled: None,
            gizmo: None,
            dof: None,
            camera: None,
            auto_exposure: None,
            exposure_readout: None,
            onion: None,
        };
        let rows = scene_settings_rows(&view);
        assert_eq!(rows.len(), 1);
        let UiRow::Group { title, .. } = &rows[0] else {
            panic!("row is not a Group");
        };
        assert_eq!(*title, "General");
    }
}
