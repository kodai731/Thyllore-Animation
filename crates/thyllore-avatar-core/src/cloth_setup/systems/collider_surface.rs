use glam::Vec3;
use thyllore_math_core::tangent_frame_around;

use thyllore_model_core::BoneId;

use crate::cloth_setup::components::chain_gen::{BodyCollider, SkinSurface, SUPPORT_RING_HEIGHT};
use crate::collider::systems::torso_volume::torso_roles;
use crate::humanoid::components::role::HumanoidRole;

const RING_SAMPLES: usize = 12;

/// Whether a bone of this role is part of the volume a garment can rest on: the torso, not the limbs.
pub fn supports_garments(role: HumanoidRole) -> bool {
    torso_roles().contains(&role)
}

fn sample_capsule(collider: &BodyCollider) -> Vec<Vec3> {
    let axis = collider.tail - collider.head;
    let axis_length = axis.length();
    let direction = if axis_length > 1e-6 {
        axis / axis_length
    } else {
        Vec3::Y
    };
    let (reference, side) = tangent_frame_around(direction);
    let axis_steps = ((axis_length / SUPPORT_RING_HEIGHT).ceil() as usize).max(1) + 1;

    let mut points = Vec::with_capacity(axis_steps * RING_SAMPLES + 2);
    for step in 0..axis_steps {
        let along = if axis_steps == 1 {
            0.0
        } else {
            step as f32 / (axis_steps - 1) as f32
        };
        let center = collider.head + direction * (axis_length * along);
        for ring in 0..RING_SAMPLES {
            let angle = std::f32::consts::TAU * ring as f32 / RING_SAMPLES as f32;
            let radial = reference * angle.cos() + side * angle.sin();
            points.push(center + radial * collider.radius);
        }
    }
    points.push(collider.head - direction * collider.radius);
    points.push(collider.tail + direction * collider.radius);
    points
}

fn heaviest_bone(influences: &[(BoneId, f32)]) -> Option<BoneId> {
    influences
        .iter()
        .filter(|(_, weight)| *weight > 0.0)
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(bone, _)| *bone)
}

/// Drops the influences of every skin vertex a limb drives, so only the torso remains a body surface.
pub fn retain_torso_skin(
    surfaces: &mut [SkinSurface],
    role_of: impl Fn(BoneId) -> Option<HumanoidRole>,
) {
    for surface in surfaces {
        for influences in &mut surface.influences {
            let on_limb = heaviest_bone(influences)
                .and_then(&role_of)
                .is_some_and(|role| !supports_garments(role));
            if on_limb {
                influences.clear();
            }
        }
    }
}

/// Every collider as a point set its bone drives, so it joins the skin meshes as a body surface.
pub fn collider_surfaces(colliders: &[BodyCollider]) -> Vec<SkinSurface> {
    colliders
        .iter()
        .map(|collider| SkinSurface::points_driven_by(collider.bone, sample_capsule(collider)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limbs_do_not_support_garments_but_the_torso_does() {
        for role in [
            HumanoidRole::LeftShoulder,
            HumanoidRole::RightUpperArm,
            HumanoidRole::LeftHand,
            HumanoidRole::LeftUpperLeg,
            HumanoidRole::RightLowerLeg,
            HumanoidRole::LeftFoot,
        ] {
            assert!(!supports_garments(role), "{role:?}");
        }
        for role in [
            HumanoidRole::Hips,
            HumanoidRole::Spine,
            HumanoidRole::Chest,
            HumanoidRole::Neck,
            HumanoidRole::Head,
        ] {
            assert!(supports_garments(role), "{role:?}");
        }
    }

    #[test]
    fn capsule_samples_lie_on_the_capsule_and_carry_the_bone() {
        let collider = BodyCollider {
            bone: 7,
            head: Vec3::new(0.0, 0.0, 0.0),
            tail: Vec3::new(0.0, 0.4, 0.0),
            radius: 0.1,
        };
        let surfaces = collider_surfaces(&[collider]);
        assert_eq!(surfaces.len(), 1);
        let surface = &surfaces[0];
        assert!(surface.influences.iter().all(|i| *i == vec![(7, 1.0)]));
        for point in &surface.positions {
            let clamped_y = point.y.clamp(0.0, 0.4);
            let distance = (point - Vec3::new(0.0, clamped_y, 0.0)).length();
            assert!((distance - 0.1).abs() < 1e-5, "{point:?}");
        }
    }

    #[test]
    fn capsule_samples_are_no_further_apart_than_a_support_ring() {
        let collider = BodyCollider {
            bone: 1,
            head: Vec3::new(0.0, 0.0, 0.0),
            tail: Vec3::new(0.0, 0.7, 0.0),
            radius: 0.1,
        };
        let surface = &collider_surfaces(&[collider])[0];
        let mut heights: Vec<f32> = surface
            .positions
            .iter()
            .map(|p| p.y)
            .filter(|&y| (0.0..=0.7).contains(&y))
            .collect();
        heights.sort_by(f32::total_cmp);
        heights.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        for pair in heights.windows(2) {
            assert!(pair[1] - pair[0] <= SUPPORT_RING_HEIGHT + 1e-6, "{pair:?}");
        }
    }

    #[test]
    fn limb_skin_loses_its_influences_and_torso_skin_keeps_them() {
        let mut surfaces = vec![SkinSurface {
            positions: vec![Vec3::new(0.0, 0.0, 0.0); 3],
            triangles: Vec::new(),
            influences: vec![vec![(1, 0.8), (2, 0.2)], vec![(2, 0.9)], vec![(3, 1.0)]],
        }];
        let role_of = |bone: BoneId| match bone {
            1 => Some(HumanoidRole::LeftUpperLeg),
            2 => Some(HumanoidRole::Hips),
            _ => None,
        };

        retain_torso_skin(&mut surfaces, role_of);

        assert!(surfaces[0].influences[0].is_empty());
        assert_eq!(surfaces[0].influences[1], vec![(2, 0.9)]);
        assert_eq!(surfaces[0].influences[2], vec![(3, 1.0)]);
    }

    #[test]
    fn sphere_samples_lie_on_the_sphere() {
        let collider = BodyCollider {
            bone: 1,
            head: Vec3::new(1.0, 2.0, 3.0),
            tail: Vec3::new(1.0, 2.0, 3.0),
            radius: 0.05,
        };
        let surface = &collider_surfaces(&[collider])[0];
        for point in &surface.positions {
            assert!(((point - collider.head).length() - 0.05).abs() < 1e-5);
        }
    }
}
