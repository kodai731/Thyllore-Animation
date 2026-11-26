use std::borrow::Cow;

use crate::rgb_channels::RGB_CHANNEL_SUFFIXES;

/// Flat-name f32 accessor for one scalar parameter; one static table per component type.
pub struct ScalarParam<C: 'static> {
    pub name: &'static str,
    pub get: fn(&C) -> f32,
    pub set: fn(&mut C, f32),
    pub debug_range: Option<(f32, f32)>,
    /// Former `name`s still accepted when a clip file or CLI flag names the parameter.
    pub renamed_from: &'static [&'static str],
    /// `#[persist(curve)]`: the parameter is an animation-curve channel.
    pub curve: bool,
}

pub fn find_scalar_param<'a, C>(
    params: &'a [ScalarParam<C>],
    name: &str,
) -> Option<&'a ScalarParam<C>> {
    params.iter().find(|param| param.name == name)
}

/// Widget family a parameter is edited with; `Color` and `Absorption` are `[f32; 3]` parameters
/// whose components are reachable through the `<name>_r/_g/_b` scalar aliases. `Offset` is a
/// `[f32; 3]` spatial offset whose components are reachable through the `<name>_x/_y/_z` scalar
/// aliases. `Int` is a whole-number parameter edited in steps of one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiKind {
    Scalar,
    Int,
    /// Whole number whose size carries no meaning, typed in rather than picked from a range.
    Seed,
    Color,
    /// Beer-Lambert coefficients per meter, edited as the colour transmitted over a reference distance.
    Absorption,
    /// 3D spatial offset (x, y, z) in local space, edited with a 3-element drag widget.
    Offset,
}

impl UiKind {
    pub fn name(self) -> &'static str {
        match self {
            UiKind::Scalar => "scalar",
            UiKind::Int => "int",
            UiKind::Seed => "seed",
            UiKind::Color => "color",
            UiKind::Absorption => "absorption",
            UiKind::Offset => "offset",
        }
    }
}

/// A situation in which a parameter has no widget, reported by whoever shows the parameters.
/// Declared as a `static`; two conditions are the same only when they are the same static.
#[derive(Debug)]
pub struct UiCondition {
    pub description: &'static str,
}

/// UI-toolkit-free display metadata of one parameter, joined to the accessor table by `name`.
#[derive(Clone, Copy, Debug)]
pub struct UiParam {
    pub name: &'static str,
    /// Dotted location of the persisted value inside the component's serde form.
    pub path: &'static str,
    pub group: &'static str,
    pub label: Option<&'static str>,
    pub kind: UiKind,
    pub min: f32,
    pub max: f32,
    pub format: &'static str,
    pub tooltip: &'static str,
    /// Persisted parameters are saved with the scene; runtime ones are driven by playback.
    pub persisted: bool,
    pub primary: bool,
    pub hidden_when: Option<&'static UiCondition>,
}

impl UiParam {
    pub fn is_hidden_in(&self, active_conditions: &[&'static UiCondition]) -> bool {
        self.hidden_when.is_some_and(|hidden_when| {
            active_conditions
                .iter()
                .any(|active| std::ptr::eq(*active, hidden_when))
        })
    }

    /// Explicit label, or the parameter name title-cased (`noise_amplitude` -> `Noise Amplitude`).
    pub fn display_label(&self) -> Cow<'static, str> {
        match self.label {
            Some(label) => Cow::Borrowed(label),
            None => Cow::Owned(title_case_snake(self.name)),
        }
    }

    /// Scalar alias names of a `Color` / `Absorption` parameter, in r, g, b order.
    pub fn color_component_names(&self) -> [String; 3] {
        RGB_CHANNEL_SUFFIXES.map(|suffix| format!("{}{}", self.name, suffix))
    }

    /// Scalar alias names of an `Offset` parameter, in x, y, z order.
    pub fn offset_component_names(&self) -> [String; 3] {
        ["_x", "_y", "_z"].map(|suffix| format!("{}{}", self.name, suffix))
    }

    /// Every `ScalarParam` name this parameter's widget reads and writes.
    pub fn scalar_accessor_names(&self) -> Vec<String> {
        match self.kind {
            UiKind::Scalar | UiKind::Int | UiKind::Seed => vec![self.name.to_string()],
            UiKind::Color | UiKind::Absorption => self.color_component_names().to_vec(),
            UiKind::Offset => self.offset_component_names().to_vec(),
        }
    }
}

pub fn title_case_snake(name: &str) -> String {
    name.split('_')
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn find_ui_param<'a>(params: &'a [UiParam], name: &str) -> Option<&'a UiParam> {
    params.iter().find(|param| param.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_title_case_snake_capitalizes_each_word() {
        assert_eq!(title_case_snake("noise_amplitude"), "Noise Amplitude");
        assert_eq!(title_case_snake("height"), "Height");
        assert_eq!(title_case_snake("wien_c_k"), "Wien C K");
    }

    #[test]
    fn test_display_label_prefers_explicit_label() {
        let explicit = UiParam {
            name: "swirl_gain",
            path: "swirl_gain",
            group: "",
            label: Some("Swirl"),
            kind: UiKind::Scalar,
            min: 0.0,
            max: 1.0,
            format: "",
            tooltip: "",
            persisted: true,
            primary: false,
            hidden_when: None,
        };
        let derived = UiParam {
            label: None,
            ..explicit
        };
        assert_eq!(explicit.display_label(), "Swirl");
        assert_eq!(derived.display_label(), "Swirl Gain");
    }

    #[test]
    fn test_param_is_hidden_only_in_the_condition_it_declares() {
        static FOLLOWS_TARGET: UiCondition = UiCondition {
            description: "a target drives the value",
        };
        static OTHER: UiCondition = UiCondition {
            description: "another situation",
        };
        let always_shown = UiParam {
            name: "end_offset",
            path: "end_offset",
            group: "",
            label: None,
            kind: UiKind::Offset,
            min: 0.0,
            max: 1.0,
            format: "",
            tooltip: "",
            persisted: true,
            primary: false,
            hidden_when: None,
        };
        let conditional = UiParam {
            hidden_when: Some(&FOLLOWS_TARGET),
            ..always_shown
        };

        assert!(!always_shown.is_hidden_in(&[&FOLLOWS_TARGET]));
        assert!(!conditional.is_hidden_in(&[]));
        assert!(!conditional.is_hidden_in(&[&OTHER]));
        assert!(conditional.is_hidden_in(&[&OTHER, &FOLLOWS_TARGET]));
    }

    #[test]
    fn test_color_component_names_follow_rgb_suffixes() {
        let tint = UiParam {
            name: "tint",
            path: "tint",
            group: "",
            label: None,
            kind: UiKind::Color,
            min: 0.0,
            max: 1.0,
            format: "",
            tooltip: "",
            persisted: true,
            primary: false,
            hidden_when: None,
        };
        assert_eq!(tint.color_component_names(), ["tint_r", "tint_g", "tint_b"]);
    }
}
