use thyllore_avatar_core::humanoid::components::geometry_warning::GeometryWarning;
use thyllore_avatar_core::humanoid::components::mapping::{HumanoidMapping, UnresolvedRole};
use thyllore_avatar_core::humanoid::components::mapping_issues::MappingIssue;
use thyllore_avatar_core::humanoid::components::rest_pose::RestPose;
use thyllore_avatar_core::humanoid::components::skeleton_input::BoneInput;
use thyllore_avatar_core::stats::components::stats::AvatarStats;
use thyllore_avatar_core::vrchat::rank::{Platform, RankReport};

#[derive(Clone, Debug, Default)]
pub struct AvatarSetupState {
    pub is_open: bool,
    pub source_model_path: String,
    pub bones: Vec<BoneInput>,
    pub mapping: HumanoidMapping,
    pub unresolved: Vec<UnresolvedRole>,
    pub issues: Vec<MappingIssue>,
    pub geometry_warnings: Vec<GeometryWarning>,
    pub rest_pose: RestPose,
    pub missing_bone_names: Vec<String>,
    pub stats: AvatarStats,
    pub platform: Platform,
    pub rank: Option<RankReport>,
    pub spring_prefix: String,
}
