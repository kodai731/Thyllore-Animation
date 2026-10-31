use crate::ecs::events::UiCommand;

use super::row::*;

pub fn find_duplicate_labels<C>(rows: &[UiRow<C>]) -> Vec<&str> {
    let labels: Vec<&str> = row_labels(rows).into_iter().flatten().collect();
    let mut duplicates = Vec::new();
    for (index, label) in labels.iter().enumerate() {
        if labels[..index].contains(label) && !duplicates.contains(label) {
            duplicates.push(*label);
        }
    }
    duplicates
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
