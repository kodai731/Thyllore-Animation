use crate::humanoid::components::chain::HUMANOID_CHAINS;
use crate::humanoid::components::mapping::HumanoidMapping;
use crate::humanoid::components::mapping_issues::MappingIssue;
use crate::humanoid::components::role::{HumanoidRole, REQUIRED};
use crate::humanoid::components::skeleton_input::BoneInput;

use super::hierarchy::is_ancestor;

pub fn validate_mapping(mapping: &HumanoidMapping, bones: &[BoneInput]) -> Vec<MappingIssue> {
    let mut issues = Vec::new();
    check_missing_required(mapping, &mut issues);
    check_hierarchy_order(mapping, bones, &mut issues);
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
