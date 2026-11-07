use glam::{Mat3, Mat4, Vec3};
use thyllore_math_core::{
    closest_segment_parameters, mean_axis_scale, project_point_to_segment, world_y_axis,
};

use super::setup::{ColliderShape, SpringColliderDef};

const MIN_TAIL_LEVER: f32 = 1e-3;
/// A rest clearance below this is a joint authored on the collider axis, which the full radius pushes out.
pub(super) const MIN_REST_CLEARANCE: f32 = 1e-4;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldCollider {
    pub center: Vec3,
    pub radius: f32,
    pub tail: Option<Vec3>,
}

/// The collider placed by its bone's transform; offset, tail and radius are bone-local (VRM 1.0).
pub fn world_collider(def: &SpringColliderDef, bone_transform: &Mat4) -> WorldCollider {
    let place = |local: Vec3| bone_transform.transform_point3(local);
    let bone_scale = mean_axis_scale(&Mat3::from_cols(
        bone_transform.x_axis.truncate(),
        bone_transform.y_axis.truncate(),
        bone_transform.z_axis.truncate(),
    ));
    let (radius, tail) = match &def.shape {
        ColliderShape::Sphere { radius } => (*radius, None),
        ColliderShape::Capsule { radius, tail } => (*radius, Some(place(*tail))),
    };
    WorldCollider {
        center: place(def.offset),
        radius: radius * bone_scale,
        tail,
    }
}

/// How much of a joint's rest penetration a collider still tolerates.
///
/// `axis_distances[i]` is the joint's distance from collider `i` at rest; `weight` scales the
/// tolerated penetration from the whole of it (1.0, a root joint) to none (0.0, a strand tip).
#[derive(Clone, Copy, Debug)]
pub struct RestTolerance<'a> {
    pub axis_distances: &'a [f32],
    pub weight: f32,
}

impl RestTolerance<'static> {
    pub const NONE: Self = Self {
        axis_distances: &[],
        weight: 0.0,
    };
}

pub fn resolve_all_segment_collisions(
    head: Vec3,
    tail: Vec3,
    joint_radius: f32,
    colliders: &[WorldCollider],
    rest: RestTolerance,
) -> Vec3 {
    let mut resolved_tail = tail;
    for (index, collider) in colliders.iter().enumerate() {
        let min_distance = effective_min_distance(joint_radius, collider, rest, index);
        resolved_tail = resolve_segment_collision(head, resolved_tail, min_distance, collider);
    }
    resolved_tail
}

pub fn resolve_all_tail_collisions(
    tail: Vec3,
    joint_radius: f32,
    colliders: &[WorldCollider],
    rest: RestTolerance,
) -> Vec3 {
    let mut resolved_tail = tail;
    for (index, collider) in colliders.iter().enumerate() {
        let min_distance = effective_min_distance(joint_radius, collider, rest, index);
        resolved_tail = resolve_tail_collision(resolved_tail, min_distance, collider);
    }
    resolved_tail
}

fn effective_min_distance(
    joint_radius: f32,
    collider: &WorldCollider,
    rest: RestTolerance,
    collider_index: usize,
) -> f32 {
    let full = joint_radius + collider.radius;
    match rest.axis_distances.get(collider_index) {
        Some(&rest_distance) if rest_distance >= MIN_REST_CLEARANCE && rest_distance < full => {
            full - (full - rest_distance) * rest.weight.clamp(0.0, 1.0)
        }
        Some(_) | None => full,
    }
}

/// Distance from the tail to the collider axis (center or capsule segment).
pub fn tail_axis_distance(tail: Vec3, collider: &WorldCollider) -> f32 {
    let closest = match collider.tail {
        Some(capsule_tail) => project_point_to_segment(tail, collider.center, capsule_tail).1,
        None => collider.center,
    };
    (tail - closest).length()
}

/// Distance between the joint segment and the collider axis.
pub fn segment_axis_distance(head: Vec3, tail: Vec3, collider: &WorldCollider) -> f32 {
    let collider_tail = collider.tail.unwrap_or(collider.center);
    let (s, t) = closest_segment_parameters(head, tail, collider.center, collider_tail);
    let joint_point = head + (tail - head) * s;
    let collider_point = collider.center + (collider_tail - collider.center) * t;
    (joint_point - collider_point).length()
}

fn resolve_tail_collision(tail: Vec3, min_distance: f32, collider: &WorldCollider) -> Vec3 {
    let closest = match collider.tail {
        Some(capsule_tail) => project_point_to_segment(tail, collider.center, capsule_tail).1,
        None => collider.center,
    };
    let diff = tail - closest;
    let distance = diff.length();

    if distance >= min_distance {
        return tail;
    }

    let penetration = min_distance - distance;
    let push_direction = if distance < 1e-8 {
        world_y_axis()
    } else {
        diff / distance
    };

    tail + push_direction * penetration
}

fn resolve_segment_collision(
    head: Vec3,
    tail: Vec3,
    min_distance: f32,
    collider: &WorldCollider,
) -> Vec3 {
    let collider_tail = collider.tail.unwrap_or(collider.center);
    let (s, t) = closest_segment_parameters(head, tail, collider.center, collider_tail);
    let joint_point = head + (tail - head) * s;
    let collider_point = collider.center + (collider_tail - collider.center) * t;

    let diff = joint_point - collider_point;
    let distance = diff.length();

    if distance >= min_distance || s <= MIN_TAIL_LEVER {
        return tail;
    }

    let penetration = min_distance - distance;
    let push_direction = if distance < 1e-8 {
        world_y_axis()
    } else {
        diff / distance
    };

    tail + push_direction * (penetration / s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: Vec3, b: Vec3, eps: f32) -> bool {
        (a.x - b.x).abs() < eps && (a.y - b.y).abs() < eps && (a.z - b.z).abs() < eps
    }

    #[test]
    fn world_collider_follows_bone_rotation_and_scale() {
        let bone_transform = Mat4::from_translation(Vec3::new(0.0, 1.0, 0.0))
            * Mat4::from_rotation_z(f32::to_radians(90.0))
            * Mat4::from_scale(Vec3::splat(2.0));
        let def = SpringColliderDef {
            id: 0,
            bone_id: 0,
            offset: Vec3::new(0.1, 0.0, 0.0),
            shape: ColliderShape::Capsule {
                radius: 0.05,
                tail: Vec3::new(0.3, 0.0, 0.0),
            },
        };

        let placed = world_collider(&def, &bone_transform);

        assert!(approx_eq(placed.center, Vec3::new(0.0, 1.2, 0.0), 1e-5));
        let tail = placed.tail.expect("a capsule keeps its tail");
        assert!(approx_eq(tail, Vec3::new(0.0, 1.6, 0.0), 1e-5));
        assert!((placed.radius - 0.1).abs() < 1e-6);
    }

    fn magnitude(v: Vec3) -> f32 {
        (v.x * v.x + v.y * v.y + v.z * v.z).sqrt()
    }

    #[test]
    fn test_no_collision_distant_spheres() {
        let head = Vec3::new(5.0, 0.0, 0.0);
        let tail = Vec3::new(6.0, 0.0, 0.0);
        let colliders = [WorldCollider {
            center: Vec3::new(0.0, 0.0, 0.0),
            radius: 0.5,
            tail: None,
        }];
        let result =
            resolve_all_segment_collisions(head, tail, 0.1, &colliders, RestTolerance::NONE);
        assert!(approx_eq(result, tail, 1e-5));
    }

    #[test]
    fn test_collision_penetrating_spheres() {
        let head = Vec3::new(1.0, 0.0, 0.0);
        let tail = Vec3::new(0.3, 0.0, 0.0);
        let center = Vec3::new(0.0, 0.0, 0.0);
        let colliders = [WorldCollider {
            center,
            radius: 0.5,
            tail: None,
        }];
        let result =
            resolve_all_segment_collisions(head, tail, 0.1, &colliders, RestTolerance::NONE);

        let dist = magnitude(result - center);
        assert!((dist - 0.6).abs() < 1e-4);
        assert!(result.x > 0.0);
    }

    #[test]
    fn test_no_collision_at_exact_contact() {
        let head = Vec3::new(1.0, 0.0, 0.0);
        let tail = Vec3::new(0.6, 0.0, 0.0);
        let center = Vec3::new(0.0, 0.0, 0.0);
        let colliders = [WorldCollider {
            center,
            radius: 0.5,
            tail: None,
        }];
        let result =
            resolve_all_segment_collisions(head, tail, 0.1, &colliders, RestTolerance::NONE);
        assert!(approx_eq(result, tail, 1e-5));
    }

    #[test]
    fn test_capsule_collision_near_center() {
        let head = Vec3::new(1.0, 1.0, 0.0);
        let tail = Vec3::new(0.3, 1.0, 0.0);
        let capsule_head = Vec3::new(0.0, 0.0, 0.0);
        let capsule_tail = Vec3::new(0.0, 2.0, 0.0);
        let colliders = [WorldCollider {
            center: capsule_head,
            radius: 0.5,
            tail: Some(capsule_tail),
        }];
        let result =
            resolve_all_segment_collisions(head, tail, 0.1, &colliders, RestTolerance::NONE);
        let dist = magnitude(result - Vec3::new(0.0, 1.0, 0.0));
        assert!((dist - 0.6).abs() < 1e-4);
    }

    #[test]
    fn test_capsule_collision_near_endpoint() {
        let head = Vec3::new(1.0, -0.5, 0.0);
        let tail = Vec3::new(0.3, -0.5, 0.0);
        let capsule_head = Vec3::new(0.0, 0.0, 0.0);
        let capsule_tail = Vec3::new(0.0, 2.0, 0.0);
        let colliders = [WorldCollider {
            center: capsule_head,
            radius: 0.5,
            tail: Some(capsule_tail),
        }];
        let result =
            resolve_all_segment_collisions(head, tail, 0.1, &colliders, RestTolerance::NONE);
        let dist = magnitude(result - capsule_head);
        assert!((dist - 0.6).abs() < 1e-4);
    }

    #[test]
    fn test_degenerate_capsule_acts_as_sphere() {
        let head = Vec3::new(1.0, 0.0, 0.0);
        let tail = Vec3::new(0.3, 0.0, 0.0);
        let center = Vec3::new(0.0, 0.0, 0.0);
        let colliders = [WorldCollider {
            center,
            radius: 0.5,
            tail: Some(center),
        }];
        let result =
            resolve_all_segment_collisions(head, tail, 0.1, &colliders, RestTolerance::NONE);
        let dist = magnitude(result - center);
        assert!((dist - 0.6).abs() < 1e-4);
    }

    #[test]
    fn test_segment_midpoint_penetrates_capsule() {
        let head = Vec3::new(0.0, 0.0, 0.0);
        let tail = Vec3::new(0.0, 4.0, 0.0);
        let capsule_head = Vec3::new(1.5, 2.0, 0.0);
        let capsule_tail = Vec3::new(-1.5, 2.0, 0.0);
        let colliders = [WorldCollider {
            center: capsule_head,
            radius: 0.3,
            tail: Some(capsule_tail),
        }];

        let segment_result =
            resolve_all_segment_collisions(head, tail, 0.1, &colliders, RestTolerance::NONE);
        assert!(approx_eq(segment_result, Vec3::new(0.0, 4.8, 0.0), 1e-4));
    }

    #[test]
    fn test_segment_far_from_collider_no_movement() {
        let head = Vec3::new(0.0, 0.0, 0.0);
        let tail = Vec3::new(0.0, 2.0, 0.0);
        let colliders = [WorldCollider {
            center: Vec3::new(10.0, 1.0, 0.0),
            radius: 0.5,
            tail: None,
        }];

        let result =
            resolve_all_segment_collisions(head, tail, 0.1, &colliders, RestTolerance::NONE);
        assert!(approx_eq(result, tail, 1e-5));
    }

    #[test]
    fn test_only_tip_penetrates_matches_point_based() {
        let head = Vec3::new(1.0, 0.0, 0.0);
        let tail = Vec3::new(0.3, 0.0, 0.0);
        let center = Vec3::new(0.0, 0.0, 0.0);
        let colliders = [WorldCollider {
            center,
            radius: 0.5,
            tail: None,
        }];

        let segment_result =
            resolve_all_segment_collisions(head, tail, 0.1, &colliders, RestTolerance::NONE);
        assert!(approx_eq(segment_result, Vec3::new(0.6, 0.0, 0.0), 1e-4));
    }

    #[test]
    fn test_head_inside_collider_tail_unchanged() {
        let head = Vec3::new(0.0, 0.0, 0.0);
        let tail = Vec3::new(0.0, 3.0, 0.0);
        let colliders = [WorldCollider {
            center: Vec3::new(0.0, 0.0, 0.0),
            radius: 1.0,
            tail: None,
        }];

        let result =
            resolve_all_segment_collisions(head, tail, 0.1, &colliders, RestTolerance::NONE);
        assert!(approx_eq(result, tail, 1e-5));
    }

    #[test]
    fn test_complete_overlap_default_direction() {
        let head = Vec3::new(1.0, 1.4, 3.0);
        let tail = Vec3::new(1.0, 2.6, 3.0);
        let center = Vec3::new(1.0, 2.0, 3.0);
        let colliders = [WorldCollider {
            center,
            radius: 0.5,
            tail: None,
        }];

        let result =
            resolve_all_segment_collisions(head, tail, 0.1, &colliders, RestTolerance::NONE);
        let expected = Vec3::new(1.0, 3.8, 3.0);
        assert!(approx_eq(result, expected, 1e-4));
    }

    #[test]
    fn test_tail_collision_distant_sphere_unchanged() {
        let tail = Vec3::new(5.0, 0.0, 0.0);
        let colliders = [WorldCollider {
            center: Vec3::new(0.0, 0.0, 0.0),
            radius: 0.5,
            tail: None,
        }];
        let result = resolve_all_tail_collisions(tail, 0.1, &colliders, RestTolerance::NONE);
        assert!(approx_eq(result, tail, 1e-5));
    }

    #[test]
    fn test_tail_collision_penetrating_sphere_pushed_out() {
        let tail = Vec3::new(0.3, 0.0, 0.0);
        let center = Vec3::new(0.0, 0.0, 0.0);
        let colliders = [WorldCollider {
            center,
            radius: 0.5,
            tail: None,
        }];
        let result = resolve_all_tail_collisions(tail, 0.1, &colliders, RestTolerance::NONE);
        let dist = magnitude(result - center);
        assert!((dist - 0.6).abs() < 1e-4);
        assert!(result.x > 0.0);
    }

    #[test]
    fn test_tail_collision_capsule_pushed_perpendicular_from_axis_center() {
        let tail = Vec3::new(0.3, 1.0, 0.0);
        let capsule_head = Vec3::new(0.0, 0.0, 0.0);
        let capsule_tail = Vec3::new(0.0, 2.0, 0.0);
        let colliders = [WorldCollider {
            center: capsule_head,
            radius: 0.5,
            tail: Some(capsule_tail),
        }];
        let result = resolve_all_tail_collisions(tail, 0.1, &colliders, RestTolerance::NONE);
        assert!(approx_eq(result, Vec3::new(0.6, 1.0, 0.0), 1e-4));
    }

    fn unit_sphere_at_origin() -> [WorldCollider; 1] {
        [WorldCollider {
            center: Vec3::new(0.0, 0.0, 0.0),
            radius: 1.0,
            tail: None,
        }]
    }

    #[test]
    fn rest_clearance_keeps_a_tail_that_rests_inside_the_collider() {
        let tail = Vec3::new(0.8, 0.0, 0.0);
        let colliders = unit_sphere_at_origin();
        let rest = RestTolerance {
            axis_distances: &[tail_axis_distance(tail, &colliders[0])],
            weight: 1.0,
        };
        let result = resolve_all_tail_collisions(tail, 0.1, &colliders, rest);
        assert!(approx_eq(result, tail, 1e-6));
    }

    #[test]
    fn rest_clearance_still_pushes_a_tail_deeper_than_its_rest() {
        let colliders = unit_sphere_at_origin();
        let rest = RestTolerance {
            axis_distances: &[0.8],
            weight: 1.0,
        };
        let result = resolve_all_tail_collisions(Vec3::new(0.5, 0.0, 0.0), 0.1, &colliders, rest);
        assert!(approx_eq(result, Vec3::new(0.8, 0.0, 0.0), 1e-5));
    }

    #[test]
    fn rest_clearance_never_enlarges_the_collider() {
        let colliders = unit_sphere_at_origin();
        let rest = RestTolerance {
            axis_distances: &[5.0],
            weight: 1.0,
        };
        let result = resolve_all_tail_collisions(Vec3::new(1.05, 0.0, 0.0), 0.1, &colliders, rest);
        assert!(approx_eq(result, Vec3::new(1.1, 0.0, 0.0), 1e-5));
    }

    #[test]
    fn rest_clearance_applies_to_segment_mode_too() {
        let head = Vec3::new(0.9, 1.0, 0.0);
        let tail = Vec3::new(0.9, -1.0, 0.0);
        let colliders = unit_sphere_at_origin();
        let rest = RestTolerance {
            axis_distances: &[segment_axis_distance(head, tail, &colliders[0])],
            weight: 1.0,
        };
        let result = resolve_all_segment_collisions(head, tail, 0.1, &colliders, rest);
        assert!(approx_eq(result, tail, 1e-6));
    }

    #[test]
    fn a_rest_clearance_of_zero_keeps_the_collider_active() {
        let colliders = unit_sphere_at_origin();
        let rest = RestTolerance {
            axis_distances: &[0.0],
            weight: 1.0,
        };
        let result = resolve_all_tail_collisions(Vec3::new(0.0, 0.0, 0.0), 0.1, &colliders, rest);
        assert!((magnitude(result) - 1.1).abs() < 1e-5);
    }

    #[test]
    fn missing_rest_distances_use_the_full_radius() {
        let colliders = unit_sphere_at_origin();
        let result = resolve_all_tail_collisions(
            Vec3::new(0.8, 0.0, 0.0),
            0.1,
            &colliders,
            RestTolerance::NONE,
        );
        assert!(approx_eq(result, Vec3::new(1.1, 0.0, 0.0), 1e-5));
    }

    #[test]
    fn a_strand_tip_is_pushed_to_the_full_radius_despite_its_rest() {
        let colliders = unit_sphere_at_origin();
        let rest = RestTolerance {
            axis_distances: &[0.8],
            weight: 0.0,
        };
        let result = resolve_all_tail_collisions(Vec3::new(0.8, 0.0, 0.0), 0.1, &colliders, rest);
        assert!(approx_eq(result, Vec3::new(1.1, 0.0, 0.0), 1e-5));
    }

    #[test]
    fn a_mid_strand_joint_keeps_half_of_its_rest_penetration() {
        let colliders = unit_sphere_at_origin();
        let rest = RestTolerance {
            axis_distances: &[0.8],
            weight: 0.5,
        };
        let result = resolve_all_tail_collisions(Vec3::new(0.8, 0.0, 0.0), 0.1, &colliders, rest);
        assert!(approx_eq(result, Vec3::new(0.95, 0.0, 0.0), 1e-5));
    }
}
