mod end_node;
mod extensions;
mod force;
mod humanoid;
mod meta;
mod rest_clearance;
mod spring_bone;
mod t_pose;

#[cfg(test)]
mod tests;

pub use end_node::{spring_end_nodes, SpringEndNode};
pub use extensions::{build_vrm_extensions, VrmExtensions, VrmInput, THYLLORE_AVATAR};
pub use force::{vrm_magnitudes, VrmJointMagnitudes};
pub use humanoid::{build_vrmc_vrm, build_vrmc_vrm_humanoid, VRMC_VRM};
pub use meta::{
    vrm_meta_fields, AvatarPermission, CommercialUsage, CreditNotation, Modification, VrmMeta,
    VrmMetaField, VrmMetaFieldKind, VRM_1_0_LICENSE_URL, VRM_META_DEFAULT_VERSION,
};
pub use rest_clearance::{vrm_rest_clearance, VrmColliderShrink, VrmRestClearance};
pub use spring_bone::{build_vrmc_spring_bone, SpringBoneInput, VRMC_SPRING_BONE};
pub use t_pose::{t_pose_warnings, T_POSE_TOLERANCE_DEG};
