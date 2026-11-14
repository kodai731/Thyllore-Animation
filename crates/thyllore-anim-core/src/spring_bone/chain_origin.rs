use super::SubmeshSelector;

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, strum::VariantArray,
)]
#[serde(rename_all = "snake_case")]
pub enum RootAttachment {
    Cap,
    Ring,
}

impl RootAttachment {
    pub fn label(self) -> &'static str {
        match self {
            RootAttachment::Cap => "Cap",
            RootAttachment::Ring => "Ring",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            RootAttachment::Cap => "Root on the top of a closed surface, such as the head",
            RootAttachment::Ring => "Root ring at the mesh top edge, such as a skirt or sleeve",
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
        attachment: RootAttachment,
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
