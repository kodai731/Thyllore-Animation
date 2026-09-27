use std::collections::{BTreeMap, HashSet};

use crate::humanoid::components::chain::HUMANOID_CHAINS;
use crate::humanoid::components::mapping::{HumanoidMapping, UnresolvedRole};
use crate::humanoid::components::naming::HumanoidNamingRules;
use crate::humanoid::components::role::{HumanoidRole, REQUIRED};
use crate::humanoid::components::skeleton_input::BoneInput;
use crate::humanoid::components::tokens::BoneNameTokens;

use super::hierarchy::{is_ancestor, iter_ancestors};
use super::tokenize::tokenize_bone_name;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct MatchCandidate {
    pattern_rank: usize,
    extra_token_count: usize,
    bone_index: usize,
    role: HumanoidRole,
}

pub fn infer_mapping(
    bones: &[BoneInput],
    rules: &HumanoidNamingRules,
) -> (HumanoidMapping, Vec<UnresolvedRole>) {
    let (mut mapping, _) = infer_mapping_by_name(bones, rules);
    for chain in HUMANOID_CHAINS {
        drop_roles_off_chain(&mut mapping, bones, chain);
    }
    for chain in HUMANOID_CHAINS {
        fill_single_bone_gaps(&mut mapping, bones, chain);
    }

    let unresolved = collect_unresolved_roles(&mapping);
    (mapping, unresolved)
}

pub fn infer_mapping_by_name(
    bones: &[BoneInput],
    rules: &HumanoidNamingRules,
) -> (HumanoidMapping, Vec<UnresolvedRole>) {
    let bone_tokens: Vec<BoneNameTokens> = bones
        .iter()
        .map(|bone| tokenize_bone_name(&bone.name))
        .collect();
    let mut candidates = collect_match_candidates(&bone_tokens, rules);
    candidates.sort();

    let mut by_role: BTreeMap<HumanoidRole, usize> = BTreeMap::new();
    let mut used_bones: HashSet<usize> = HashSet::new();
    for candidate in candidates {
        if by_role.contains_key(&candidate.role) || used_bones.contains(&candidate.bone_index) {
            continue;
        }
        by_role.insert(candidate.role, candidate.bone_index);
        used_bones.insert(candidate.bone_index);
    }

    let mapping = HumanoidMapping { by_role };
    let unresolved = collect_unresolved_roles(&mapping);
    (mapping, unresolved)
}

pub fn collect_unresolved_roles(mapping: &HumanoidMapping) -> Vec<UnresolvedRole> {
    HumanoidRole::ALL
        .iter()
        .filter(|role| !mapping.by_role.contains_key(role))
        .map(|role| UnresolvedRole { role: *role })
        .collect()
}

fn drop_roles_off_chain(
    mapping: &mut HumanoidMapping,
    bones: &[BoneInput],
    chain: &[HumanoidRole],
) {
    let mut previous_bone: Option<usize> = None;
    for role in chain {
        let Some(&bone_index) = mapping.by_role.get(role) else {
            continue;
        };
        match previous_bone {
            Some(ancestor_index) if !is_ancestor(bones, ancestor_index, bone_index) => {
                mapping.by_role.remove(role);
            }
            _ => previous_bone = Some(bone_index),
        }
    }
}

fn fill_single_bone_gaps(
    mapping: &mut HumanoidMapping,
    bones: &[BoneInput],
    chain: &[HumanoidRole],
) {
    for window in chain.windows(3) {
        let (before_role, gap_role, after_role) = (window[0], window[1], window[2]);
        if !REQUIRED.contains(&gap_role) || mapping.by_role.contains_key(&gap_role) {
            continue;
        }
        let (Some(&before_index), Some(&after_index)) = (
            mapping.by_role.get(&before_role),
            mapping.by_role.get(&after_role),
        ) else {
            continue;
        };
        let Some(between_index) = find_single_bone_between(bones, before_index, after_index) else {
            continue;
        };
        if mapping
            .by_role
            .values()
            .any(|&index| index == between_index)
        {
            continue;
        }
        mapping.by_role.insert(gap_role, between_index);
    }
}

fn find_single_bone_between(
    bones: &[BoneInput],
    ancestor_index: usize,
    descendant_index: usize,
) -> Option<usize> {
    let ancestors: Vec<usize> = iter_ancestors(bones, descendant_index).collect();
    match ancestors.iter().position(|&index| index == ancestor_index) {
        Some(1) => Some(ancestors[0]),
        _ => None,
    }
}

fn collect_match_candidates(
    bone_tokens: &[BoneNameTokens],
    rules: &HumanoidNamingRules,
) -> Vec<MatchCandidate> {
    let mut candidates = Vec::new();
    for role in HumanoidRole::ALL {
        let Some(entry) = rules
            .role
            .iter()
            .find(|entry| entry.part == role_to_part(role))
        else {
            continue;
        };

        for (bone_index, tokens) in bone_tokens.iter().enumerate() {
            if tokens.side != role.side() {
                continue;
            }
            let Some(pattern_rank) = entry
                .patterns
                .iter()
                .position(|pattern| pattern_matches(&tokens.tokens, pattern))
            else {
                continue;
            };
            candidates.push(MatchCandidate {
                pattern_rank,
                extra_token_count: tokens
                    .tokens
                    .len()
                    .saturating_sub(entry.patterns[pattern_rank].len()),
                bone_index,
                role,
            });
        }
    }
    candidates
}

fn pattern_matches(bone_tokens: &[String], pattern: &[String]) -> bool {
    if pattern.len() == 1 {
        let joined = bone_tokens.join("");
        if joined == pattern[0] {
            return true;
        }
    }

    let mut pattern_pos = 0;
    for token in bone_tokens {
        if pattern_pos < pattern.len() && token == &pattern[pattern_pos] {
            pattern_pos += 1;
        }
    }

    pattern_pos == pattern.len()
}

fn role_to_part(role: HumanoidRole) -> &'static str {
    match role {
        HumanoidRole::Hips => "Hips",
        HumanoidRole::Spine => "Spine",
        HumanoidRole::Chest => "Chest",
        HumanoidRole::UpperChest => "UpperChest",
        HumanoidRole::Neck => "Neck",
        HumanoidRole::Head => "Head",
        HumanoidRole::Jaw => "Jaw",
        HumanoidRole::LeftShoulder | HumanoidRole::RightShoulder => "Shoulder",
        HumanoidRole::LeftUpperArm | HumanoidRole::RightUpperArm => "UpperArm",
        HumanoidRole::LeftLowerArm | HumanoidRole::RightLowerArm => "LowerArm",
        HumanoidRole::LeftHand | HumanoidRole::RightHand => "Hand",
        HumanoidRole::LeftUpperLeg | HumanoidRole::RightUpperLeg => "UpperLeg",
        HumanoidRole::LeftLowerLeg | HumanoidRole::RightLowerLeg => "LowerLeg",
        HumanoidRole::LeftFoot | HumanoidRole::RightFoot => "Foot",
        HumanoidRole::LeftToes | HumanoidRole::RightToes => "Toes",
        HumanoidRole::LeftEye | HumanoidRole::RightEye => "Eye",
        HumanoidRole::LeftThumbProximal | HumanoidRole::RightThumbProximal => "ThumbProximal",
        HumanoidRole::LeftThumbIntermediate | HumanoidRole::RightThumbIntermediate => {
            "ThumbIntermediate"
        }
        HumanoidRole::LeftThumbDistal | HumanoidRole::RightThumbDistal => "ThumbDistal",
        HumanoidRole::LeftIndexProximal | HumanoidRole::RightIndexProximal => "IndexProximal",
        HumanoidRole::LeftIndexIntermediate | HumanoidRole::RightIndexIntermediate => {
            "IndexIntermediate"
        }
        HumanoidRole::LeftIndexDistal | HumanoidRole::RightIndexDistal => "IndexDistal",
        HumanoidRole::LeftMiddleProximal | HumanoidRole::RightMiddleProximal => "MiddleProximal",
        HumanoidRole::LeftMiddleIntermediate | HumanoidRole::RightMiddleIntermediate => {
            "MiddleIntermediate"
        }
        HumanoidRole::LeftMiddleDistal | HumanoidRole::RightMiddleDistal => "MiddleDistal",
        HumanoidRole::LeftRingProximal | HumanoidRole::RightRingProximal => "RingProximal",
        HumanoidRole::LeftRingIntermediate | HumanoidRole::RightRingIntermediate => {
            "RingIntermediate"
        }
        HumanoidRole::LeftRingDistal | HumanoidRole::RightRingDistal => "RingDistal",
        HumanoidRole::LeftLittleProximal | HumanoidRole::RightLittleProximal => "LittleProximal",
        HumanoidRole::LeftLittleIntermediate | HumanoidRole::RightLittleIntermediate => {
            "LittleIntermediate"
        }
        HumanoidRole::LeftLittleDistal | HumanoidRole::RightLittleDistal => "LittleDistal",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::humanoid::components::role::REQUIRED;

    fn make_bone(name: &str) -> BoneInput {
        BoneInput {
            name: name.to_string(),
            parent: None,
            rest_position: [0.0, 0.0, 0.0],
        }
    }

    fn blender_style_skeleton() -> Vec<BoneInput> {
        vec![
            make_bone("Hips"),
            make_bone("Spine"),
            make_bone("Chest"),
            make_bone("Neck"),
            make_bone("Head"),
            make_bone("Shoulder.L"),
            make_bone("Upper_arm.L"),
            make_bone("Lower_arm.L"),
            make_bone("Hand.L"),
            make_bone("Upper_leg.L"),
            make_bone("Lower_leg.L"),
            make_bone("Foot.L"),
            make_bone("Toe.L"),
            make_bone("Shoulder.R"),
            make_bone("Upper_arm.R"),
            make_bone("Lower_arm.R"),
            make_bone("Hand.R"),
            make_bone("Upper_leg.R"),
            make_bone("Lower_leg.R"),
            make_bone("Foot.R"),
            make_bone("Toe.R"),
            make_bone("LeftEye"),
            make_bone("RightEye"),
        ]
    }

    fn unity_style_skeleton() -> Vec<BoneInput> {
        vec![
            make_bone("Hips"),
            make_bone("Spine"),
            make_bone("Chest"),
            make_bone("Neck"),
            make_bone("Head"),
            make_bone("LeftShoulder"),
            make_bone("LeftUpperArm"),
            make_bone("LeftLowerArm"),
            make_bone("LeftHand"),
            make_bone("LeftUpperLeg"),
            make_bone("LeftLowerLeg"),
            make_bone("LeftFoot"),
            make_bone("LeftToes"),
            make_bone("RightShoulder"),
            make_bone("RightUpperArm"),
            make_bone("RightLowerArm"),
            make_bone("RightHand"),
            make_bone("RightUpperLeg"),
            make_bone("RightLowerLeg"),
            make_bone("RightFoot"),
            make_bone("RightToes"),
            make_bone("LeftEye"),
            make_bone("RightEye"),
        ]
    }

    #[test]
    fn test_blender_style_resolves_required() {
        let bones = blender_style_skeleton();
        let rules = HumanoidNamingRules::default();
        let (mapping, unresolved) = infer_mapping_by_name(&bones, &rules);

        for role in REQUIRED {
            assert!(
                mapping.by_role.contains_key(&role),
                "Blender style: required role {:?} not resolved",
                role
            );
        }

        let unresolved_required: Vec<_> = unresolved
            .iter()
            .filter(|u| REQUIRED.contains(&u.role))
            .collect();
        assert!(
            unresolved_required.is_empty(),
            "Blender style: unresolved required roles: {:?}",
            unresolved_required
        );
    }

    #[test]
    fn test_unity_style_resolves_required() {
        let bones = unity_style_skeleton();
        let rules = HumanoidNamingRules::default();
        let (mapping, unresolved) = infer_mapping_by_name(&bones, &rules);

        for role in REQUIRED {
            assert!(
                mapping.by_role.contains_key(&role),
                "Unity style: required role {:?} not resolved",
                role
            );
        }

        let unresolved_required: Vec<_> = unresolved
            .iter()
            .filter(|u| REQUIRED.contains(&u.role))
            .collect();
        assert!(
            unresolved_required.is_empty(),
            "Unity style: unresolved required roles: {:?}",
            unresolved_required
        );
    }

    #[test]
    fn test_no_duplicate_assignments() {
        let bones = blender_style_skeleton();
        let rules = HumanoidNamingRules::default();
        let (mapping, _) = infer_mapping_by_name(&bones, &rules);

        let mut indices: Vec<_> = mapping.by_role.values().copied().collect();
        indices.sort();
        indices.dedup();
        assert_eq!(
            indices.len(),
            mapping.by_role.len(),
            "Duplicate bone assignments found"
        );
    }

    fn make_child_bone(name: &str, parent: usize) -> BoneInput {
        BoneInput {
            name: name.to_string(),
            parent: Some(parent),
            rest_position: [0.0, 0.0, 0.0],
        }
    }

    fn left_arm_skeleton(lower_arm_name: &str, hand_parent: usize) -> Vec<BoneInput> {
        vec![
            make_bone("Hips"),
            make_child_bone("Spine", 0),
            make_child_bone("LeftShoulder", 1),
            make_child_bone("LeftUpperArm", 2),
            make_child_bone(lower_arm_name, 3),
            make_child_bone("LeftHand", hand_parent),
        ]
    }

    #[test]
    fn test_infer_mapping_drops_role_outside_parent_chain() {
        let mut bones = left_arm_skeleton("LeftLowerArm", 4);
        bones.push(make_bone("Head"));
        let rules = HumanoidNamingRules::default();
        let (mapping, _) = infer_mapping(&bones, &rules);

        assert_eq!(mapping.by_role.get(&HumanoidRole::Spine), Some(&1));
        assert_eq!(mapping.by_role.get(&HumanoidRole::LeftHand), Some(&5));
        assert!(!mapping.by_role.contains_key(&HumanoidRole::Head));
    }

    #[test]
    fn test_infer_mapping_fills_single_bone_gap() {
        let bones = left_arm_skeleton("Segment001", 4);
        let rules = HumanoidNamingRules::default();
        let (mapping, unresolved) = infer_mapping(&bones, &rules);

        assert_eq!(mapping.by_role.get(&HumanoidRole::LeftLowerArm), Some(&4));
        assert!(unresolved
            .iter()
            .all(|role| role.role != HumanoidRole::LeftLowerArm));
    }

    #[test]
    fn test_infer_mapping_leaves_gap_when_several_bones_between() {
        let mut bones = left_arm_skeleton("Segment001", 6);
        bones.push(make_child_bone("Segment002", 4));
        let rules = HumanoidNamingRules::default();
        let (mapping, unresolved) = infer_mapping(&bones, &rules);

        assert!(!mapping.by_role.contains_key(&HumanoidRole::LeftLowerArm));
        assert!(unresolved
            .iter()
            .any(|role| role.role == HumanoidRole::LeftLowerArm));
    }
}
