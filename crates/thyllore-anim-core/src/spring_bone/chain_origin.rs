use super::SubmeshSelector;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RootAttachment {
    /// The root sits on the top of a closed surface, such as the head.
    Cap,
    /// The root is a ring at the top edge of the mesh, such as a skirt or a sleeve.
    Ring,
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
