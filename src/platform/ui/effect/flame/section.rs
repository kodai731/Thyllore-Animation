use crate::ecs::component::{FlameEffect, FLAME_DOMAIN};
use crate::ecs::systems::resolve_selected_flame;
use crate::ecs::World;

use super::body_rows::body_rows;
use super::head_rows::{fit_rows, head_rows, style_rows};
use super::section_view::{
    commit_flame_view, draw_fit_path_status, read_flame_section_view, FlameSectionView,
};
use crate::platform::ui::form_window::{FormSection, RowsSection};
use crate::platform::ui::ui_rows::{FieldRows, UiRow};

fn flame_rows(view: &FlameSectionView) -> Vec<UiRow<FlameSectionView>> {
    let mut rows = head_rows(view);
    rows.extend(fit_rows(view));
    rows.push(UiRow::Canvas(|ui, _, view: &FlameSectionView| {
        draw_fit_path_status(ui, &view.flame_ui)
    }));
    rows.extend(style_rows(view));
    if view.selected.is_some() {
        rows.push(UiRow::Field(Box::new(FieldRows {
            scope: "body",
            field: |view: &mut FlameSectionView| &mut view.body,
            rows: body_rows(&view.body),
        })));
    }
    rows
}

fn flame_section(world: &World) -> Option<Box<dyn FormSection>> {
    let flames = world.entities_with::<FlameEffect>();
    let selected = resolve_selected_flame(world);
    let view = read_flame_section_view(world, &flames, selected);

    let group = UiRow::Group {
        title: "Flame".into(),
        initially_open: false,
        rows: flame_rows(&view),
    };

    Some(Box::new(RowsSection {
        rows: vec![group],
        view,
        commit: commit_flame_view,
        keys: Some(&FLAME_DOMAIN),
    }))
}

crate::effect_section_hook!(crate::platform::ui::effect::sections::EffectSectionHook {
    key: "flame",
    order: 4,
    section: flame_section,
});

#[cfg(test)]
mod tests {
    use super::super::body_rows::FlameBodyView;
    use super::super::section_view::FlameRequests;
    use super::*;
    use crate::ecs::resource::FlameUIState;
    use crate::platform::ui::ui_rows::find_duplicate_labels;
    use thyllore_effect_core::FlameRenderSettings;

    #[test]
    fn selected_flame_rows_have_no_duplicate_labels() {
        let view = FlameSectionView {
            instances: vec!["flame".to_string()],
            instance_index: 0,
            preset: None,
            flame_ui: FlameUIState::default(),
            applied_style: None,
            selected: Some(0),
            body: FlameBodyView {
                effect: FlameEffect::default(),
                trail_enabled: false,
                trail_fade: 0.0,
                render: FlameRenderSettings::default(),
                jitter: 0.0,
                jitter_freq: 0.0,
                viewport_size: [1.0, 1.0],
                gpu_timings: Vec::new(),
            },
            requests: FlameRequests::default(),
        };
        assert!(find_duplicate_labels(&flame_rows(&view)).is_empty());
    }
}
