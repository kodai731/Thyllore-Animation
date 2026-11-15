pub(crate) mod canonical;
pub(crate) mod components;
pub(crate) mod systems;

pub use canonical::skeleton::{skeleton, CanonicalBone, CanonicalTip};
pub use components::avatar_rig::AvatarRig;
pub use components::body_collider_roles::body_collider_bone_indices;
pub use components::geometry_warning::GeometryWarning;
pub use components::humanoid_frame::HumanoidFrame;
pub use components::mapping::{HumanoidMapping, StoredHumanoidMapping, StoredRig, UnresolvedRole};
pub use components::mapping_issues::MappingIssue;
pub use components::rest_pose::RestPose;
pub use components::role::*;
pub use components::skeleton_input::BoneInput;
pub use components::spring_chain::StructuralChain;
pub use components::tokens::BoneNameTokens;
pub use components::vrm_version::VrmVersion;
pub use systems::geometry_checks::check_mapping_geometry;
pub use systems::hierarchy::{is_ancestor, iter_ancestors};
pub use systems::humanoid_frame::derive_humanoid_frame;
pub use systems::mapping_io::{
    humanoid_mapping_path, load_mapping, load_or_infer_rig, save_generic_rig, save_mapping,
};
pub use systems::name_match::{collect_unresolved_roles, infer_mapping, infer_mapping_by_name};
pub use systems::pose::{detect_rest_pose, A_POSE_ANGLE_THRESHOLD, T_POSE_ANGLE_THRESHOLD};
pub use systems::structural_chains::find_chains_under;
pub use systems::tokenize::tokenize_bone_name;
pub use systems::validate::validate_mapping;
