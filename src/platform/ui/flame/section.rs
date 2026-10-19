use crate::ecs::component::{FlameEffect, FLAME_DOMAIN};
use crate::ecs::systems::resolve_selected_flame;
use crate::ecs::World;

use super::super::form_window::{FormSection, RowsSection};
use super::super::ui_rows::{FieldRows, UiRow};
use super::body_rows::body_rows;
use super::head_rows::{fit_rows, head_rows, style_rows};
use super::section_view::{
    commit_flame_view, draw_fit_path_status, read_flame_section_view, FlameSectionView,
};

fn flame_rows(view: &FlameSectionView) -> Vec<UiRow<FlameSectionView>> {
    let mut rows = head_rows(view);
    rows.extend(fit_rows(view));
    rows.push(UiRow::Canvas(|ui, _, view: &FlameSectionView| {
        draw_fit_path_status(ui, &view.flame_ui)
    }));
    rows.extend(style_rows(view));
    if view.selected.is_some() {
        rows.push(UiRow::Field(Box::new(FieldRows {
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

crate::effect_section_hook!(crate::platform::ui::effect_sections::EffectSectionHook {
    key: "flame",
    order: 4,
    section: flame_section,
});
