use crate::ecs::events::UiCommand;
use crate::ecs::systems::phases::event_dispatch::scalar_curve::ScalarCurveEvent;
use crate::ecs::World;
use crate::platform::ui::numeric_format::numeric_format;
use crate::platform::ui::pointer::is_last_item_double_clicked;
use crate::platform::ui::theme::{
    label_column_width, property_label_colored, property_label_in_column, section_header,
    vector3_field, SectionDefault,
};
use imgui::{ColorDataType, ColorPickerFlags, ColorPickerOptions};
use std::borrow::Cow;
use thyllore_anim_core::editable::PropertyType;
use thyllore_effect_core::{
    absorption_to_transmitted_color, transmitted_color_to_absorption, ABSORPTION_REFERENCE_DISTANCE,
};
use thyllore_scene_core::{find_scalar_param, ScalarParam, UiKind, UiParam};

use super::row::*;

/// Returns whether any row wrote to `value`.
pub fn draw_rows<C>(
    ui: &imgui::Ui,
    world: &World,
    rows: &[UiRow<C>],
    value: &mut C,
    keys: KeyDomain,
) -> bool {
    let label_column = label_column_width(ui, displayed_labels(rows));
    let mut changed = false;
    for row in rows {
        changed |= draw_row(ui, world, row, value, keys, label_column);
    }
    changed
}

fn displayed_labels<C>(rows: &[UiRow<C>]) -> Vec<Cow<'_, str>> {
    rows.iter()
        .flat_map(|row| match row {
            UiRow::Scalar { param, .. } => vec![param.display_label()],
            UiRow::Toggle { label, .. }
            | UiRow::Choice { label, .. }
            | UiRow::Text { label, .. }
            | UiRow::Slider { label, .. }
            | UiRow::IntSlider { label, .. }
            | UiRow::IntInput { label, .. }
            | UiRow::FloatInput { label, .. }
            | UiRow::Vector3 { label, .. }
            | UiRow::Switch { label, .. }
            | UiRow::Color3 { label, .. } => vec![Cow::Borrowed(*label)],
            UiRow::Group { rows, .. } => displayed_labels(rows),
            UiRow::Action { .. }
            | UiRow::Apply { .. }
            | UiRow::Sliders { .. }
            | UiRow::Choices { .. }
            | UiRow::Heading(_)
            | UiRow::Tooltip(_)
            | UiRow::List(_)
            | UiRow::Canvas(_)
            | UiRow::Field(_) => Vec::new(),
        })
        .collect()
}

fn draw_row<C>(
    ui: &imgui::Ui,
    world: &World,
    row: &UiRow<C>,
    value: &mut C,
    keys: KeyDomain,
    label_column: f32,
) -> bool {
    match row {
        UiRow::Scalar { param, scalars } => {
            draw_scalar_row(ui, world, param, scalars, value, keys, label_column)
        }
        UiRow::Toggle { label, get, set } => {
            draw_toggle(ui, label, *get, *set, value, label_column)
        }
        UiRow::Choice {
            label,
            options,
            pick,
        } => draw_choice(
            ui,
            label,
            &options(world, value),
            *pick,
            value,
            label_column,
        ),
        UiRow::Text { label, get, set } => draw_text(ui, label, *get, *set, value, label_column),
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
        UiRow::Sliders {
            items,
            min,
            max,
            on_change,
        } => {
            draw_sliders(
                ui,
                world,
                &items(world, value),
                (*min, *max),
                *on_change,
                value,
                label_column,
            );
            false
        }
        UiRow::Choices { items, on_pick } => {
            draw_choices(ui, world, &items(world, value), *on_pick, value);
            false
        }
        UiRow::Apply {
            label,
            enabled,
            apply,
        } => {
            let _disabled = ui.begin_disabled_with_cond(!enabled(world, value));
            let clicked = ui.button(label);
            if clicked {
                apply(value);
            }
            clicked
        }
        UiRow::Slider {
            label,
            min,
            max,
            format,
            get,
            set,
        } => {
            let mut v = (*get)(value);
            property_label_in_column(ui, label, label_column);
            let changed = ui
                .slider_config(format!("##{}", label), *min, *max)
                .display_format(numeric_format(format))
                .build(&mut v);
            open_text_entry_on_double_click(ui);
            if changed {
                (*set)(value, v);
                return true;
            }
            false
        }
        UiRow::IntSlider {
            label,
            min,
            max,
            get,
            set,
        } => {
            let mut v = (*get)(value);
            property_label_in_column(ui, label, label_column);
            let changed = ui
                .slider_config(format!("##{}", label), *min, *max)
                .build(&mut v);
            open_text_entry_on_double_click(ui);
            if changed {
                (*set)(value, v);
                return true;
            }
            false
        }
        UiRow::IntInput { label, get, set } => {
            let mut v = (*get)(value);
            property_label_in_column(ui, label, label_column);
            if ui.input_int(format!("##{}", label), &mut v) {
                (*set)(value, v);
                return true;
            }
            false
        }
        UiRow::FloatInput {
            label,
            format,
            get,
            set,
        } => {
            let mut v = (*get)(value);
            property_label_in_column(ui, label, label_column);
            if ui
                .input_float_config(format!("##{}", label))
                .display_format(numeric_format(format))
                .build(&mut v)
            {
                (*set)(value, v);
                return true;
            }
            false
        }
        UiRow::Vector3 {
            label,
            speed,
            format,
            get,
            set,
        } => {
            let mut v = (*get)(value);
            property_label_in_column(ui, label, label_column);
            if vector3_field(ui, &format!("##{}", label), &mut v, *speed, format) {
                (*set)(value, v);
                return true;
            }
            false
        }
        UiRow::Group {
            title,
            initially_open,
            rows,
        } => {
            let default = if *initially_open {
                SectionDefault::Open
            } else {
                SectionDefault::Closed
            };
            if !section_header(ui, world, &title, default) {
                return false;
            }
            let _id = ui.push_id(title.as_ref());
            let mut group_changed = false;
            for row in rows {
                group_changed |= draw_row(ui, world, row, value, keys, label_column);
            }
            group_changed
        }
        UiRow::Heading(label) => {
            ui.text(label);
            false
        }
        UiRow::Tooltip(tooltip) => {
            show_tooltip(ui, tooltip);
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
        UiRow::Switch { label, get, set } => {
            let mut v = (*get)(value);
            property_label_in_column(ui, label, label_column);
            if crate::platform::ui::theme::toggle_switch(ui, world, &format!("##{label}"), &mut v) {
                (*set)(value, v);
                return true;
            }
            false
        }
        UiRow::Color3 { label, get, set } => {
            let mut v = (*get)(value);
            property_label_in_column(ui, label, label_column);
            if ui.color_edit3(format!("##{}", label), &mut v) {
                (*set)(value, v);
                return true;
            }
            false
        }
        UiRow::Field(field) => field.draw(ui, world, value, keys),
    }
}

fn draw_toggle<C>(
    ui: &imgui::Ui,
    label: &str,
    get: fn(&C) -> bool,
    set: fn(&mut C, bool),
    value: &mut C,
    label_column: f32,
) -> bool {
    let mut checked = get(value);
    property_label_in_column(ui, label, label_column);
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
    label_column: f32,
) -> bool {
    let preview = options
        .iter()
        .find(|option| option.selected)
        .map_or("", |option| option.label.as_str());
    property_label_in_column(ui, label, label_column);
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
    label_column: f32,
) -> bool {
    let mut text = get(value).to_owned();
    property_label_in_column(ui, label, label_column);
    if !ui.input_text(format!("##{label}"), &mut text).build() {
        return false;
    }
    set(value, text);
    true
}

fn draw_sliders<C>(
    ui: &imgui::Ui,
    world: &World,
    items: &[NamedValue],
    (min, max): (f32, f32),
    on_change: fn(&C, &NamedValue, f32) -> Box<dyn UiCommand>,
    value: &C,
    label_column: f32,
) {
    for item in items {
        let mut slider_value = item.value;
        property_label_in_column(ui, &item.label, label_column);
        ui.set_next_item_width(-1.0);
        if ui.slider(format!("##{}", item.key), min, max, &mut slider_value) {
            world.send_boxed_command(on_change(value, item, slider_value));
        }
    }
}

const CHOICES_WARNING_COLOR: [f32; 4] = [1.0, 0.6, 0.2, 1.0];

fn draw_choices<C>(
    ui: &imgui::Ui,
    world: &World,
    items: &[NamedChoice],
    on_pick: fn(&C, &NamedChoice, usize) -> Box<dyn UiCommand>,
    value: &C,
) {
    for item in items {
        let label_color = if item.emphasized {
            CHOICES_WARNING_COLOR
        } else {
            crate::platform::ui::theme::colors::srgb_to_linear_rgba(
                crate::platform::ui::theme::colors::TEXT_SECONDARY,
            )
        };
        let preview = item
            .options
            .iter()
            .find(|option| option.selected)
            .map_or("", |option| option.label.as_str());
        property_label_colored(ui, &item.label, label_color);
        let Some(combo) = ui.begin_combo(format!("##{}", item.key), preview) else {
            continue;
        };
        for (index, option) in item.options.iter().enumerate() {
            if ui
                .selectable_config(&option.label)
                .selected(option.selected)
                .build()
            {
                world.send_boxed_command(on_pick(value, item, index));
            }
        }
        combo.end();
    }
}

fn draw_scalar_row<C>(
    ui: &imgui::Ui,
    world: &World,
    meta: &UiParam,
    scalars: &[ScalarParam<C>],
    component: &mut C,
    keys: KeyDomain,
    label_column: f32,
) -> bool {
    match meta.kind {
        UiKind::Scalar => draw_scalar(ui, world, meta, scalars, component, keys, label_column),
        UiKind::Color => draw_color(
            ui,
            world,
            meta,
            scalars,
            component,
            ColorMapping::Identity,
            keys,
            label_column,
        ),
        UiKind::Absorption => draw_color(
            ui,
            world,
            meta,
            scalars,
            component,
            ColorMapping::Transmitted,
            keys,
            label_column,
        ),
        UiKind::Offset => draw_offset(ui, world, meta, scalars, component, keys, label_column),
    }
}

const KEY_BUTTON_LABEL: &str = "K";

/// Resolves the channels this widget edits to the curve properties a key button writes.
fn key_targets<C>(keys: KeyDomain, channels: &[&ScalarParam<C>]) -> Vec<PropertyType> {
    let Some(domain) = keys else {
        return Vec::new();
    };
    channels
        .iter()
        .filter_map(|channel| domain.property_type_for_cli_name(channel.name))
        .collect()
}

fn reserve_key_button_width(ui: &imgui::Ui, targets: &[PropertyType]) {
    if targets.is_empty() {
        return;
    }
    let style = ui.clone_style();
    let button_width = ui.calc_text_size(KEY_BUTTON_LABEL)[0] + 2.0 * style.frame_padding()[0];
    ui.set_next_item_width(-(button_width + style.item_spacing()[0]));
}

fn draw_key_button(
    ui: &imgui::Ui,
    world: &World,
    id: &str,
    targets: &[PropertyType],
    values: &[f32],
) {
    if targets.is_empty() {
        return;
    }
    ui.same_line();
    if !ui.small_button(format!("{KEY_BUTTON_LABEL}##{id}")) {
        return;
    }
    for (&property_type, &value) in targets.iter().zip(values) {
        world.send_command(ScalarCurveEvent::InsertScalarKey {
            property_type,
            value,
        });
    }
}

fn draw_scalar<C>(
    ui: &imgui::Ui,
    world: &World,
    meta: &UiParam,
    scalars: &[ScalarParam<C>],
    component: &mut C,
    keys: KeyDomain,
    label_column: f32,
) -> bool {
    let Some(scalar) = find_scalar_param(scalars, meta.name) else {
        return false;
    };
    let targets = key_targets(keys, &[scalar]);

    let mut value = (scalar.get)(component);
    property_label_in_column(ui, &meta.display_label(), label_column);
    reserve_key_button_width(ui, &targets);
    let changed = ui
        .slider_config(format!("##{}", meta.name), meta.min, meta.max)
        .display_format(numeric_format(meta.format))
        .build(&mut value);
    open_text_entry_on_double_click(ui);
    if changed {
        (scalar.set)(component, value);
    }
    show_tooltip(ui, meta.tooltip);
    draw_key_button(ui, world, meta.name, &targets, &[value]);
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
    world: &World,
    meta: &UiParam,
    scalars: &[ScalarParam<C>],
    component: &mut C,
    mapping: ColorMapping,
    keys: KeyDomain,
    label_column: f32,
) -> bool {
    let component_names = meta.color_component_names();
    let Some(channels) = resolve_channels(scalars, &component_names) else {
        return false;
    };
    let targets = key_targets(keys, &channels);

    let stored = channels.map(|channel| (channel.get)(component));
    let mut picked = mapping.to_picker(stored);
    property_label_in_column(ui, &meta.display_label(), label_column);
    reserve_key_button_width(ui, &targets);
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

    draw_key_button(ui, world, meta.name, &targets, &written);
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
    world: &World,
    meta: &UiParam,
    scalars: &[ScalarParam<C>],
    component: &mut C,
    keys: KeyDomain,
    label_column: f32,
) -> bool {
    let component_names = meta.offset_component_names();
    let Some(channels) = resolve_channels(scalars, &component_names) else {
        return false;
    };
    let targets = key_targets(keys, &channels);

    let stored = channels.map(|channel| (channel.get)(component));
    let mut dragged = stored;
    property_label_in_column(ui, &meta.display_label(), label_column);
    reserve_key_button_width(ui, &targets);
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

    draw_key_button(ui, world, meta.name, &targets, &written);
    changed
}

fn open_text_entry_on_double_click(ui: &imgui::Ui) {
    if is_last_item_double_clicked(ui) {
        ui.set_keyboard_focus_here_with_offset(-1);
    }
}

fn show_tooltip(ui: &imgui::Ui, tooltip: &str) {
    if !tooltip.is_empty() && ui.is_item_hovered() {
        ui.tooltip_text(tooltip);
    }
}
