use thyllore_scene_core::{find_ui_param, ScalarParam, UiParam};

use crate::ecs::World;
use crate::platform::ui::theme::{section_header, SectionDefault};

use super::ui_rows::{draw_rows, UiRow};

pub use super::ui_rows::EditedScalars;

/// Names are validated against both tables by unit tests; an unresolved name is skipped.
pub fn draw_params<C>(
    ui: &imgui::Ui,
    world: &World,
    names: &[&str],
    ui_params: &'static [UiParam],
    scalars: &'static [ScalarParam<C>],
    component: &mut C,
    after_item: impl FnMut(&imgui::Ui, EditedScalars),
) {
    let rows: Vec<UiRow<C>> = names
        .iter()
        .filter_map(|name| find_ui_param(ui_params, name))
        .map(|param| UiRow::Scalar { param, scalars })
        .collect();
    draw_rows(ui, world, &rows, component, after_item);
}

/// `(group heading, names)`; `None` holds the ungrouped names and always comes last.
pub type ParamGroup = (Option<&'static str>, Vec<&'static str>);

pub fn determine_primary_order(ui_params: &[UiParam], hidden: &[&str]) -> Vec<&'static str> {
    ui_params
        .iter()
        .filter(|param| param.primary && !hidden.contains(&param.name))
        .map(|param| param.name)
        .collect()
}

pub fn group_remaining_params(ui_params: &[UiParam], hidden: &[&str]) -> Vec<ParamGroup> {
    let remaining: Vec<&UiParam> = ui_params
        .iter()
        .filter(|param| !param.primary && !hidden.contains(&param.name))
        .collect();

    let mut groups: Vec<ParamGroup> = Vec::new();
    for param in remaining.iter().filter(|param| !param.group.is_empty()) {
        match groups
            .iter_mut()
            .find(|(heading, _)| *heading == Some(param.group))
        {
            Some((_, names)) => names.push(param.name),
            None => groups.push((Some(param.group), vec![param.name])),
        }
    }

    let ungrouped: Vec<&'static str> = remaining
        .iter()
        .filter(|param| param.group.is_empty())
        .map(|param| param.name)
        .collect();
    if !ungrouped.is_empty() {
        groups.push((None, ungrouped));
    }

    groups
}

pub fn draw_tiered_params<C>(
    ui: &imgui::Ui,
    world: &World,
    ui_params: &'static [UiParam],
    scalars: &'static [ScalarParam<C>],
    component: &mut C,
    hidden: &[&str],
    mut after_item: impl FnMut(&imgui::Ui, EditedScalars),
) -> bool {
    let primary_names = determine_primary_order(ui_params, hidden);
    draw_params(
        ui,
        world,
        &primary_names,
        ui_params,
        scalars,
        component,
        &mut after_item,
    );

    let remaining_groups = group_remaining_params(ui_params, hidden);
    if remaining_groups.is_empty() || !section_header(ui, world, "Advanced", SectionDefault::Closed)
    {
        return false;
    }

    for (heading, names) in &remaining_groups {
        if let Some(group_heading) = heading {
            ui.text(group_heading);
        }
        draw_params(
            ui,
            world,
            names,
            ui_params,
            scalars,
            component,
            &mut after_item,
        );
    }
    true
}

pub(super) fn draw_preset_combo(
    ui: &imgui::Ui,
    label: &str,
    names: &[&str],
    applied: Option<&str>,
) -> Option<String> {
    let preview = applied.unwrap_or("(none)");
    let combo = ui.begin_combo(label, preview)?;
    let mut chosen = None;
    for &name in names {
        if ui
            .selectable_config(name)
            .selected(Some(name) == applied)
            .build()
        {
            chosen = Some(name.to_string());
        }
    }
    combo.end();
    chosen
}

#[cfg(test)]
mod tests {
    use super::*;
    use thyllore_scene_core::UiKind;

    fn make_ui_param(name: &'static str, group: &'static str, primary: bool) -> UiParam {
        UiParam {
            name,
            path: name,
            group,
            label: None,
            kind: UiKind::Scalar,
            min: 0.0,
            max: 1.0,
            format: "%.3f",
            tooltip: "",
            persisted: true,
            primary,
        }
    }

    fn mixed_params() -> [UiParam; 6] {
        [
            make_ui_param("a", "g1", true),
            make_ui_param("b", "g1", false),
            make_ui_param("c", "", false),
            make_ui_param("d", "g2", false),
            make_ui_param("e", "g1", false),
            make_ui_param("f", "g2", true),
        ]
    }

    #[test]
    fn primary_order_follows_declaration_order() {
        assert_eq!(determine_primary_order(&mixed_params(), &[]), ["a", "f"]);
    }

    #[test]
    fn primary_order_skips_hidden() {
        assert_eq!(determine_primary_order(&mixed_params(), &["a"]), ["f"]);
    }

    #[test]
    fn remaining_groups_keep_first_appearance_order_and_put_ungrouped_last() {
        assert_eq!(
            group_remaining_params(&mixed_params(), &[]),
            [
                (Some("g1"), vec!["b", "e"]),
                (Some("g2"), vec!["d"]),
                (None, vec!["c"]),
            ]
        );
    }

    #[test]
    fn remaining_groups_drop_a_group_whose_names_are_all_hidden() {
        assert_eq!(
            group_remaining_params(&mixed_params(), &["d", "c"]),
            [(Some("g1"), vec!["b", "e"])]
        );
    }

    #[test]
    fn remaining_groups_are_empty_when_every_param_is_primary() {
        let params = [make_ui_param("a", "g1", true), make_ui_param("b", "", true)];
        assert!(group_remaining_params(&params, &[]).is_empty());
    }

    #[test]
    fn primary_and_remaining_partition_every_param() {
        let params = mixed_params();
        let mut drawn: Vec<&str> = determine_primary_order(&params, &[]);
        for (_, names) in group_remaining_params(&params, &[]) {
            drawn.extend(names);
        }
        drawn.sort_unstable();
        assert_eq!(drawn, ["a", "b", "c", "d", "e", "f"]);
    }
}
