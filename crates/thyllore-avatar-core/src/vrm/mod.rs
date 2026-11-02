mod extensions;
mod force;
mod humanoid;
mod meta;
mod spring_bone;

#[cfg(test)]
mod tests;

pub use extensions::{build_vrm_extensions, VrmExtensions, VrmInput, THYLLORE_AVATAR};
pub use force::{vrm_magnitudes, VrmJointMagnitudes};
pub use humanoid::{build_vrmc_vrm, build_vrmc_vrm_humanoid, VRMC_VRM};
pub use meta::{
    AvatarPermission, CommercialUsage, CreditNotation, Modification, VrmMeta, VRM_1_0_LICENSE_URL,
};
pub use spring_bone::{build_vrmc_spring_bone, SpringBoneInput, VRMC_SPRING_BONE};
