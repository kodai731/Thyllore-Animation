use imgui::Condition;

use crate::ecs::World;
use crate::platform::ui::theme::shadow::draw_window_shadow;
use crate::platform::ui::theme::{draw_raised_window_frame, push_raised_window_style};

use super::param_widgets::EditedScalars;
use super::ui_rows::{draw_rows, UiRow};

pub trait FormSection {
    fn draw(self: Box<Self>, ui: &imgui::Ui, world: &World);
}

pub struct RowsSection<V: Clone + 'static> {
    pub rows: Vec<UiRow<V>>,
    pub view: V,
    pub commit: fn(&World, &V, &V),
    pub after_item: Option<fn(&imgui::Ui, &World, EditedScalars)>,
}

impl<V: Clone + 'static> FormSection for RowsSection<V> {
    fn draw(self: Box<Self>, ui: &imgui::Ui, world: &World) {
        let shown = self.view.clone();
        let mut view = self.view;
        draw_rows(ui, world, &self.rows, &mut view, |ui, edited| {
            if let Some(ref after_item) = self.after_item {
                after_item(ui, world, edited);
            }
        });
        (self.commit)(world, &shown, &view);
    }
}

pub fn rows_section<V: Clone + 'static>(
    rows: Vec<UiRow<V>>,
    view: V,
    commit: fn(&World, &V, &V),
) -> Box<dyn FormSection> {
    Box::new(RowsSection {
        rows,
        view,
        commit,
        after_item: None,
    })
}

pub fn message_section(
    title: &'static str,
    initially_open: bool,
    message: String,
) -> Box<dyn FormSection> {
    rows_section(
        vec![UiRow::Group {
            title: title.into(),
            initially_open,
            rows: vec![UiRow::Canvas(|ui, _, message: &String| ui.text(message))],
        }],
        message,
        |_, _, _| {},
    )
}

pub enum FormPlacement {
    Docked {
        position: [f32; 2],
        size: [f32; 2],
    },
    Floating {
        initial_size: [f32; 2],
        on_close: fn(&World),
    },
    Overlay {
        position: [f32; 2],
        width: f32,
        max_height: f32,
    },
}

pub struct FormWindow {
    pub title: &'static str,
    pub placement: FormPlacement,
    pub sections: Vec<Box<dyn FormSection>>,
}

pub fn draw_form_window(ui: &imgui::Ui, world: &World, window: FormWindow) {
    match window.placement {
        FormPlacement::Docked { position, size } => {
            ui.window(window.title)
                .position(position, Condition::Always)
                .size(size, Condition::Always)
                .resizable(false)
                .movable(false)
                .collapsible(false)
                .bring_to_front_on_focus(false)
                .build(|| {
                    for section in window.sections {
                        section.draw(ui, world);
                    }
                });
        }
        FormPlacement::Floating {
            initial_size,
            on_close,
        } => {
            let mut is_open = true;
            let _raised = push_raised_window_style(ui);
            ui.window(window.title)
                .size(initial_size, Condition::FirstUseEver)
                .movable(true)
                .opened(&mut is_open)
                .build(|| {
                    draw_raised_window_frame(ui);
                    for section in window.sections {
                        section.draw(ui, world);
                    }
                });
            if !is_open {
                (on_close)(world);
            }
        }
        FormPlacement::Overlay {
            position,
            width,
            max_height,
        } => {
            ui.window(window.title)
                .position(position, Condition::Always)
                .size_constraints([width, 0.0], [width, max_height])
                .always_auto_resize(true)
                .no_decoration()
                .bg_alpha(0.7)
                .no_nav()
                .focus_on_appearing(false)
                .save_settings(false)
                .build(|| {
                    draw_window_shadow(ui, ui.clone_style().window_rounding());
                    for section in window.sections {
                        section.draw(ui, world);
                    }
                });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_form_window_constructs() {
        let section = rows_section::<()>(Vec::new(), (), |_world, _a, _b| {});
        let window = FormWindow {
            title: "Test",
            placement: FormPlacement::Docked {
                position: [0.0, 0.0],
                size: [100.0, 100.0],
            },
            sections: vec![section],
        };
        assert_eq!(window.title, "Test");
        assert_eq!(window.sections.len(), 1);
    }

    #[test]
    fn test_form_placement_floating() {
        let window = FormWindow {
            title: "Floating",
            placement: FormPlacement::Floating {
                initial_size: [200.0, 300.0],
                on_close: |_world| {},
            },
            sections: vec![],
        };
        match window.placement {
            FormPlacement::Floating { initial_size, .. } => {
                assert_eq!(initial_size, [200.0, 300.0]);
            }
            _ => panic!("expected Floating"),
        }
    }

    #[test]
    fn test_form_placement_overlay() {
        let window = FormWindow {
            title: "Overlay",
            placement: FormPlacement::Overlay {
                position: [10.0, 20.0],
                width: 400.0,
                max_height: 600.0,
            },
            sections: vec![],
        };
        match window.placement {
            FormPlacement::Overlay {
                position,
                width,
                max_height,
            } => {
                assert_eq!(position, [10.0, 20.0]);
                assert_eq!(width, 400.0);
                assert_eq!(max_height, 600.0);
            }
            _ => panic!("expected Overlay"),
        }
    }
}
