const GROUPING_TOML: &str = include_str!("../../../data/expression_grouping.toml");

#[derive(Clone, Debug)]
pub struct ChannelGroup {
    pub name: String,
    pub channel_indices: Vec<usize>,
    pub collapsed: bool,
    pub exclude_from_reset: bool,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct ExpressionGrouping {
    #[serde(rename = "group")]
    pub groups: Vec<ChannelGroupRule>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct ChannelGroupRule {
    pub name: String,
    pub prefixes: Vec<String>,
    #[serde(default)]
    pub collapsed: bool,
    #[serde(default)]
    pub exclude_from_reset: bool,
}

impl Default for ExpressionGrouping {
    fn default() -> Self {
        toml::from_str(GROUPING_TOML).expect("expression_grouping.toml is valid TOML")
    }
}
