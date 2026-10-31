use crate::ecs::events::UiCommand;

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
        rows: |_| Vec::new(),
        on_change: |_, _, _| unreachable!(),
        initially_open: |_| false,
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

#[test]
fn new_variants_appear_in_visible_labels() {
    let rows: Vec<UiRow<()>> = vec![
        UiRow::Slider {
            label: "S",
            min: 0.0,
            max: 1.0,
            format: "%.2f",
            get: |_| 0.0,
            set: |_, _| {},
        },
        UiRow::IntSlider {
            label: "IS",
            min: 0,
            max: 10,
            get: |_| 0,
            set: |_, _| {},
        },
        UiRow::IntInput {
            label: "II",
            get: |_| 0,
            set: |_, _| {},
        },
        UiRow::FloatInput {
            label: "FI",
            format: "%.3f",
            get: |_| 0.0,
            set: |_, _| {},
        },
        UiRow::Vector3 {
            label: "V3",
            speed: 1.0,
            format: "%.2f",
            get: |_| [0.0, 0.0, 0.0],
            set: |_, _| {},
        },
    ];
    assert_eq!(visible_labels(&rows), ["S", "IS", "II", "FI", "V3"]);
}

#[test]
fn new_variants_duplicate_detection() {
    let rows: Vec<UiRow<()>> = vec![
        UiRow::Slider {
            label: "dup",
            min: 0.0,
            max: 1.0,
            format: "%.2f",
            get: |_| 0.0,
            set: |_, _| {},
        },
        UiRow::IntSlider {
            label: "dup",
            min: 0,
            max: 10,
            get: |_| 0,
            set: |_, _| {},
        },
        UiRow::Vector3 {
            label: "uniq",
            speed: 1.0,
            format: "%.2f",
            get: |_| [0.0, 0.0, 0.0],
            set: |_, _| {},
        },
    ];
    assert_eq!(find_duplicate_labels(&rows), ["dup"]);
}

#[test]
fn switch_and_color3_appear_in_visible_labels() {
    let rows: Vec<UiRow<()>> = vec![
        UiRow::Switch {
            label: "SW",
            get: |_| false,
            set: |_, _| {},
        },
        UiRow::Color3 {
            label: "C3",
            get: |_| [0.0, 0.0, 0.0],
            set: |_, _| {},
        },
    ];
    assert_eq!(visible_labels(&rows), ["SW", "C3"]);
}

#[test]
fn switch_and_color3_duplicate_detection() {
    let rows: Vec<UiRow<()>> = vec![
        UiRow::Switch {
            label: "dup",
            get: |_| false,
            set: |_, _| {},
        },
        UiRow::Color3 {
            label: "dup",
            get: |_| [0.0, 0.0, 0.0],
            set: |_, _| {},
        },
    ];
    assert_eq!(find_duplicate_labels(&rows), ["dup"]);
}

#[test]
fn field_rows_row_label_is_none() {
    let rows: Vec<UiRow<()>> = vec![UiRow::Field(Box::new(FieldRows::<(), ()> {
        field: |_| panic!("should not be called"),
        rows: vec![],
    }))];
    assert_eq!(row_labels(&rows), [None]);
}
