use std::fmt;

use super::role::HumanoidRole;

#[derive(Clone, Debug)]
pub enum MappingIssue {
    MissingRequired(HumanoidRole),
    HierarchyOrder {
        child: HumanoidRole,
        expected_ancestor: HumanoidRole,
    },
    MirroredRolesShareBone {
        left: HumanoidRole,
        right: HumanoidRole,
    },
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
