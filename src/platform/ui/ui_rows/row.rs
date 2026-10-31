use super::draw::draw_rows;
use crate::ecs::component::ScalarChannelDomain;
use crate::ecs::events::UiCommand;
use crate::ecs::World;
use crate::platform::ui::theme::{section_header, SectionDefault};
use std::borrow::Cow;
use thyllore_scene_core::{ScalarParam, UiParam};

/// Scalar rows whose name resolves in the domain get a key button after the widget.
pub type KeyDomain = Option<&'static ScalarChannelDomain>;

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
    Apply {
        label: &'static str,
        enabled: fn(&World, &C) -> bool,
        apply: fn(&mut C),
    },
    Slider {
        label: &'static str,
        min: f32,
        max: f32,
        format: &'static str,
        get: fn(&C) -> f32,
        set: fn(&mut C, f32),
    },
    IntSlider {
        label: &'static str,
        min: i32,
        max: i32,
        get: fn(&C) -> i32,
        set: fn(&mut C, i32),
    },
    IntInput {
        label: &'static str,
        get: fn(&C) -> i32,
        set: fn(&mut C, i32),
    },
    FloatInput {
        label: &'static str,
        format: &'static str,
        get: fn(&C) -> f32,
        set: fn(&mut C, f32),
    },
    Vector3 {
        label: &'static str,
        speed: f32,
        format: &'static str,
        get: fn(&C) -> [f32; 3],
        set: fn(&mut C, [f32; 3]),
    },
    Sliders {
        items: fn(&World, &C) -> Vec<NamedValue>,
        min: f32,
        max: f32,
        on_change: fn(&C, &NamedValue, f32) -> Box<dyn UiCommand>,
    },
    Choices {
        items: fn(&World, &C) -> Vec<NamedChoice>,
        on_pick: fn(&C, &NamedChoice, usize) -> Box<dyn UiCommand>,
    },
    Group {
        title: Cow<'static, str>,
        initially_open: bool,
        rows: Vec<UiRow<C>>,
    },
    Heading(&'static str),
    Tooltip(&'static str),
    List(Box<dyn DrawList<C>>),
    Canvas(fn(&imgui::Ui, &World, &C)),
    Switch {
        label: &'static str,
        get: fn(&C) -> bool,
        set: fn(&mut C, bool),
    },
    Color3 {
        label: &'static str,
        get: fn(&C) -> [f32; 3],
        set: fn(&mut C, [f32; 3]),
    },
    Field(Box<dyn DrawField<C>>),
}

#[derive(Clone)]
pub struct ChoiceOption {
    pub label: String,
    pub selected: bool,
}

#[derive(Clone)]
pub struct NamedValue {
    pub label: String,
    pub key: usize,
    pub value: f32,
}

#[derive(Clone)]
pub struct NamedChoice {
    pub label: String,
    pub key: usize,
    pub emphasized: bool,
    pub options: Vec<ChoiceOption>,
}

pub struct ListRows<C, I: 'static> {
    pub title: &'static str,
    pub items: fn(&World, &C) -> Vec<(String, I)>,
    pub rows: fn(&I) -> Vec<UiRow<I>>,
    pub on_change: fn(&C, usize, I) -> Box<dyn UiCommand>,
    pub initially_open: fn(&I) -> bool,
}

pub trait DrawField<C> {
    fn draw(&self, ui: &imgui::Ui, world: &World, value: &mut C, keys: KeyDomain) -> bool;
}

pub struct FieldRows<C, I: 'static> {
    pub field: fn(&mut C) -> &mut I,
    pub rows: Vec<UiRow<I>>,
}

impl<C: 'static, I: 'static> DrawField<C> for FieldRows<C, I> {
    fn draw(&self, ui: &imgui::Ui, world: &World, value: &mut C, keys: KeyDomain) -> bool {
        let item = (self.field)(value);
        draw_rows(ui, world, &self.rows, item, keys)
    }
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
        for (index, (name, item)) in (self.items)(world, owner).into_iter().enumerate() {
            self.draw_item(ui, world, owner, index, &name, item);
        }
    }
}

impl<C: 'static, I: 'static> ListRows<C, I> {
    fn draw_item(
        &self,
        ui: &imgui::Ui,
        world: &World,
        owner: &C,
        index: usize,
        name: &str,
        mut item: I,
    ) {
        let _item_id = ui.push_id(index);
        let section_default = if (self.initially_open)(&item) {
            SectionDefault::Open
        } else {
            SectionDefault::Closed
        };
        if !section_header(ui, world, name, section_default) {
            return;
        }
        let rows = (self.rows)(&item);
        if draw_rows(ui, world, &rows, &mut item, None) {
            world.send_boxed_command((self.on_change)(owner, index, item));
        }
    }
}

// imgui activates a slider as a text input when a keyboard focus request lands on it.
