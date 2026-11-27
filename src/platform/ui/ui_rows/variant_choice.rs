/// A `UiRow::Choice` over every variant of an enum that declares `ALL` and `label()`.
#[macro_export]
macro_rules! variant_choice_row {
    ($label:expr, $view:ty, $variant:ty, $($field:ident).+) => {
        $crate::platform::ui::ui_rows::UiRow::Choice {
            label: $label,
            options: |_world, view: &$view| {
                <$variant>::ALL
                    .iter()
                    .map(|&variant| $crate::platform::ui::ui_rows::ChoiceOption {
                        label: variant.label().to_string(),
                        selected: variant == view.$($field).+,
                    })
                    .collect()
            },
            pick: |view: &mut $view, index| {
                if let Some(&variant) = <$variant>::ALL.get(index) {
                    view.$($field).+ = variant;
                }
            },
        }
    };
}
