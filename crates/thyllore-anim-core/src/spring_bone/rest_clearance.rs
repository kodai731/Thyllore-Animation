use thyllore_math_core::lerp;

use super::setup::{SpringChainId, SpringColliderId};
use super::MIN_REST_CLEARANCE;
use crate::BoneId;

/// A collider shrunk so a joint resting inside it keeps its rest penetration without a tolerance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColliderShrink {
    pub collider_id: SpringColliderId,
    pub chain_id: SpringChainId,
    pub bone_id: BoneId,
    pub from: f32,
    pub to: f32,
}

/// One joint's rest geometry: its hit radius, its rest distance to every chain collider and the
/// root-to-tip tolerance weight, all in the solver's units.
#[derive(Clone, Debug, PartialEq)]
pub struct JointRestGeometry {
    pub bone_id: BoneId,
    pub hit_radius: f32,
    pub rest_axis_distances: Vec<f32>,
    pub tolerance_weight: f32,
}

/// The hit radius per joint and the collider shrinks that give a chain without rest tolerance the same
/// minimum distance per joint-collider pair as the tolerance would.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChainClearance {
    pub hit_radii: Vec<f32>,
    pub shrinks: Vec<ColliderShrink>,
}

/// Projects the rest tolerance onto radii: a pair whose rest distance is inside the full radius keeps
/// `lerp(full, rest, weight)` as its minimum, first through the joint's hit radius and, when that
/// would go below zero, through the collider's radius.
pub fn project_chain_clearance(
    chain_id: SpringChainId,
    joints: &[JointRestGeometry],
    colliders: &[(SpringColliderId, f32)],
) -> ChainClearance {
    let mut clearance = ChainClearance::default();
    for joint in joints {
        let penetrating =
            colliders
                .iter()
                .zip(&joint.rest_axis_distances)
                .filter(|(&(_, radius), &rest)| {
                    rest >= MIN_REST_CLEARANCE && rest < joint.hit_radius + radius
                });

        let mut hit_radius = joint.hit_radius;
        for (&(collider_id, radius), &rest) in penetrating {
            let weight = joint.tolerance_weight.clamp(0.0, 1.0);
            let pair_hit_radius = lerp(joint.hit_radius, rest - radius, weight);
            hit_radius = hit_radius.min(pair_hit_radius.max(0.0));
            if pair_hit_radius < 0.0 {
                let shrink = ColliderShrink {
                    collider_id,
                    chain_id,
                    bone_id: joint.bone_id,
                    from: radius,
                    to: lerp(joint.hit_radius + radius, rest, weight),
                };
                merge_collider_shrink(&mut clearance.shrinks, shrink);
            }
        }
        clearance.hit_radii.push(hit_radius);
    }
    clearance
}

/// Keeps one shrink per collider: the smallest radius wins, with the joint that asked for it.
pub fn merge_collider_shrink(shrinks: &mut Vec<ColliderShrink>, shrink: ColliderShrink) {
    match shrinks
        .iter_mut()
        .find(|s| s.collider_id == shrink.collider_id)
    {
        Some(existing) if existing.to <= shrink.to => {}
        Some(existing) => *existing = shrink,
        None => shrinks.push(shrink),
    }
}

/// The radius a collider is simulated and exported with: its smallest shrink, else the authored one.
pub fn shrunk_collider_radius(
    shrinks: &[ColliderShrink],
    collider_id: SpringColliderId,
    authored: f32,
) -> f32 {
    shrinks
        .iter()
        .filter(|s| s.collider_id == collider_id)
        .map(|s| s.to)
        .fold(authored, f32::min)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn joint(hit_radius: f32, rests: &[f32], weight: f32) -> JointRestGeometry {
        JointRestGeometry {
            bone_id: 7,
            hit_radius,
            rest_axis_distances: rests.to_vec(),
            tolerance_weight: weight,
        }
    }

    #[test]
    fn a_clear_joint_keeps_its_hit_radius() {
        let clearance = project_chain_clearance(0, &[joint(0.1, &[0.5], 1.0)], &[(3, 0.2)]);
        assert_eq!(clearance.hit_radii, vec![0.1]);
        assert!(clearance.shrinks.is_empty());
    }

    #[test]
    fn a_root_joint_keeps_its_whole_rest_penetration_through_its_hit_radius() {
        let clearance = project_chain_clearance(0, &[joint(0.1, &[0.25], 1.0)], &[(3, 0.2)]);
        assert!(
            (clearance.hit_radii[0] - 0.05).abs() < 1e-6,
            "{clearance:?}"
        );
        assert!(clearance.shrinks.is_empty());
    }

    #[test]
    fn a_tip_joint_is_pushed_fully_out() {
        let clearance = project_chain_clearance(0, &[joint(0.1, &[0.25], 0.0)], &[(3, 0.2)]);
        assert_eq!(clearance.hit_radii, vec![0.1]);
    }

    #[test]
    fn a_mid_joint_interpolates_the_tolerated_penetration() {
        let clearance = project_chain_clearance(0, &[joint(0.1, &[0.25], 0.5)], &[(3, 0.2)]);
        assert!(
            (clearance.hit_radii[0] - 0.075).abs() < 1e-6,
            "{clearance:?}"
        );
    }

    #[test]
    fn two_colliders_take_the_smaller_hit_radius() {
        let clearance =
            project_chain_clearance(0, &[joint(0.1, &[0.25, 0.22], 1.0)], &[(3, 0.2), (4, 0.2)]);
        assert!(
            (clearance.hit_radii[0] - 0.02).abs() < 1e-6,
            "{clearance:?}"
        );
    }

    #[test]
    fn a_joint_inside_the_collider_shrinks_the_collider_instead() {
        let clearance = project_chain_clearance(5, &[joint(0.1, &[0.15], 1.0)], &[(3, 0.2)]);
        assert_eq!(clearance.hit_radii, vec![0.0]);
        assert_eq!(
            clearance.shrinks,
            vec![ColliderShrink {
                collider_id: 3,
                chain_id: 5,
                bone_id: 7,
                from: 0.2,
                to: 0.15,
            }]
        );
        assert!((shrunk_collider_radius(&clearance.shrinks, 3, 0.2) - 0.15).abs() < 1e-6);
        assert_eq!(shrunk_collider_radius(&clearance.shrinks, 4, 0.2), 0.2);
    }

    #[test]
    fn the_deepest_joint_decides_the_shrink() {
        let clearance = project_chain_clearance(
            0,
            &[joint(0.1, &[0.15], 1.0), joint(0.1, &[0.12], 1.0)],
            &[(3, 0.2)],
        );
        assert_eq!(clearance.shrinks.len(), 1);
        assert!(
            (clearance.shrinks[0].to - 0.12).abs() < 1e-6,
            "{clearance:?}"
        );
    }

    #[test]
    fn a_joint_on_the_collider_axis_is_pushed_out_like_the_tolerance_does() {
        let clearance = project_chain_clearance(0, &[joint(0.1, &[0.0], 1.0)], &[(3, 0.2)]);
        assert_eq!(clearance.hit_radii, vec![0.1]);
        assert!(clearance.shrinks.is_empty());
    }
}
