use crate::ecs::systems::flame::{FLAMES_STYLE_DIR, FLAMES_TEXTURE_DIR};

use super::fit_browser::canonical_dir_or;
use super::section_view::{draw_fit_path_status, FlameSectionView};
use crate::platform::ui::ui_rows::{ChoiceOption, UiRow};

pub(super) fn flame_extra_head_rows(view: &FlameSectionView) -> Vec<UiRow<FlameSectionView>> {
    let mut rows = fit_rows(view);
    rows.push(UiRow::Canvas(|ui, _, view: &FlameSectionView| {
        draw_fit_path_status(ui, &view.extra.flame_ui)
    }));
    rows.extend(style_rows(view));
    rows
}

fn fit_rows(view: &FlameSectionView) -> Vec<UiRow<FlameSectionView>> {
    let mut rows: Vec<UiRow<FlameSectionView>> = vec![UiRow::Heading("Texture Fit")];

    rows.extend(fit_source_rows());
    rows.extend(fit_group_rows(view));
    rows.extend(fit_apply_rows());
    rows
}

fn fit_source_rows() -> Vec<UiRow<FlameSectionView>> {
    let mut rows: Vec<UiRow<FlameSectionView>> = Vec::new();

    rows.push(UiRow::Choice {
        label: "Fit Texture",
        options: |_world, view| {
            let mut items = vec![ChoiceOption {
                label: "(custom path)".to_string(),
                selected: view.extra.flame_ui.texture_fit_path.is_empty(),
            }];
            for name in &view.extra.flame_ui.texture_fit_scan {
                let expected_path = format!("{}/{}", FLAMES_TEXTURE_DIR, name);
                items.push(ChoiceOption {
                    label: name.clone(),
                    selected: view.extra.flame_ui.texture_fit_path == expected_path,
                });
            }
            items
        },
        pick: |view, index| {
            if index == 0 {
                view.extra.flame_ui.texture_fit_path.clear();
            } else {
                let name = &view.extra.flame_ui.texture_fit_scan[index - 1];
                view.extra.flame_ui.texture_fit_path = format!("{}/{}", FLAMES_TEXTURE_DIR, name);
            }
        },
    });

    rows.push(UiRow::Apply {
        label: "Rescan Textures",
        enabled: |_world, _view| true,
        apply: |view| {
            view.extra.flame_ui.texture_fit_scan_done = false;
            view.extra.flame_ui.texture_fit_scan.clear();
        },
    });

    rows.push(UiRow::Text {
        label: "Fit Image",
        get: |view| &view.extra.flame_ui.texture_fit_path,
        set: |view, value| {
            view.extra.flame_ui.texture_fit_path = value;
        },
    });

    rows.push(UiRow::Apply {
        label: "Browse Textures",
        enabled: |_world, _view| true,
        apply: |view| {
            let browser = &mut view.extra.flame_ui;
            browser.texture_fit_browser_open = true;
            if browser.texture_fit_browser_dir.is_empty() {
                browser.texture_fit_browser_dir = FLAMES_TEXTURE_DIR.to_string();
            }
            browser.texture_fit_browser_dir = canonical_dir_or(&browser.texture_fit_browser_dir);
        },
    });

    rows
}

fn fit_group_rows(view: &FlameSectionView) -> Vec<UiRow<FlameSectionView>> {
    let mut rows: Vec<UiRow<FlameSectionView>> = Vec::new();

    rows.push(UiRow::Slider {
        label: "Fit Blend",
        min: 0.0,
        max: 1.0,
        format: "%.2f",
        get: |view| view.extra.flame_ui.texture_fit_blend,
        set: |view, value| {
            view.extra.flame_ui.texture_fit_blend = value;
        },
    });

    rows.push(UiRow::Toggle {
        label: "Silhouette",
        get: |view| view.extra.flame_ui.texture_fit_groups[0],
        set: |view, value| {
            view.extra.flame_ui.texture_fit_groups[0] = value;
        },
    });

    rows.push(UiRow::Toggle {
        label: "Color",
        get: |view| view.extra.flame_ui.texture_fit_groups[1],
        set: |view, value| {
            view.extra.flame_ui.texture_fit_groups[1] = value;
        },
    });

    if view.extra.flame_ui.texture_fit_profile {
        rows.push(UiRow::Canvas(|ui, _world, _view| {
            ui.text_disabled("Turbulence");
        }));
        rows.push(UiRow::Tooltip(
            "Ignored in profile (reproduction) mode: the turbulence estimate is far below the calibrated pattern amplitude and would crush the noise",
        ));
    } else {
        rows.push(UiRow::Toggle {
            label: "Turbulence",
            get: |view| view.extra.flame_ui.texture_fit_groups[2],
            set: |view, value| {
                view.extra.flame_ui.texture_fit_groups[2] = value;
            },
        });
    }

    rows.push(UiRow::Toggle {
        label: "Tilt",
        get: |view| view.extra.flame_ui.texture_fit_groups[3],
        set: |view, value| {
            view.extra.flame_ui.texture_fit_groups[3] = value;
        },
    });

    rows
}

fn fit_apply_rows() -> Vec<UiRow<FlameSectionView>> {
    let mut rows: Vec<UiRow<FlameSectionView>> = Vec::new();

    rows.push(UiRow::Choice {
        label: "Fit Mode",
        options: |_world, view| {
            let profile = view.extra.flame_ui.texture_fit_profile;
            vec![
                ChoiceOption {
                    label: "statistics (projection)".to_string(),
                    selected: !profile,
                },
                ChoiceOption {
                    label: "profile (reproduction)".to_string(),
                    selected: profile,
                },
            ]
        },
        pick: |view, index| {
            view.extra.flame_ui.texture_fit_profile = index == 1;
        },
    });

    rows.push(UiRow::Apply {
        label: "Apply Texture Fit",
        enabled: |_world, view| view.selected.is_some(),
        apply: |view| {
            view.extra.requests.apply_fit = true;
        },
    });

    rows
}

fn style_rows(view: &FlameSectionView) -> Vec<UiRow<FlameSectionView>> {
    let mut rows: Vec<UiRow<FlameSectionView>> = vec![UiRow::Heading("Style")];

    rows.extend(style_source_rows(view));
    rows.extend(style_group_rows());
    rows.extend(style_apply_rows(view));
    rows.extend(style_save_rows());
    rows
}

fn style_source_rows(view: &FlameSectionView) -> Vec<UiRow<FlameSectionView>> {
    if view.extra.flame_ui.style_scan.is_empty() {
        return vec![UiRow::Canvas(|ui, _world, _view| {
            ui.text_disabled(format!("no styles in {}", FLAMES_STYLE_DIR));
        })];
    }

    vec![
        UiRow::Choice {
            label: "Style File",
            options: |_world, view| {
                view.extra
                    .flame_ui
                    .style_scan
                    .iter()
                    .enumerate()
                    .map(|(index, name)| ChoiceOption {
                        label: name.clone(),
                        selected: index == view.extra.flame_ui.style_index,
                    })
                    .collect()
            },
            pick: |view, index| {
                if index < view.extra.flame_ui.style_scan.len() {
                    view.extra.flame_ui.style_index = index;
                }
            },
        },
        UiRow::Apply {
            label: "Rescan Styles",
            enabled: |_world, _view| true,
            apply: |view| {
                view.extra.flame_ui.style_scan_done = false;
                view.extra.flame_ui.style_scan.clear();
            },
        },
    ]
}

fn style_group_rows() -> Vec<UiRow<FlameSectionView>> {
    vec![
        UiRow::Toggle {
            label: "Motion",
            get: |view| view.extra.flame_ui.style_groups[0],
            set: |view, value| {
                view.extra.flame_ui.style_groups[0] = value;
            },
        },
        UiRow::Toggle {
            label: "Texture",
            get: |view| view.extra.flame_ui.style_groups[1],
            set: |view, value| {
                view.extra.flame_ui.style_groups[1] = value;
            },
        },
        UiRow::Toggle {
            label: "Optics",
            get: |view| view.extra.flame_ui.style_groups[2],
            set: |view, value| {
                view.extra.flame_ui.style_groups[2] = value;
            },
        },
    ]
}

fn style_apply_rows(view: &FlameSectionView) -> Vec<UiRow<FlameSectionView>> {
    let mut rows: Vec<UiRow<FlameSectionView>> = vec![
        UiRow::Apply {
            label: "Apply Style",
            enabled: |_world, view| {
                view.selected.is_some() && !view.extra.flame_ui.style_scan.is_empty()
            },
            apply: |view| {
                view.extra.requests.apply_style = true;
            },
        },
        UiRow::Tooltip(
            "Apply the selected style file's parameters (Style-owned only, dimensionless: lengths scale with the flame's radius, opacity via tau0). Fields the style leaves unset keep their current values",
        ),
    ];

    if view.extra.applied_style.is_some() {
        rows.push(UiRow::Canvas(|ui, _world, view: &FlameSectionView| {
            if let Some((name, version)) = &view.extra.applied_style {
                ui.text_disabled(format!("applied: {} v{}", name, version));
            }
        }));
    }
    rows
}

fn style_save_rows() -> Vec<UiRow<FlameSectionView>> {
    vec![
        UiRow::Text {
            label: "Style Name",
            get: |view| &view.extra.flame_ui.style_save_name,
            set: |view, value| {
                view.extra.flame_ui.style_save_name = value;
            },
        },
        UiRow::Apply {
            label: "Save Style",
            enabled: |_world, view| {
                !view.extra.flame_ui.style_save_name.trim().is_empty() && view.selected.is_some()
            },
            apply: |view| {
                view.extra.requests.save_style = true;
            },
        },
        UiRow::Tooltip(
            "Save the selected flame's current look as assets/flames/styles/<name>.style.ron (all Style-owned parameters, lengths over r0, opacity as tau0)",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::super::section_view::FlameExtra;
    use super::*;
    use crate::ecs::component::FlameEffect;
    use crate::ecs::resource::{FlameRenderSettings, FlameUIState};
    use crate::platform::ui::ui_rows::visible_labels;

    fn view(instances: &[&str], profile: bool, styles: &[&str]) -> FlameSectionView {
        let mut flame_ui = FlameUIState::default();
        flame_ui.texture_fit_profile = profile;
        for s in styles {
            flame_ui.style_scan.push(s.to_string());
        }
        FlameSectionView {
            instances: instances.iter().map(|s| s.to_string()).collect(),
            instance_index: 0,
            preset: None,
            selected: Some(0),
            effect: FlameEffect::default(),
            render: FlameRenderSettings::default(),
            extra: FlameExtra {
                flame_ui,
                ..FlameExtra::default()
            },
        }
    }

    #[test]
    fn fit_non_profile_has_turbulence_toggle() {
        let v = view(&["a"], false, &[]);
        let rows = fit_rows(&v);
        let labels = visible_labels(&rows);
        assert!(
            labels.contains(&"Turbulence"),
            "non-profile should have Turbulence toggle"
        );
    }

    #[test]
    fn fit_profile_has_turbulence_disabled() {
        let v = view(&["a"], true, &[]);
        let rows = fit_rows(&v);
        let labels = visible_labels(&rows);
        assert!(
            !labels.contains(&"Turbulence"),
            "profile should NOT have Turbulence toggle"
        );
        let canvas_count = rows
            .iter()
            .filter(|r| matches!(r, UiRow::Canvas(_)))
            .count();
        assert_eq!(
            canvas_count, 1,
            "profile should have 1 Canvas row (Turbulence disabled)"
        );
    }

    #[test]
    fn style_empty_scan_shows_no_styles() {
        let v = view(&["a"], false, &[]);
        let rows = style_rows(&v);
        let labels = visible_labels(&rows);
        assert!(
            !labels.contains(&"Style File"),
            "empty scan should not have Style File"
        );
        assert!(labels.contains(&"Apply Style"));
    }

    #[test]
    fn style_with_scan_has_style_file() {
        let v = view(&["a"], false, &["fire.style.ron"]);
        let rows = style_rows(&v);
        let labels = visible_labels(&rows);
        assert!(
            labels.contains(&"Style File"),
            "scan with entries should have Style File"
        );
    }

    #[test]
    fn fit_texture_pick_builds_path() {
        let mut v = view(&["a"], false, &[]);
        v.extra
            .flame_ui
            .texture_fit_scan
            .push("test.png".to_string());
        let rows = fit_rows(&v);
        for row in &rows {
            if let UiRow::Choice { label, pick, .. } = row {
                if *label == "Fit Texture" {
                    pick(&mut v, 1);
                    assert_eq!(
                        v.extra.flame_ui.texture_fit_path,
                        format!("{}/test.png", FLAMES_TEXTURE_DIR)
                    );
                    return;
                }
            }
        }
        panic!("Fit Texture row not found");
    }

    #[test]
    fn fit_texture_pick_zero_clears_path() {
        let mut v = view(&["a"], false, &[]);
        v.extra.flame_ui.texture_fit_path = "some/path".to_string();
        let rows = fit_rows(&v);
        for row in &rows {
            if let UiRow::Choice { label, pick, .. } = row {
                if *label == "Fit Texture" {
                    pick(&mut v, 0);
                    assert!(v.extra.flame_ui.texture_fit_path.is_empty());
                    return;
                }
            }
        }
        panic!("Fit Texture row not found");
    }

    #[test]
    fn fit_rows_no_duplicates() {
        let v = view(&["a"], false, &[]);
        let rows = fit_rows(&v);
        let labels = visible_labels(&rows);
        let mut sorted = labels.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), labels.len(), "fit rows have duplicate labels");
    }

    #[test]
    fn style_rows_no_duplicates() {
        let v = view(&["a"], false, &["fire.style.ron"]);
        let rows = style_rows(&v);
        let labels = visible_labels(&rows);
        let mut sorted = labels.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            labels.len(),
            "style rows have duplicate labels"
        );
    }
}
