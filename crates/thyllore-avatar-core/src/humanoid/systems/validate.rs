use std::collections::BTreeMap;

use crate::expression::components::side::Side;
use crate::humanoid::components::mapping::HumanoidMapping;
use crate::humanoid::components::mapping_issues::MappingIssue;
use crate::humanoid::components::role::{HumanoidRole, HUMANOID_CHAINS, REQUIRED};
use crate::humanoid::components::skeleton_input::BoneInput;

use super::hierarchy::is_ancestor;

pub fn validate_mapping(mapping: &HumanoidMapping, bones: &[BoneInput]) -> Vec<MappingIssue> {
    let mut issues = Vec::new();
    check_missing_required(mapping, &mut issues);
    check_hierarchy_order(mapping, bones, &mut issues);
    check_mirrored_roles_share_bone(mapping, &mut issues);
    check_bone_in_two_roles(mapping, &mut issues);
    issues
}

fn check_missing_required(mapping: &HumanoidMapping, issues: &mut Vec<MappingIssue>) {
    for role in REQUIRED {
        if !mapping.by_role.contains_key(&role) {
            issues.push(MappingIssue::MissingRequired(role));
        }
    }
}

fn check_hierarchy_order(
    mapping: &HumanoidMapping,
    bones: &[BoneInput],
    issues: &mut Vec<MappingIssue>,
) {
    for chain in HUMANOID_CHAINS {
        let mut previous: Option<(HumanoidRole, usize)> = None;
        for &role in chain {
            let Some(&bone_index) = mapping.by_role.get(&role) else {
                continue;
            };
            if let Some((ancestor_role, ancestor_index)) = previous {
                if !is_ancestor(bones, ancestor_index, bone_index) {
                    issues.push(MappingIssue::HierarchyOrder {
                        child: role,
                        expected_ancestor: ancestor_role,
                    });
                }
            }
            previous = Some((role, bone_index));
        }
    }
}

fn check_mirrored_roles_share_bone(mapping: &HumanoidMapping, issues: &mut Vec<MappingIssue>) {
    for role in HumanoidRole::ALL
        .iter()
        .copied()
        .filter(|role| role.side() == Some(Side::Left))
    {
        let right_role = role.mirrored();
        if let (Some(&left_bone), Some(&right_bone)) =
            (mapping.by_role.get(&role), mapping.by_role.get(&right_role))
        {
            if left_bone == right_bone {
                issues.push(MappingIssue::MirroredRolesShareBone {
                    left: role,
                    right: right_role,
                });
            }
        }
    }
}

fn check_bone_in_two_roles(mapping: &HumanoidMapping, issues: &mut Vec<MappingIssue>) {
    let mut bone_to_roles: BTreeMap<usize, Vec<HumanoidRole>> = BTreeMap::new();
    for (&role, &bone_index) in &mapping.by_role {
        bone_to_roles.entry(bone_index).or_default().push(role);
    }
    for (bone_index, roles) in &bone_to_roles {
        if roles.len() != 2 {
            continue;
        }
        let [a, b] = roles.as_slice() else {
            continue;
        };
        if a.mirrored() == *b || b.mirrored() == *a {
            continue;
        }
        let mut sorted = roles.clone();
        sorted.sort_by_key(|r| r.index());
        issues.push(MappingIssue::BoneInTwoRoles {
            bone_index: *bone_index,
            roles: [sorted[0], sorted[1]],
        });
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn bone(name: &str, parent: Option<usize>, pos: [f32; 3]) -> BoneInput {
        BoneInput {
            name: name.to_string(),
            parent,
            rest_position: pos,
        }
    }

    fn make_mapping(pairs: &[(HumanoidRole, usize)]) -> HumanoidMapping {
        let mut by_role = BTreeMap::new();
        for (role, idx) in pairs {
            by_role.insert(*role, *idx);
        }
        HumanoidMapping { by_role }
    }

    #[test]
    fn test_missing_required() {
        let bones: Vec<BoneInput> = vec![
            bone("Hips", None, [0.0, 1.0, 0.0]),
            bone("Spine", Some(0), [0.0, 2.0, 0.0]),
        ];
        let mapping = make_mapping(&[(HumanoidRole::Hips, 0), (HumanoidRole::Spine, 1)]);
        let issues = validate_mapping(&mapping, &bones);

        let missing: Vec<_> = issues
            .iter()
            .filter_map(|i| {
                if let MappingIssue::MissingRequired(r) = i {
                    Some(*r)
                } else {
                    None
                }
            })
            .collect();
        assert!(missing.contains(&HumanoidRole::Head));
        assert!(missing.contains(&HumanoidRole::LeftUpperArm));
    }

    #[test]
    fn test_hierarchy_order_violation() {
        let bones: Vec<BoneInput> = vec![
            bone("Hips", None, [0.0, 1.0, 0.0]),
            bone("Spine", Some(0), [0.0, 2.0, 0.0]),
            bone("Head", None, [0.0, 3.0, 0.0]),
        ];
        let mapping = make_mapping(&[
            (HumanoidRole::Hips, 0),
            (HumanoidRole::Spine, 1),
            (HumanoidRole::Head, 2),
        ]);
        let issues = validate_mapping(&mapping, &bones);

        let hierarchy: Vec<_> = issues
            .iter()
            .filter_map(|i| {
                if let MappingIssue::HierarchyOrder { child, .. } = i {
                    Some(*child)
                } else {
                    None
                }
            })
            .collect();
        assert!(hierarchy.contains(&HumanoidRole::Head));
    }

    #[test]
    fn test_valid_mapping_has_no_issues() {
        let bones: Vec<BoneInput> = vec![
            bone("Hips", None, [0.0, 1.0, 0.0]),
            bone("Spine", Some(0), [0.0, 2.0, 0.0]),
            bone("Head", Some(1), [0.0, 3.0, 0.0]),
        ];
        let mapping = make_mapping(&[
            (HumanoidRole::Hips, 0),
            (HumanoidRole::Spine, 1),
            (HumanoidRole::Head, 2),
        ]);
        let issues = validate_mapping(&mapping, &bones);

        let hierarchy: Vec<_> = issues
            .iter()
            .filter_map(|i| {
                if let MappingIssue::HierarchyOrder { child, .. } = i {
                    Some(*child)
                } else {
                    None
                }
            })
            .collect();
        assert!(hierarchy.is_empty());
    }
}
