use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ExpressionPreset {
    pub name: String,
    pub weights: BTreeMap<String, f32>,
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ExpressionLibrary {
    pub presets: Vec<ExpressionPreset>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PresetApplication {
    pub weights: Vec<f32>,
    pub unknown_channels: Vec<String>,
}
