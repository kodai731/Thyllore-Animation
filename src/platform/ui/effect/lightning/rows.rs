use std::rc::Rc;
use std::sync::LazyLock;

use thyllore_scene_core::{FieldPath, ScalarParam, SceneFields, UiParam};

use crate::ecs::component::LightningEffect;
use crate::ecs::resource::{
    LightningDebugCapture, LightningDebugView, LightningRenderSettings, LightningShadingMode,
};
use crate::ecs::systems::lightning::LightningUiCommand;
use crate::ecs::systems::phases::event_dispatch::camera::CameraEvent;

use crate::platform::ui::effect::sections::EffectSectionView;
use crate::platform::ui::ui_rows::{ChoiceOption, ListRows, UiRow};

#[derive(Clone, Default)]
pub(super) struct LightningExtra {
    pub target_name: Option<String>,
    pub waypoints: Vec<String>,
}

pub(super) type LightningSectionView =
    EffectSectionView<LightningEffect, LightningRenderSettings, LightningExtra>;

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

fn waypoint_items(
    _world: &crate::ecs::world::World,
    view: &LightningSectionView,
) -> Vec<(String, usize)> {
    view.extra
        .waypoints
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

pub(super) fn lightning_extra_head_rows(
    view: &LightningSectionView,
) -> Vec<UiRow<LightningSectionView>> {
    let mut rows = Vec::new();

    match &view.extra.target_name {
        Some(_) => {
            rows.push(UiRow::Canvas(|ui, _world, view: &LightningSectionView| {
                if let Some(name) = &view.extra.target_name {
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

pub(super) fn lightning_debug_rows() -> Vec<UiRow<LightningSectionView>> {
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
    use crate::platform::ui::ui_rows::{find_duplicate_labels, visible_labels};

    fn view_with_target(target_name: Option<&str>) -> LightningSectionView {
        LightningSectionView {
            instances: vec!["Lightning 1".to_string()],
            instance_index: 0,
            preset: None,
            selected: None,
            effect: LightningEffect::default(),
            render: LightningRenderSettings::default(),
            extra: LightningExtra {
                target_name: target_name.map(str::to_string),
                waypoints: Vec::new(),
            },
        }
    }

    #[test]
    fn extra_head_rows_without_target_show_add_target() {
        let rows = lightning_extra_head_rows(&view_with_target(None));
        assert_eq!(
            visible_labels(&rows),
            ["Add Target", "Waypoints", "Add Waypoint"]
        );
        assert!(find_duplicate_labels(&rows).is_empty());
    }

    #[test]
    fn extra_head_rows_with_target_show_clear_target() {
        let rows = lightning_extra_head_rows(&view_with_target(Some("Target Locator")));
        assert_eq!(
            visible_labels(&rows),
            ["Clear Target", "Waypoints", "Add Waypoint"]
        );
        assert!(find_duplicate_labels(&rows).is_empty());
    }

    #[test]
    fn debug_rows_keep_order_without_duplicates() {
        let rows = lightning_debug_rows();
        assert_eq!(
            visible_labels(&rows),
            [
                "Shading Mode",
                "Debug View",
                "reference_step_count",
                "Dump Debug",
            ]
        );
        assert!(find_duplicate_labels(&rows).is_empty());
    }
}
