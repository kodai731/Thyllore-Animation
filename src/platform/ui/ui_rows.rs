use imgui::{ColorDataType, ColorPickerFlags, ColorPickerOptions};
use thyllore_effect_core::{
    absorption_to_transmitted_color, transmitted_color_to_absorption, ABSORPTION_REFERENCE_DISTANCE,
};
use thyllore_scene_core::{find_scalar_param, ScalarParam, UiKind, UiParam};

use crate::ecs::events::UiCommand;
use crate::ecs::World;
use crate::platform::ui::numeric_format::numeric_format;
use crate::platform::ui::theme::{property_label, section_header, vector3_field, SectionDefault};

/// Scalar keys touched by one widget, `(alias name, value)`; a colour yields its r, g, b aliases.
pub type EditedScalars<'a> = &'a [(&'static str, f32)];

pub enum UiRow<C: 'static> {
    Scalar {
        param: &'static UiParam,
        scalars: &'static [ScalarParam<C>],
    },
    Toggle {
        label: &'static str,
        get: fn(&C) -> bool,
        set: fn(&mut C, bool),
    },
    Choice {
        label: &'static str,
        options: fn(&World, &C) -> Vec<ChoiceOption>,
        pick: fn(&mut C, usize),
    },
    Text {
        label: &'static str,
        get: fn(&C) -> &str,
        set: fn(&mut C, String),
    },
    Action {
        label: &'static str,
        enabled: fn(&World, &C) -> bool,
        on_click: fn(&C) -> Box<dyn UiCommand>,
    },
    List(Box<dyn DrawList<C>>),
    Canvas(fn(&imgui::Ui, &World, &C)),
}

pub struct ChoiceOption {
    pub label: String,
    pub selected: bool,
}

pub struct ListRows<C, I: 'static> {
    pub title: &'static str,
    pub items: fn(&World, &C) -> Vec<(String, I)>,
    pub rows: fn() -> Vec<UiRow<I>>,
    pub on_change: fn(&C, usize, I) -> Box<dyn UiCommand>,
}

pub trait DrawList<C> {
    fn title(&self) -> &'static str;
    fn draw(&self, ui: &imgui::Ui, world: &World, owner: &C);
}

impl<C: 'static, I: 'static> DrawList<C> for ListRows<C, I> {
    fn title(&self) -> &'static str {
        self.title
    }

    fn draw(&self, ui: &imgui::Ui, world: &World, owner: &C) {
        ui.text(self.title);
        let rows = (self.rows)();
        for (index, (name, item)) in (self.items)(world, owner).into_iter().enumerate() {
            self.draw_item(ui, world, owner, &rows, index, &name, item);
        }
    }
}

impl<C: 'static, I: 'static> ListRows<C, I> {
    fn draw_item(
        &self,
        ui: &imgui::Ui,
        world: &World,
        owner: &C,
        rows: &[UiRow<I>],
        index: usize,
        name: &str,
        mut item: I,
    ) {
        let _item_id = ui.push_id(index);
        if !section_header(ui, world, name, SectionDefault::Closed) {
            return;
        }
        if draw_rows(ui, world, rows, &mut item, |_, _| {}) {
            world.send_boxed_command((self.on_change)(owner, index, item));
        }
    }
}

/// Returns whether any row wrote to `value`.
pub fn draw_rows<C>(
    ui: &imgui::Ui,
    world: &World,
    rows: &[UiRow<C>],
    value: &mut C,
    mut after_item: impl FnMut(&imgui::Ui, EditedScalars),
) -> bool {
    let mut changed = false;
    for row in rows {
        changed |= draw_row(ui, world, row, value, &mut after_item);
    }
    changed
}

fn draw_row<C>(
    ui: &imgui::Ui,
    world: &World,
    row: &UiRow<C>,
    value: &mut C,
    after_item: &mut impl FnMut(&imgui::Ui, EditedScalars),
) -> bool {
    match row {
        UiRow::Scalar { param, scalars } => draw_scalar_row(ui, param, scalars, value, after_item),
        UiRow::Toggle { label, get, set } => draw_toggle(ui, label, *get, *set, value),
        UiRow::Choice {
            label,
            options,
            pick,
        } => draw_choice(ui, label, &options(world, value), *pick, value),
        UiRow::Text { label, get, set } => draw_text(ui, label, *get, *set, value),
        UiRow::Action {
            label,
            enabled,
            on_click,
        } => {
            let _disabled = ui.begin_disabled_with_cond(!enabled(world, value));
            if ui.button(label) {
                world.send_boxed_command(on_click(value));
            }
            false
        }
        UiRow::List(list) => {
            list.draw(ui, world, value);
            false
        }
        UiRow::Canvas(draw) => {
            draw(ui, world, value);
            false
        }
    }
}

fn draw_toggle<C>(
    ui: &imgui::Ui,
    label: &str,
    get: fn(&C) -> bool,
    set: fn(&mut C, bool),
    value: &mut C,
) -> bool {
    let mut checked = get(value);
    property_label(ui, label);
    if !ui.checkbox(format!("##{label}"), &mut checked) {
        return false;
    }
    set(value, checked);
    true
}

fn draw_choice<C>(
    ui: &imgui::Ui,
    label: &str,
    options: &[ChoiceOption],
    pick: fn(&mut C, usize),
    value: &mut C,
) -> bool {
    let preview = options
        .iter()
        .find(|option| option.selected)
        .map_or("", |option| option.label.as_str());
    property_label(ui, label);
    let Some(combo) = ui.begin_combo(format!("##{label}"), preview) else {
        return false;
    };

    let mut picked = None;
    for (index, option) in options.iter().enumerate() {
        if ui
            .selectable_config(&option.label)
            .selected(option.selected)
            .build()
        {
            picked = Some(index);
        }
    }
    combo.end();

    let Some(index) = picked.filter(|&index| !options[index].selected) else {
        return false;
    };
    pick(value, index);
    true
}

fn draw_text<C>(
    ui: &imgui::Ui,
    label: &str,
    get: fn(&C) -> &str,
    set: fn(&mut C, String),
    value: &mut C,
) -> bool {
    let mut text = get(value).to_owned();
    property_label(ui, label);
    if !ui.input_text(format!("##{label}"), &mut text).build() {
        return false;
    }
    set(value, text);
    true
}

fn draw_scalar_row<C>(
    ui: &imgui::Ui,
    meta: &UiParam,
    scalars: &[ScalarParam<C>],
    component: &mut C,
    after_item: &mut impl FnMut(&imgui::Ui, EditedScalars),
) -> bool {
    match meta.kind {
        UiKind::Scalar => draw_scalar(ui, meta, scalars, component, after_item),
        UiKind::Color => draw_color(
            ui,
            meta,
            scalars,
            component,
            ColorMapping::Identity,
            after_item,
        ),
        UiKind::Absorption => draw_color(
            ui,
            meta,
            scalars,
            component,
            ColorMapping::Transmitted,
            after_item,
        ),
        UiKind::Offset => draw_offset(ui, meta, scalars, component, after_item),
    }
}

fn draw_scalar<C>(
    ui: &imgui::Ui,
    meta: &UiParam,
    scalars: &[ScalarParam<C>],
    component: &mut C,
    after_item: &mut impl FnMut(&imgui::Ui, EditedScalars),
) -> bool {
    let Some(scalar) = find_scalar_param(scalars, meta.name) else {
        return false;
    };

    let mut value = (scalar.get)(component);
    property_label(ui, &meta.display_label());
    let changed = ui
        .slider_config(format!("##{}", meta.name), meta.min, meta.max)
        .display_format(numeric_format(meta.format))
        .build(&mut value);
    if changed {
        (scalar.set)(component, value);
    }
    show_tooltip(ui, meta.tooltip);
    after_item(ui, &[(scalar.name, value)]);
    changed
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
) -> bool {
    let component_names = meta.color_component_names();
    let Some(channels) = resolve_channels(scalars, &component_names) else {
        return false;
    };

    let stored = channels.map(|channel| (channel.get)(component));
    let mut picked = mapping.to_picker(stored);
    property_label(ui, &meta.display_label());
    let changed = ui
        .color_picker3_config(format!("##{}", meta.name), &mut picked)
        .flags(ColorPickerOptions {
            flags: ColorPickerFlags::NO_ALPHA | ColorPickerFlags::NO_INPUTS,
            data_type: Some(ColorDataType::Float),
            ..ColorPickerOptions::default()
        })
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
    changed
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
) -> bool {
    let component_names = meta.offset_component_names();
    let Some(channels) = resolve_channels(scalars, &component_names) else {
        return false;
    };

    let stored = channels.map(|channel| (channel.get)(component));
    let mut dragged = stored;
    property_label(ui, &meta.display_label());
    let changed = vector3_field(ui, meta.name, &mut dragged, 1.0, meta.format);
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
    changed
}

fn show_tooltip(ui: &imgui::Ui, tooltip: &str) {
    if !tooltip.is_empty() && ui.is_item_hovered() {
        ui.tooltip_text(tooltip);
    }
}

pub fn find_duplicate_labels<C>(rows: &[UiRow<C>]) -> Vec<&str> {
    let labels: Vec<&str> = rows.iter().filter_map(row_label).collect();
    let mut duplicates = Vec::new();
    for (index, label) in labels.iter().enumerate() {
        if labels[..index].contains(label) && !duplicates.contains(label) {
            duplicates.push(*label);
        }
    }
    duplicates
}

fn row_label<C>(row: &UiRow<C>) -> Option<&str> {
    match row {
        UiRow::Scalar { param, .. } => Some(param.name),
        UiRow::Toggle { label, .. }
        | UiRow::Choice { label, .. }
        | UiRow::Text { label, .. }
        | UiRow::Action { label, .. } => Some(label),
        UiRow::List(list) => Some(list.title()),
        UiRow::Canvas(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toggle(label: &'static str) -> UiRow<()> {
        UiRow::Toggle {
            label,
            get: |_| false,
            set: |_, _| {},
        }
    }

    fn list(title: &'static str) -> UiRow<()> {
        UiRow::List(Box::new(ListRows::<(), ()> {
            title,
            items: |_, _| Vec::new(),
            rows: Vec::new,
            on_change: |_, _, _| unreachable!(),
        }))
    }

    #[test]
    fn unique_labels_have_no_duplicates() {
        assert!(find_duplicate_labels(&[toggle("a"), toggle("b"), list("c")]).is_empty());
    }

    #[test]
    fn each_duplicate_label_is_reported_once() {
        let rows = [
            toggle("a"),
            toggle("b"),
            toggle("a"),
            toggle("b"),
            toggle("a"),
        ];
        assert_eq!(find_duplicate_labels(&rows), ["a", "b"]);
    }

    #[test]
    fn list_title_collides_with_a_row_label() {
        assert_eq!(find_duplicate_labels(&[toggle("a"), list("a")]), ["a"]);
    }

    #[test]
    fn canvas_rows_have_no_label() {
        let rows = [UiRow::Canvas(|_, _, _| {}), UiRow::Canvas(|_, _, _| {})];
        assert!(find_duplicate_labels::<()>(&rows).is_empty());
    }
}
