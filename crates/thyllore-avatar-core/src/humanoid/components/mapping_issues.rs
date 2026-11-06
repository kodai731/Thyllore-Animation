use std::fmt;

use super::role::HumanoidRole;

/// A reason `validate_mapping` rejects a humanoid mapping; a mapping with no issue is exportable as is.
#[derive(Clone, Debug)]
pub enum MappingIssue {
    /// A role marked `required` in `humanoid_rig.toml` has no bone.
    MissingRequired(HumanoidRole),
    /// `child` is mapped but its bone is not a descendant of the bone mapped to `expected_ancestor`.
    HierarchyOrder {
        child: HumanoidRole,
        expected_ancestor: HumanoidRole,
    },
    /// A left role and its mirror are mapped to the same bone.
    MirroredRolesShareBone {
        left: HumanoidRole,
        right: HumanoidRole,
    },
    /// One bone is mapped to two roles that are not mirrors of each other.
    BoneInTwoRoles {
        bone_index: usize,
        roles: [HumanoidRole; 2],
    },
}

impl fmt::Display for MappingIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MappingIssue::MissingRequired(role) => {
                write!(f, "Missing required role {}", role.unity_name())
            }
            MappingIssue::HierarchyOrder {
                child,
                expected_ancestor,
            } => write!(
                f,
                "{} is not under {}",
                child.unity_name(),
                expected_ancestor.unity_name()
            ),
            MappingIssue::MirroredRolesShareBone { left, right } => write!(
                f,
                "{} and {} share the same bone",
                left.unity_name(),
                right.unity_name()
            ),
            MappingIssue::BoneInTwoRoles { roles, .. } => write!(
                f,
                "bone used by both {} and {}",
                roles[0].unity_name(),
                roles[1].unity_name()
            ),
        }
    }
}
