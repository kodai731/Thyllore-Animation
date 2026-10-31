use crate::ecs::events::UiCommand;

use super::row::*;

/// Widget ids that collide inside one row list, fields included as `scope/label`.
pub fn find_duplicate_labels<C>(rows: &[UiRow<C>]) -> Vec<String> {
    let ids = widget_ids(rows);
    let mut duplicates = Vec::new();
    for (index, id) in ids.iter().enumerate() {
        if ids[..index].contains(id) && !duplicates.contains(id) {
            duplicates.push(id.clone());
        }
    }
    duplicates
}

/// Every id imgui sees in this row list: labels at this level and the scoped ids of each field and group.
pub fn widget_ids<C>(rows: &[UiRow<C>]) -> Vec<String> {
    rows.iter()
        .flat_map(|row| match row {
            UiRow::Field(field) => scoped_ids(field.scope(), field.widget_ids()),
            UiRow::Group { title, rows, .. } => scoped_ids(title, widget_ids(rows)),
            _ => row_label(row).map(str::to_string).into_iter().collect(),
        })
        .collect()
}

fn scoped_ids(scope: &str, inner: Vec<String>) -> Vec<String> {
    std::iter::once(scope.to_string())
        .chain(inner.into_iter().map(|id| format!("{scope}/{id}")))
        .collect()
}

pub fn row_labels<C>(rows: &[UiRow<C>]) -> Vec<Option<&str>> {
    rows.iter().map(row_label).collect()
}

pub fn visible_labels<C>(rows: &[UiRow<C>]) -> Vec<&str> {
    row_labels(rows).into_iter().flatten().collect()
}

fn row_label<C>(row: &UiRow<C>) -> Option<&str> {
    match row {
        UiRow::Scalar { param, .. } => Some(param.name),
        UiRow::Toggle { label, .. }
        | UiRow::Choice { label, .. }
        | UiRow::Text { label, .. }
        | UiRow::Action { label, .. }
        | UiRow::Apply { label, .. }
        | UiRow::Slider { label, .. }
        | UiRow::IntSlider { label, .. }
        | UiRow::IntInput { label, .. }
        | UiRow::FloatInput { label, .. }
        | UiRow::Vector3 { label, .. }
        | UiRow::Switch { label, .. }
        | UiRow::Color3 { label, .. } => Some(label),
        UiRow::List(list) => Some(list.title()),
        UiRow::Canvas(_)
        | UiRow::Sliders { .. }
        | UiRow::Choices { .. }
        | UiRow::Group { .. }
        | UiRow::Heading(_)
        | UiRow::Tooltip(_)
        | UiRow::Field(_) => None,
    }
}
