use glam::{Mat3, Mat4, Vec3};
use thyllore_anim_core::spring_bone::FittedCapsule;
use thyllore_math_core::mean_axis_scale;
use thyllore_model_core::{BoneId, Skeleton};

use crate::cloth_setup::components::chain_gen::BodyCollider;
use crate::humanoid::components::mapping::HumanoidMapping;
use crate::humanoid::components::role::{HumanoidRole, HUMANOID_CHAINS};

/// A humanoid bone with its rest head in the same space as the colliders.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoleBone {
    pub role: HumanoidRole,
    pub bone: BoneId,
    pub head: Vec3,
}

/// The torso chain from the hips to the head, in order.
pub fn torso_roles() -> &'static [HumanoidRole] {
    HUMANOID_CHAINS
        .iter()
        .copied()
        .find(|chain| chain.contains(&HumanoidRole::Hips))
        .unwrap_or(&[])
}

fn mean_radius(colliders: &[BodyCollider], bones: &[BoneId]) -> Option<f32> {
    let radii: Vec<f32> = colliders
        .iter()
        .filter(|collider| bones.contains(&collider.bone))
        .map(|collider| collider.radius)
        .collect();
    if radii.is_empty() {
        return None;
    }
    Some(radii.iter().sum::<f32>() / radii.len() as f32)
}

/// The radius a torso bone's own skin gives its fallback capsule around the axis `head` → `tail`,
/// None when the bone drives too little skin.
pub type OwnRadius<'a> = &'a dyn Fn(BoneId, Vec3, Vec3) -> Option<f32>;

/// Capsules for the torso bones without a collider, so a rig whose torso skin was removed still has a
/// body there: the pelvis spans the upper legs' heads (when both legs are mapped) and the spine bones
/// reach the next torso bone. The radius comes from the bone's own skin, else the mean of its torso
/// neighbours' colliders, else the mean of the upper leg colliders.
pub fn torso_fallback_colliders(
    bones: &[RoleBone],
    existing: &[BodyCollider],
    own_radius: OwnRadius,
) -> Vec<BodyCollider> {
    let find = |role: HumanoidRole| bones.iter().find(|bone| bone.role == role).copied();
    let legs = find(HumanoidRole::LeftUpperLeg).zip(find(HumanoidRole::RightUpperLeg));
    let legs_radius =
        legs.and_then(|(left, right)| mean_radius(existing, &[left.bone, right.bone]));
    let torso: Vec<RoleBone> = torso_roles()
        .iter()
        .filter_map(|&role| find(role))
        .collect();
    let has_collider = |bone: BoneId| existing.iter().any(|collider| collider.bone == bone);
    let neighbour_radius = |index: usize| {
        let neighbours: Vec<BoneId> = [index.checked_sub(1), Some(index + 1)]
            .into_iter()
            .flatten()
            .filter_map(|neighbour| torso.get(neighbour))
            .map(|neighbour| neighbour.bone)
            .collect();
        mean_radius(existing, &neighbours)
    };

    let mut fallback = Vec::new();
    for (index, bone) in torso.iter().enumerate() {
        if has_collider(bone.bone) {
            continue;
        }
        let (head, tail) = match (bone.role, legs, torso.get(index + 1)) {
            (HumanoidRole::Hips, Some((left, right)), _) => (left.head, right.head),
            (HumanoidRole::Hips, None, _) => continue,
            (_, _, Some(next)) => (bone.head, next.head),
            (_, _, None) => continue,
        };
        let radius = own_radius(bone.bone, head, tail)
            .or_else(|| neighbour_radius(index))
            .or(legs_radius);
        let Some(radius) = radius else {
            continue;
        };
        fallback.push(BodyCollider {
            bone: bone.bone,
            head,
            tail,
            radius,
        });
    }
    fallback
}

struct BindFrame {
    to_model: Mat4,
    to_local: Mat4,
    scale: f32,
}

fn bind_frame(skeleton: &Skeleton, bone: BoneId) -> Option<BindFrame> {
    let to_local = skeleton.get_bone(bone)?.inverse_bind_pose;
    let to_model = to_local.try_inverse()?;
    let linear = Mat3::from_cols(
        to_model.x_axis.truncate(),
        to_model.y_axis.truncate(),
        to_model.z_axis.truncate(),
    );
    Some(BindFrame {
        to_model,
        to_local,
        scale: mean_axis_scale(&linear),
    })
}

fn place(matrix: &Mat4, point: Vec3) -> Vec3 {
    (matrix * point.extend(1.0)).truncate()
}

/// The fallback torso capsules as fitted capsules in their bones' bind space, for the torso bones the
/// skin fit left without a capsule.
pub fn fitted_torso_fallback(
    skeleton: &Skeleton,
    mapping: &HumanoidMapping,
    fitted: &[FittedCapsule],
    own_radius: OwnRadius,
) -> Vec<FittedCapsule> {
    let roles: Vec<RoleBone> = mapping
        .by_role
        .iter()
        .filter_map(|(&role, &bone)| {
            let frame = bind_frame(skeleton, bone as BoneId)?;
            Some(RoleBone {
                role,
                bone: bone as BoneId,
                head: frame.to_model.w_axis.truncate(),
            })
        })
        .collect();
    let existing: Vec<BodyCollider> = fitted
        .iter()
        .filter_map(|capsule| {
            let frame = bind_frame(skeleton, capsule.bone_id)?;
            Some(BodyCollider {
                bone: capsule.bone_id,
                head: place(&frame.to_model, capsule.head),
                tail: place(&frame.to_model, capsule.tail),
                radius: capsule.radius * frame.scale,
            })
        })
        .collect();

    torso_fallback_colliders(&roles, &existing, own_radius)
        .into_iter()
        .filter_map(|collider| {
            let frame = bind_frame(skeleton, collider.bone)?;
            Some(FittedCapsule {
                bone_id: collider.bone,
                head: place(&frame.to_local, collider.head),
                tail: place(&frame.to_local, collider.tail),
                radius: collider.radius / frame.scale,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use thyllore_model_core::Bone;

    fn role_bone(role: HumanoidRole, bone: BoneId, y: f32, x: f32) -> RoleBone {
        RoleBone {
            role,
            bone,
            head: Vec3::new(x, y, 0.0),
        }
    }

    fn legs_with_colliders() -> (Vec<RoleBone>, Vec<BodyCollider>) {
        let bones = vec![
            role_bone(HumanoidRole::Hips, 0, 1.0, 0.0),
            role_bone(HumanoidRole::Spine, 1, 1.1, 0.0),
            role_bone(HumanoidRole::Chest, 2, 1.3, 0.0),
            role_bone(HumanoidRole::Neck, 3, 1.5, 0.0),
            role_bone(HumanoidRole::LeftUpperLeg, 4, 0.95, 0.09),
            role_bone(HumanoidRole::RightUpperLeg, 5, 0.95, -0.09),
        ];
        let leg = |bone: BoneId, x: f32, radius: f32| BodyCollider {
            bone,
            head: Vec3::new(x, 0.95, 0.0),
            tail: Vec3::new(x, 0.5, 0.0),
            radius,
        };
        (bones, vec![leg(4, 0.09, 0.08), leg(5, -0.09, 0.10)])
    }

    #[test]
    fn pelvis_spans_the_leg_heads_and_spine_bones_reach_the_next_torso_bone() {
        let (bones, existing) = legs_with_colliders();

        let fallback = torso_fallback_colliders(&bones, &existing, &|_, _, _| None);

        assert_eq!(fallback.len(), 3);
        let pelvis = &fallback[0];
        assert_eq!(pelvis.bone, 0);
        assert_eq!(pelvis.head, Vec3::new(0.09, 0.95, 0.0));
        assert_eq!(pelvis.tail, Vec3::new(-0.09, 0.95, 0.0));
        assert!((pelvis.radius - 0.09).abs() < 1e-6);
        assert_eq!((fallback[1].bone, fallback[1].tail.y), (1, 1.3));
        assert_eq!((fallback[2].bone, fallback[2].tail.y), (2, 1.5));
    }

    #[test]
    fn torso_bones_with_a_collider_are_left_alone() {
        let (bones, mut existing) = legs_with_colliders();
        existing.push(BodyCollider {
            bone: 0,
            head: Vec3::new(0.0, 1.0, 0.0),
            tail: Vec3::new(0.0, 1.0, 0.0),
            radius: 0.2,
        });

        let fallback = torso_fallback_colliders(&bones, &existing, &|_, _, _| None);

        assert!(fallback.iter().all(|collider| collider.bone != 0));
        assert_eq!(fallback.len(), 2);
    }

    #[test]
    fn own_skin_radius_wins_over_the_neighbours_and_the_legs() {
        let (bones, existing) = legs_with_colliders();

        let fallback =
            torso_fallback_colliders(&bones, &existing, &|bone, _, _| (bone == 2).then_some(0.03));

        let chest = fallback.iter().find(|collider| collider.bone == 2).unwrap();
        assert!((chest.radius - 0.03).abs() < 1e-6);
        let spine = fallback.iter().find(|collider| collider.bone == 1).unwrap();
        assert!((spine.radius - 0.09).abs() < 1e-6);
    }

    #[test]
    fn a_bone_without_skin_takes_the_mean_radius_of_its_torso_neighbours() {
        let (bones, mut existing) = legs_with_colliders();
        existing.push(BodyCollider {
            bone: 1,
            head: Vec3::new(0.0, 1.1, 0.0),
            tail: Vec3::new(0.0, 1.3, 0.0),
            radius: 0.2,
        });

        let fallback = torso_fallback_colliders(&bones, &existing, &|_, _, _| None);

        let chest = fallback.iter().find(|collider| collider.bone == 2).unwrap();
        assert!((chest.radius - 0.2).abs() < 1e-6);
        let pelvis = fallback.iter().find(|collider| collider.bone == 0).unwrap();
        assert!((pelvis.radius - 0.2).abs() < 1e-6);
    }

    #[test]
    fn without_leg_colliders_and_skin_there_is_no_radius_and_no_fallback() {
        let (bones, _) = legs_with_colliders();
        assert!(torso_fallback_colliders(&bones, &[], &|_, _, _| None).is_empty());
    }

    #[test]
    fn fitted_fallback_moves_through_each_bone_bind_frame() {
        let (role_bones, existing) = legs_with_colliders();
        let mut skeleton = Skeleton::default();
        for role_bone in &role_bones {
            let bind = Mat4::from_translation(role_bone.head) * Mat4::from_scale(Vec3::splat(2.0));
            skeleton.bones.push(Bone {
                id: role_bone.bone,
                name: format!("{:?}", role_bone.role),
                inverse_bind_pose: bind.try_inverse().unwrap(),
                ..Bone::default()
            });
        }
        let mut mapping = HumanoidMapping::default();
        for role_bone in &role_bones {
            mapping
                .by_role
                .insert(role_bone.role, role_bone.bone as usize);
        }
        let fitted: Vec<FittedCapsule> = existing
            .iter()
            .map(|collider| FittedCapsule {
                bone_id: collider.bone,
                head: (collider.head
                    - role_bones
                        .iter()
                        .find(|b| b.bone == collider.bone)
                        .unwrap()
                        .head)
                    / 2.0,
                tail: (collider.tail
                    - role_bones
                        .iter()
                        .find(|b| b.bone == collider.bone)
                        .unwrap()
                        .head)
                    / 2.0,
                radius: collider.radius / 2.0,
            })
            .collect();

        let fallback = fitted_torso_fallback(&skeleton, &mapping, &fitted, &|_, _, _| None);

        let pelvis = fallback.iter().find(|c| c.bone_id == 0).expect("pelvis");
        assert!((pelvis.radius - 0.045).abs() < 1e-5);
        assert!((pelvis.head - Vec3::new(0.045, -0.025, 0.0)).length() < 1e-5);
        assert!((pelvis.tail - Vec3::new(-0.045, -0.025, 0.0)).length() < 1e-5);
    }
}
