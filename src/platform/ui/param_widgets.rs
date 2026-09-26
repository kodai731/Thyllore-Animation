use imgui::ColorEditFlags;
use thyllore_effect_core::{
    absorption_to_transmitted_color, transmitted_color_to_absorption, ABSORPTION_REFERENCE_DISTANCE,
};
use thyllore_scene_core::{find_scalar_param, find_ui_param, ScalarParam, UiKind, UiParam};

/// Scalar keys touched by one widget, `(alias name, value)`; a colour yields its r, g, b aliases.
pub type EditedScalars<'a> = &'a [(&'static str, f32)];

/// Names are validated against both tables by unit tests; an unresolved name is skipped.
pub fn draw_params<C>(
    ui: &imgui::Ui,
    names: &[&str],
    ui_params: &[UiParam],
    scalars: &[ScalarParam<C>],
    component: &mut C,
    mut after_item: impl FnMut(&imgui::Ui, EditedScalars),
) {
    for name in names {
        let Some(meta) = find_ui_param(ui_params, name) else {
            continue;
        };
        match meta.kind {
            UiKind::Scalar => draw_scalar(ui, meta, scalars, component, &mut after_item),
            UiKind::Color => draw_color(
                ui,
                meta,
                scalars,
                component,
                ColorMapping::Identity,
                &mut after_item,
            ),
            UiKind::Absorption => draw_color(
                ui,
                meta,
                scalars,
                component,
                ColorMapping::Transmitted,
                &mut after_item,
            ),
            UiKind::Offset => draw_offset(ui, meta, scalars, component, &mut after_item),
        }
    }
}

fn draw_scalar<C>(
    ui: &imgui::Ui,
    meta: &UiParam,
    scalars: &[ScalarParam<C>],
    component: &mut C,
    after_item: &mut impl FnMut(&imgui::Ui, EditedScalars),
) {
    let Some(scalar) = find_scalar_param(scalars, meta.name) else {
        return;
    };

    let mut value = (scalar.get)(component);
    if ui
        .slider_config(meta.display_label(), meta.min, meta.max)
        .display_format(meta.format)
        .build(&mut value)
    {
        (scalar.set)(component, value);
    }
    show_tooltip(ui, meta.tooltip);
    after_item(ui, &[(scalar.name, value)]);
}

#[derive(Clone, Copy)]
enum ColorMapping {
    Identity,
    Transmitted,
}

impl ColorMapping {
    fn to_picker(self, stored: [f32; 3]) -> [f32; 3] {
        match self {
            ColorMapping::Identity => stored,
            ColorMapping::Transmitted => {
                absorption_to_transmitted_color(stored, ABSORPTION_REFERENCE_DISTANCE)
            }
        }
    }

    fn from_picker(self, picked: [f32; 3]) -> [f32; 3] {
        match self {
            ColorMapping::Identity => picked,
            ColorMapping::Transmitted => {
                transmitted_color_to_absorption(picked, ABSORPTION_REFERENCE_DISTANCE)
            }
        }
    }

    fn tooltip(self, base: &str) -> String {
        match self {
            ColorMapping::Identity => base.to_string(),
            ColorMapping::Transmitted => {
                format!("{base} (transmitted colour over {ABSORPTION_REFERENCE_DISTANCE} m)")
            }
        }
    }
}

fn draw_color<C>(
    ui: &imgui::Ui,
    meta: &UiParam,
    scalars: &[ScalarParam<C>],
    component: &mut C,
    mapping: ColorMapping,
    after_item: &mut impl FnMut(&imgui::Ui, EditedScalars),
) {
    let component_names = meta.color_component_names();
    let Some(channels) = resolve_channels(scalars, &component_names) else {
        return;
    };

    let stored = channels.map(|channel| (channel.get)(component));
    let mut picked = mapping.to_picker(stored);
    let changed = ui
        .color_picker3_config(meta.display_label(), &mut picked)
        .flags(ColorEditFlags::FLOAT | ColorEditFlags::NO_ALPHA | ColorEditFlags::NO_INPUTS)
        .build();
    show_tooltip(ui, &mapping.tooltip(meta.tooltip));

    let written = if changed {
        let clamped = mapping
            .from_picker(picked)
            .map(|value| value.clamp(meta.min, meta.max));
        for (channel, value) in channels.iter().zip(clamped) {
            (channel.set)(component, value);
        }
        clamped
    } else {
        stored
    };

    let edited: [(&'static str, f32); 3] = [
        (channels[0].name, written[0]),
        (channels[1].name, written[1]),
        (channels[2].name, written[2]),
    ];
    after_item(ui, &edited);
}

fn resolve_channels<'a, C>(
    scalars: &'a [ScalarParam<C>],
    names: &[String; 3],
) -> Option<[&'a ScalarParam<C>; 3]> {
    Some([
        find_scalar_param(scalars, &names[0])?,
        find_scalar_param(scalars, &names[1])?,
        find_scalar_param(scalars, &names[2])?,
    ])
}

fn draw_offset<C>(
    ui: &imgui::Ui,
    meta: &UiParam,
    scalars: &[ScalarParam<C>],
    component: &mut C,
    after_item: &mut impl FnMut(&imgui::Ui, EditedScalars),
) {
    let component_names = meta.offset_component_names();
    let Some(channels) = resolve_channels(scalars, &component_names) else {
        return;
    };

    let stored = channels.map(|channel| (channel.get)(component));
    let mut dragged = stored;
    let changed = imgui::Drag::new(meta.display_label())
        .range(meta.min, meta.max)
        .display_format(meta.format)
        .build_array(ui, &mut dragged);
    show_tooltip(ui, meta.tooltip);

    let written = if changed {
        let clamped = dragged.map(|value| value.clamp(meta.min, meta.max));
        for (channel, value) in channels.iter().zip(clamped) {
            (channel.set)(component, value);
        }
        clamped
    } else {
        stored
    };

    let edited: [(&'static str, f32); 3] = [
        (channels[0].name, written[0]),
        (channels[1].name, written[1]),
        (channels[2].name, written[2]),
    ];
    after_item(ui, &edited);
}

fn show_tooltip(ui: &imgui::Ui, tooltip: &str) {
    if !tooltip.is_empty() && ui.is_item_hovered() {
        ui.tooltip_text(tooltip);
    }
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
    ui_params: &[UiParam],
    scalars: &[ScalarParam<C>],
    component: &mut C,
    hidden: &[&str],
    mut after_item: impl FnMut(&imgui::Ui, EditedScalars),
) -> bool {
    let primary_names = determine_primary_order(ui_params, hidden);
    draw_params(
        ui,
        &primary_names,
        ui_params,
        scalars,
        component,
        &mut after_item,
    );

    let remaining_groups = group_remaining_params(ui_params, hidden);
    if remaining_groups.is_empty()
        || !ui.collapsing_header("Advanced", imgui::TreeNodeFlags::empty())
    {
        return false;
    }

    for (heading, names) in &remaining_groups {
        if let Some(group_heading) = heading {
            ui.text(group_heading);
        }
        draw_params(ui, names, ui_params, scalars, component, &mut after_item);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ui_param(name: &'static str, group: &'static str, primary: bool) -> UiParam {
        UiParam {
            name,
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
