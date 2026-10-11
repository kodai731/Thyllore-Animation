use crate::animation::editable::{EditableKeyframe, PropertyCurve};
use crate::ecs::resource::{CurveEditorState, CurveSelectedKeyframe, CurveTrackRef};
use crate::ecs::systems::phases::event_dispatch::timeline::TimelineEvent;
use crate::ecs::world::World;

const SHARED_VALUE_TOLERANCE: f32 = 1e-6;
const FIELD_WIDTH: f32 = 80.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NumericEdit {
    Set(f32),
    Add(f32),
    Multiply(f32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum KeyField {
    Time,
    Value,
}

pub fn parse_numeric_edit(text: &str) -> Option<NumericEdit> {
    let text = text.trim();

    if let Some(rest) = text.strip_prefix("*=") {
        rest.trim().parse().ok().map(NumericEdit::Multiply)
    } else if let Some(rest) = text.strip_prefix("+=") {
        rest.trim().parse().ok().map(NumericEdit::Add)
    } else if let Some(rest) = text.strip_prefix("-=") {
        rest.trim()
            .parse::<f32>()
            .ok()
            .map(|delta| NumericEdit::Add(-delta))
    } else {
        text.parse().ok().map(NumericEdit::Set)
    }
}

pub fn apply_numeric_edit(edit: NumericEdit, current: f32) -> f32 {
    match edit {
        NumericEdit::Set(value) => value,
        NumericEdit::Add(delta) => current + delta,
        NumericEdit::Multiply(factor) => current * factor,
    }
}

pub fn shared_value(mut values: impl Iterator<Item = f32>) -> Option<f32> {
    let first = values.next()?;
    values
        .all(|value| (value - first).abs() <= SHARED_VALUE_TOLERANCE)
        .then_some(first)
}

pub(super) fn build_selected_key_fields(
    ui: &imgui::Ui,
    world: &World,
    editor_state: &mut CurveEditorState,
    curves: &[(&PropertyCurve, [f32; 4], &str)],
    track_ref: CurveTrackRef,
) {
    if editor_state.selected_keyframes.is_empty() {
        return;
    }

    let selected_keys = collect_selected_keys(&editor_state.selected_keyframes, curves);
    let shared_time = shared_value(selected_keys.iter().map(|(_, key)| key.time));
    let shared_key_value = shared_value(selected_keys.iter().map(|(_, key)| key.value));

    ui.same_line_with_spacing(0.0, 20.0);
    let time_edit = build_numeric_field(ui, "Time", &mut editor_state.time_field, shared_time);
    ui.same_line();
    let value_edit =
        build_numeric_field(ui, "Value", &mut editor_state.value_field, shared_key_value);

    if let Some(edit) = time_edit {
        send_key_edits(world, track_ref, &selected_keys, edit, KeyField::Time);
    }
    if let Some(edit) = value_edit {
        send_key_edits(world, track_ref, &selected_keys, edit, KeyField::Value);
    }
}

fn collect_selected_keys<'a>(
    selected_keyframes: &'a [CurveSelectedKeyframe],
    curves: &[(&'a PropertyCurve, [f32; 4], &str)],
) -> Vec<(&'a CurveSelectedKeyframe, &'a EditableKeyframe)> {
    selected_keyframes
        .iter()
        .filter_map(|selected| {
            curves
                .iter()
                .find(|(curve, _, _)| curve.property_type == selected.property_type)
                .and_then(|(curve, _, _)| curve.get_keyframe(selected.keyframe_id))
                .map(|key| (selected, key))
        })
        .collect()
}

fn build_numeric_field(
    ui: &imgui::Ui,
    label: &str,
    field_text: &mut String,
    shared: Option<f32>,
) -> Option<NumericEdit> {
    ui.text(label);
    ui.same_line();
    ui.set_next_item_width(FIELD_WIDTH);
    let is_entered = ui
        .input_text(format!("##key_{}", label), field_text)
        .hint("mixed")
        .enter_returns_true(true)
        .build();
    let edit = if is_entered {
        parse_numeric_edit(field_text)
    } else {
        None
    };

    if !ui.is_item_active() {
        *field_text = shared
            .map(|value| format!("{:.4}", value))
            .unwrap_or_default();
    }
    edit
}

fn send_key_edits(
    world: &World,
    track_ref: CurveTrackRef,
    selected_keys: &[(&CurveSelectedKeyframe, &EditableKeyframe)],
    edit: NumericEdit,
    field: KeyField,
) {
    for (selected, key) in selected_keys {
        let (new_time, new_value) = match field {
            KeyField::Time => (apply_numeric_edit(edit, key.time).max(0.0), key.value),
            KeyField::Value => (key.time, apply_numeric_edit(edit, key.value)),
        };
        world.send_command(TimelineEvent::MoveKeyframe {
            track: track_ref,
            property_type: selected.property_type,
            keyframe_id: selected.keyframe_id,
            new_time,
            new_value,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_absolute_value() {
        assert_eq!(parse_numeric_edit("0.5"), Some(NumericEdit::Set(0.5)));
        assert_eq!(parse_numeric_edit("-1.2"), Some(NumericEdit::Set(-1.2)));
        assert_eq!(parse_numeric_edit("  3  "), Some(NumericEdit::Set(3.0)));
    }

    #[test]
    fn parse_accepts_add_and_subtract() {
        assert_eq!(parse_numeric_edit("+=0.1"), Some(NumericEdit::Add(0.1)));
        assert_eq!(parse_numeric_edit("-=0.5"), Some(NumericEdit::Add(-0.5)));
        assert_eq!(parse_numeric_edit("  += 2  "), Some(NumericEdit::Add(2.0)));
    }

    #[test]
    fn parse_accepts_multiply() {
        assert_eq!(parse_numeric_edit("*=2"), Some(NumericEdit::Multiply(2.0)));
        assert_eq!(
            parse_numeric_edit("  *=0.5  "),
            Some(NumericEdit::Multiply(0.5))
        );
    }

    #[test]
    fn parse_rejects_invalid_text() {
        assert_eq!(parse_numeric_edit(""), None);
        assert_eq!(parse_numeric_edit("   "), None);
        assert_eq!(parse_numeric_edit("abc"), None);
        assert_eq!(parse_numeric_edit("+="), None);
        assert_eq!(parse_numeric_edit("*=x"), None);
        assert_eq!(parse_numeric_edit("/=2"), None);
    }

    #[test]
    fn apply_edit_sets_adds_and_multiplies() {
        assert_eq!(apply_numeric_edit(NumericEdit::Set(5.0), 1.0), 5.0);
        assert_eq!(apply_numeric_edit(NumericEdit::Add(2.0), 3.0), 5.0);
        assert_eq!(apply_numeric_edit(NumericEdit::Add(-0.5), 3.0), 2.5);
        assert_eq!(apply_numeric_edit(NumericEdit::Multiply(3.0), 2.0), 6.0);
    }

    #[test]
    fn shared_value_requires_all_values_equal() {
        assert_eq!(shared_value([1.0].into_iter()), Some(1.0));
        assert_eq!(shared_value([1.0, 1.0 + 1e-7, 1.0].into_iter()), Some(1.0));
        assert_eq!(shared_value([1.0, 2.0].into_iter()), None);
        assert_eq!(shared_value(std::iter::empty()), None);
    }
}
