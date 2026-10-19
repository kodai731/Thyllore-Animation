use std::rc::Rc;
use std::sync::LazyLock;

use thyllore_scene_core::{FieldPath, ScalarParam, SceneFields, UiParam};

use crate::ecs::resource::{
    LightningDebugCapture, LightningDebugView, LightningRenderSettings, LightningShadingMode,
};
use crate::ecs::systems::lightning::LightningUiCommand;
use crate::ecs::systems::phases::event_dispatch::camera::CameraEvent;
use crate::ecs::systems::phases::event_dispatch::scalar_curve::ScalarCurveEvent;
use crate::ecs::systems::LIGHTNING_SPAWN_HOOK;

use super::effect_rows::{instance_options, preset_options};
use super::ui_rows::{ChoiceOption, ListRows, UiRow};

#[derive(Clone)]
pub struct LightningSectionView {
    pub instances: Vec<String>,
    pub instance_index: usize,
    pub preset: Option<String>,
    pub target_name: Option<String>,
    pub waypoints: Vec<String>,
    pub render: LightningRenderSettings,
}

struct LightningRenderPath;

impl FieldPath<LightningSectionView> for LightningRenderPath {
    type Field = LightningRenderSettings;

    fn get(root: &LightningSectionView) -> &LightningRenderSettings {
        &root.render
    }

    fn get_mut(root: &mut LightningSectionView) -> &mut LightningRenderSettings {
        &mut root.render
    }
}

static RENDER_SCALARS: LazyLock<Vec<ScalarParam<LightningSectionView>>> = LazyLock::new(|| {
    let mut out = Vec::new();
    LightningRenderSettings::collect_scalars::<LightningSectionView, LightningRenderPath>(
        "", &mut out,
    );
    out
});

static RENDER_UI_PARAMS: LazyLock<Vec<UiParam>> = LazyLock::new(|| {
    let mut out = Vec::new();
    LightningRenderSettings::collect_ui("", "", &mut out);
    out
});

fn waypoint_items(_world: &crate::ecs::World, view: &LightningSectionView) -> Vec<(String, usize)> {
    view.waypoints
        .iter()
        .enumerate()
        .map(|(i, name)| (name.clone(), i))
        .collect()
}

fn waypoint_rows(_index: &usize) -> Vec<UiRow<usize>> {
    vec![UiRow::Action {
        label: "Remove Waypoint",
        enabled: |_, _| true,
        on_click: |index| Box::new(LightningUiCommand::RemoveWaypoint(*index)),
    }]
}

fn waypoint_on_change(
    _view: &LightningSectionView,
    _index: usize,
    item: usize,
) -> Box<dyn crate::ecs::events::UiCommand> {
    Box::new(LightningUiCommand::RemoveWaypoint(item))
}

pub fn lightning_head_rows(view: &LightningSectionView) -> Vec<UiRow<LightningSectionView>> {
    let mut rows = vec![UiRow::Action {
        label: "Add Lightning",
        enabled: |_, _| true,
        on_click: |_| Box::new(ScalarCurveEvent::AddEffect(LIGHTNING_SPAWN_HOOK.key)),
    }];

    if view.instances.len() > 1 {
        rows.push(UiRow::Choice {
            label: "Instance",
            options: |_, view: &LightningSectionView| {
                instance_options(&view.instances, view.instance_index)
            },
            pick: |view, index| view.instance_index = index,
        });
    }

    rows.push(UiRow::Choice {
        label: "Preset",
        options: |_, view: &LightningSectionView| {
            preset_options(
                thyllore_effect_core::LIGHTNING_PRESET_NAMES,
                view.preset.as_deref(),
            )
        },
        pick: |view, index| {
            view.preset = Some(thyllore_effect_core::LIGHTNING_PRESET_NAMES[index].to_string());
        },
    });

    match &view.target_name {
        Some(_) => {
            rows.push(UiRow::Canvas(|ui, _world, view: &LightningSectionView| {
                if let Some(name) = &view.target_name {
                    ui.text(format!("Target: {name}"));
                }
            }));
            rows.push(UiRow::Action {
                label: "Clear Target",
                enabled: |_, _| true,
                on_click: |_| Box::new(LightningUiCommand::ClearTarget),
            });
        }
        None => {
            rows.push(UiRow::Action {
                label: "Add Target",
                enabled: |_, _| true,
                on_click: |_| Box::new(LightningUiCommand::AddTarget),
            });
            rows.push(UiRow::Tooltip(
                "Spawn a locator at the end point; move it to aim the bolt",
            ));
        }
    }

    rows.push(UiRow::List(Box::new(ListRows::<
        LightningSectionView,
        usize,
    > {
        title: "Waypoints",
        items: waypoint_items,
        rows: waypoint_rows,
        on_change: waypoint_on_change,
        initially_open: |_| true,
    })));

    rows.push(UiRow::Action {
        label: "Add Waypoint",
        enabled: |_, _| true,
        on_click: |_| Box::new(LightningUiCommand::AddWaypoint),
    });
    rows.push(UiRow::Tooltip(
        "Spawn a locator halfway to the end point; the bolt passes through waypoints in order",
    ));

    rows
}

pub fn lightning_tail_rows() -> Vec<UiRow<LightningSectionView>> {
    vec![
        UiRow::Action {
            label: "Curves",
            enabled: |_, _| true,
            on_click: |_| Box::new(ScalarCurveEvent::OpenScalarCurveEditor),
        },
        UiRow::Group {
            title: "Lightning Debug",
            initially_open: false,
            rows: lightning_debug_rows(),
        },
    ]
}

fn lightning_debug_rows() -> Vec<UiRow<LightningSectionView>> {
    let mut rows = vec![
        UiRow::Choice {
            label: "Shading Mode",
            options: |_, view: &LightningSectionView| {
                LightningShadingMode::ALL
                    .iter()
                    .map(|&mode| ChoiceOption {
                        label: mode.label().to_string(),
                        selected: mode == view.render.shading_mode,
                    })
                    .collect()
            },
            pick: |view, index| view.render.shading_mode = LightningShadingMode::ALL[index],
        },
        UiRow::Choice {
            label: "Debug View",
            options: |_, view: &LightningSectionView| {
                LightningDebugView::ALL
                    .iter()
                    .map(|&debug_view| ChoiceOption {
                        label: debug_view.label().to_string(),
                        selected: debug_view == view.render.debug_view,
                    })
                    .collect()
            },
            pick: |view, index| view.render.debug_view = LightningDebugView::ALL[index],
        },
    ];

    rows.extend(RENDER_UI_PARAMS.iter().map(|param| UiRow::Scalar {
        param,
        scalars: &RENDER_SCALARS,
    }));

    rows.push(UiRow::Action {
        label: "Dump Debug",
        enabled: |_, _| true,
        on_click: |_| Box::new(CameraEvent::CaptureNow(Rc::new(LightningDebugCapture))),
    });
    rows.push(UiRow::Tooltip(
        "Write lightning parameters, UBO, render settings, camera and a screenshot to log/lightning/",
    ));
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view_with_instances(count: usize) -> LightningSectionView {
        LightningSectionView {
            instances: (1..=count).map(|i| format!("Lightning {i}")).collect(),
            instance_index: 0,
            preset: None,
            target_name: None,
            waypoints: Vec::new(),
            render: LightningRenderSettings::default(),
        }
    }

    fn view_with_target() -> LightningSectionView {
        let mut v = view_with_instances(1);
        v.target_name = Some("Target Locator".to_string());
        v
    }

    #[test]
    fn head_rows_without_target_show_add_target() {
        let rows = lightning_head_rows(&view_with_instances(1));
        let labels: Vec<&str> = crate::platform::ui::ui_rows::visible_labels(&rows);
        assert!(
            labels.contains(&"Add Target"),
            "expected Add Target in {:?}",
            labels
        );
        assert!(
            !labels.contains(&"Clear Target"),
            "should not have Clear Target in {:?}",
            labels
        );
    }

    #[test]
    fn head_rows_with_target_show_clear_target() {
        let rows = lightning_head_rows(&view_with_target());
        let labels: Vec<&str> = crate::platform::ui::ui_rows::visible_labels(&rows);
        assert!(
            labels.contains(&"Clear Target"),
            "expected Clear Target in {:?}",
            labels
        );
        assert!(
            !labels.contains(&"Add Target"),
            "should not have Add Target in {:?}",
            labels
        );
    }

    #[test]
    fn head_rows_with_two_instances_show_instance_choice() {
        let rows = lightning_head_rows(&view_with_instances(2));
        let labels: Vec<&str> = crate::platform::ui::ui_rows::visible_labels(&rows);
        assert!(
            labels.contains(&"Instance"),
            "expected Instance in {:?}",
            labels
        );
    }

    #[test]
    fn head_rows_with_one_instance_hide_instance_choice() {
        let rows = lightning_head_rows(&view_with_instances(1));
        let labels: Vec<&str> = crate::platform::ui::ui_rows::visible_labels(&rows);
        assert!(
            !labels.contains(&"Instance"),
            "should not have Instance in {:?}",
            labels
        );
    }

    #[test]
    fn head_rows_no_duplicates() {
        let rows = lightning_head_rows(&view_with_instances(2));
        let dupes = crate::platform::ui::ui_rows::find_duplicate_labels(&rows);
        assert!(dupes.is_empty(), "duplicate labels: {:?}", dupes);
    }

    #[test]
    fn tail_rows_show_curves_before_debug_group() {
        let rows = lightning_tail_rows();
        let labels: Vec<&str> = crate::platform::ui::ui_rows::visible_labels(&rows);
        assert_eq!(labels, ["Curves"]);
        assert!(matches!(
            rows.last(),
            Some(UiRow::Group {
                title: "Lightning Debug",
                ..
            })
        ));
    }

    #[test]
    fn debug_rows_keep_order_without_duplicates() {
        let rows = lightning_debug_rows();
        let labels: Vec<&str> = crate::platform::ui::ui_rows::visible_labels(&rows);
        assert_eq!(
            labels,
            [
                "Shading Mode",
                "Debug View",
                "reference_step_count",
                "Dump Debug",
            ]
        );
        let dupes = crate::platform::ui::ui_rows::find_duplicate_labels(&rows);
        assert!(dupes.is_empty(), "duplicate labels: {:?}", dupes);
    }
}
