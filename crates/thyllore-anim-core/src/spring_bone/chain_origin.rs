use super::SubmeshSelector;

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ChainRole {
    Hair,
    #[serde(alias = "skirt")]
    Garment,
}

impl ChainRole {
    pub fn bone_prefix(self) -> &'static str {
        match self {
            ChainRole::Hair => "hair",
            ChainRole::Garment => "garment",
        }
    }
}

/// Where a spring chain came from: hand-authored, or generated on a submesh by the chain generator.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ChainOrigin {
    #[default]
    Authored,
    Generated {
        target: SubmeshSelector,
        role: ChainRole,
    },
}

impl ChainOrigin {
    pub fn is_generated_on(&self, target: &SubmeshSelector) -> bool {
        match self {
            ChainOrigin::Authored => false,
            ChainOrigin::Generated { target: own, .. } => own == target,
        }
    }
}
