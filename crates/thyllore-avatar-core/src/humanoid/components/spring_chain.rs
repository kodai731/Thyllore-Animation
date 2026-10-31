use thyllore_model_core::BoneId;

/// A run of non-humanoid bones under a root: each joint is the only such child of the one before.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructuralChain {
    pub joints: Vec<BoneId>,
}
