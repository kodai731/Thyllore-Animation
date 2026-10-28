use cgmath::{EuclideanSpace, InnerSpace, Matrix3, Matrix4, Point3, Transform, Vector3};
use thyllore_math_core::{closest_segment_parameters, mean_axis_scale, project_point_to_segment};

use super::setup::{ColliderShape, SpringColliderDef};

const MIN_TAIL_LEVER: f32 = 1e-3;
/// A rest clearance below this is a joint authored on the collider axis, which the full radius pushes out.
const MIN_REST_CLEARANCE: f32 = 1e-4;

pub struct WorldCollider {
    pub center: Vector3<f32>,
    pub radius: f32,
    pub tail: Option<Vector3<f32>>,
}

/// The collider placed by its bone's transform; offset, tail and radius are bone-local (VRM 1.0).
pub fn world_collider(def: &SpringColliderDef, bone_transform: &Matrix4<f32>) -> WorldCollider {
    let place = |local: Vector3<f32>| {
        bone_transform
            .transform_point(Point3::from_vec(local))
            .to_vec()
    };
    let bone_scale = mean_axis_scale(&Matrix3::from_cols(
        bone_transform.x.truncate(),
        bone_transform.y.truncate(),
        bone_transform.z.truncate(),
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

/// Resolves the tail against every collider; `rest_axis_distances[i]` caps the clearance the joint
/// had from collider `i` at rest so the rest pose is never pushed, and an empty slice keeps the full radius.
pub fn resolve_all_segment_collisions(
    head: Vector3<f32>,
    tail: Vector3<f32>,
    joint_radius: f32,
    colliders: &[WorldCollider],
    rest_axis_distances: &[f32],
) -> Vector3<f32> {
    let mut resolved_tail = tail;
    for (index, collider) in colliders.iter().enumerate() {
        let min_distance =
            effective_min_distance(joint_radius, collider, rest_axis_distances.get(index));
        resolved_tail = resolve_segment_collision(head, resolved_tail, min_distance, collider);
    }
    resolved_tail
}

pub fn resolve_all_tail_collisions(
    tail: Vector3<f32>,
    joint_radius: f32,
    colliders: &[WorldCollider],
    rest_axis_distances: &[f32],
) -> Vector3<f32> {
    let mut resolved_tail = tail;
    for (index, collider) in colliders.iter().enumerate() {
        let min_distance =
            effective_min_distance(joint_radius, collider, rest_axis_distances.get(index));
        resolved_tail = resolve_tail_collision(resolved_tail, min_distance, collider);
    }
    resolved_tail
}

fn effective_min_distance(
    joint_radius: f32,
    collider: &WorldCollider,
    rest_axis_distance: Option<&f32>,
) -> f32 {
    let full = joint_radius + collider.radius;
    match rest_axis_distance {
        Some(&rest) if rest >= MIN_REST_CLEARANCE => full.min(rest),
        Some(_) | None => full,
    }
}

/// Distance from the tail to the collider axis (center or capsule segment).
pub fn tail_axis_distance(tail: Vector3<f32>, collider: &WorldCollider) -> f32 {
    let closest = match collider.tail {
        Some(capsule_tail) => project_point_to_segment(tail, collider.center, capsule_tail).1,
        None => collider.center,
    };
    (tail - closest).magnitude()
}

/// Distance between the joint segment and the collider axis.
pub fn segment_axis_distance(
    head: Vector3<f32>,
    tail: Vector3<f32>,
    collider: &WorldCollider,
) -> f32 {
    let collider_tail = collider.tail.unwrap_or(collider.center);
    let (s, t) = closest_segment_parameters(head, tail, collider.center, collider_tail);
    let joint_point = head + (tail - head) * s;
    let collider_point = collider.center + (collider_tail - collider.center) * t;
    (joint_point - collider_point).magnitude()
}

fn resolve_tail_collision(
    tail: Vector3<f32>,
    min_distance: f32,
    collider: &WorldCollider,
) -> Vector3<f32> {
    let closest = match collider.tail {
        Some(capsule_tail) => project_point_to_segment(tail, collider.center, capsule_tail).1,
        None => collider.center,
    };
    let diff = tail - closest;
    let distance = diff.magnitude();

    if distance >= min_distance {
        return tail;
    }

    let penetration = min_distance - distance;
    let push_direction = if distance < 1e-8 {
        Vector3::new(0.0, 1.0, 0.0)
    } else {
        diff / distance
    };

    tail + push_direction * penetration
}

fn resolve_segment_collision(
    head: Vector3<f32>,
    tail: Vector3<f32>,
    min_distance: f32,
    collider: &WorldCollider,
) -> Vector3<f32> {
    let collider_tail = collider.tail.unwrap_or(collider.center);
    let (s, t) = closest_segment_parameters(head, tail, collider.center, collider_tail);
    let joint_point = head + (tail - head) * s;
    let collider_point = collider.center + (collider_tail - collider.center) * t;

    let diff = joint_point - collider_point;
    let distance = diff.magnitude();

    if distance >= min_distance || s <= MIN_TAIL_LEVER {
        return tail;
    }

    let penetration = min_distance - distance;
    let push_direction = if distance < 1e-8 {
        Vector3::new(0.0, 1.0, 0.0)
    } else {
        diff / distance
    };

    tail + push_direction * (penetration / s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: Vector3<f32>, b: Vector3<f32>, eps: f32) -> bool {
        (a.x - b.x).abs() < eps && (a.y - b.y).abs() < eps && (a.z - b.z).abs() < eps
    }

    #[test]
    fn world_collider_follows_bone_rotation_and_scale() {
        let bone_transform = Matrix4::from_translation(Vector3::new(0.0, 1.0, 0.0))
            * Matrix4::from_angle_z(cgmath::Deg(90.0))
            * Matrix4::from_scale(2.0);
        let def = SpringColliderDef {
            id: 0,
            bone_id: 0,
            offset: Vector3::new(0.1, 0.0, 0.0),
            shape: ColliderShape::Capsule {
                radius: 0.05,
                tail: Vector3::new(0.3, 0.0, 0.0),
            },
        };

        let placed = world_collider(&def, &bone_transform);

        assert!(approx_eq(placed.center, Vector3::new(0.0, 1.2, 0.0), 1e-5));
        let tail = placed.tail.expect("a capsule keeps its tail");
        assert!(approx_eq(tail, Vector3::new(0.0, 1.6, 0.0), 1e-5));
        assert!((placed.radius - 0.1).abs() < 1e-6);
    }

    fn magnitude(v: Vector3<f32>) -> f32 {
        (v.x * v.x + v.y * v.y + v.z * v.z).sqrt()
    }

    #[test]
    fn test_no_collision_distant_spheres() {
        let head = Vector3::new(5.0, 0.0, 0.0);
        let tail = Vector3::new(6.0, 0.0, 0.0);
        let colliders = [WorldCollider {
            center: Vector3::new(0.0, 0.0, 0.0),
            radius: 0.5,
            tail: None,
        }];
        let result = resolve_all_segment_collisions(head, tail, 0.1, &colliders, &[]);
        assert!(approx_eq(result, tail, 1e-5));
    }

    #[test]
    fn test_collision_penetrating_spheres() {
        let head = Vector3::new(1.0, 0.0, 0.0);
        let tail = Vector3::new(0.3, 0.0, 0.0);
        let center = Vector3::new(0.0, 0.0, 0.0);
        let colliders = [WorldCollider {
            center,
            radius: 0.5,
            tail: None,
        }];
        let result = resolve_all_segment_collisions(head, tail, 0.1, &colliders, &[]);

        let dist = magnitude(result - center);
        assert!((dist - 0.6).abs() < 1e-4);
        assert!(result.x > 0.0);
    }

    #[test]
    fn test_no_collision_at_exact_contact() {
        let head = Vector3::new(1.0, 0.0, 0.0);
        let tail = Vector3::new(0.6, 0.0, 0.0);
        let center = Vector3::new(0.0, 0.0, 0.0);
        let colliders = [WorldCollider {
            center,
            radius: 0.5,
            tail: None,
        }];
        let result = resolve_all_segment_collisions(head, tail, 0.1, &colliders, &[]);
        assert!(approx_eq(result, tail, 1e-5));
    }

    #[test]
    fn test_capsule_collision_near_center() {
        let head = Vector3::new(1.0, 1.0, 0.0);
        let tail = Vector3::new(0.3, 1.0, 0.0);
        let capsule_head = Vector3::new(0.0, 0.0, 0.0);
        let capsule_tail = Vector3::new(0.0, 2.0, 0.0);
        let colliders = [WorldCollider {
            center: capsule_head,
            radius: 0.5,
            tail: Some(capsule_tail),
        }];
        let result = resolve_all_segment_collisions(head, tail, 0.1, &colliders, &[]);
        let dist = magnitude(result - Vector3::new(0.0, 1.0, 0.0));
        assert!((dist - 0.6).abs() < 1e-4);
    }

    #[test]
    fn test_capsule_collision_near_endpoint() {
        let head = Vector3::new(1.0, -0.5, 0.0);
        let tail = Vector3::new(0.3, -0.5, 0.0);
        let capsule_head = Vector3::new(0.0, 0.0, 0.0);
        let capsule_tail = Vector3::new(0.0, 2.0, 0.0);
        let colliders = [WorldCollider {
            center: capsule_head,
            radius: 0.5,
            tail: Some(capsule_tail),
        }];
        let result = resolve_all_segment_collisions(head, tail, 0.1, &colliders, &[]);
        let dist = magnitude(result - capsule_head);
        assert!((dist - 0.6).abs() < 1e-4);
    }

    #[test]
    fn test_degenerate_capsule_acts_as_sphere() {
        let head = Vector3::new(1.0, 0.0, 0.0);
        let tail = Vector3::new(0.3, 0.0, 0.0);
        let center = Vector3::new(0.0, 0.0, 0.0);
        let colliders = [WorldCollider {
            center,
            radius: 0.5,
            tail: Some(center),
        }];
        let result = resolve_all_segment_collisions(head, tail, 0.1, &colliders, &[]);
        let dist = magnitude(result - center);
        assert!((dist - 0.6).abs() < 1e-4);
    }

    #[test]
    fn test_segment_midpoint_penetrates_capsule() {
        let head = Vector3::new(0.0, 0.0, 0.0);
        let tail = Vector3::new(0.0, 4.0, 0.0);
        let capsule_head = Vector3::new(1.5, 2.0, 0.0);
        let capsule_tail = Vector3::new(-1.5, 2.0, 0.0);
        let colliders = [WorldCollider {
            center: capsule_head,
            radius: 0.3,
            tail: Some(capsule_tail),
        }];

        let segment_result = resolve_all_segment_collisions(head, tail, 0.1, &colliders, &[]);
        assert!(approx_eq(segment_result, Vector3::new(0.0, 4.8, 0.0), 1e-4));
    }

    #[test]
    fn test_segment_far_from_collider_no_movement() {
        let head = Vector3::new(0.0, 0.0, 0.0);
        let tail = Vector3::new(0.0, 2.0, 0.0);
        let colliders = [WorldCollider {
            center: Vector3::new(10.0, 1.0, 0.0),
            radius: 0.5,
            tail: None,
        }];

        let result = resolve_all_segment_collisions(head, tail, 0.1, &colliders, &[]);
        assert!(approx_eq(result, tail, 1e-5));
    }

    #[test]
    fn test_only_tip_penetrates_matches_point_based() {
        let head = Vector3::new(1.0, 0.0, 0.0);
        let tail = Vector3::new(0.3, 0.0, 0.0);
        let center = Vector3::new(0.0, 0.0, 0.0);
        let colliders = [WorldCollider {
            center,
            radius: 0.5,
            tail: None,
        }];

        let segment_result = resolve_all_segment_collisions(head, tail, 0.1, &colliders, &[]);
        assert!(approx_eq(segment_result, Vector3::new(0.6, 0.0, 0.0), 1e-4));
    }

    #[test]
    fn test_head_inside_collider_tail_unchanged() {
        let head = Vector3::new(0.0, 0.0, 0.0);
        let tail = Vector3::new(0.0, 3.0, 0.0);
        let colliders = [WorldCollider {
            center: Vector3::new(0.0, 0.0, 0.0),
            radius: 1.0,
            tail: None,
        }];

        let result = resolve_all_segment_collisions(head, tail, 0.1, &colliders, &[]);
        assert!(approx_eq(result, tail, 1e-5));
    }

    #[test]
    fn test_complete_overlap_default_direction() {
        let head = Vector3::new(1.0, 1.4, 3.0);
        let tail = Vector3::new(1.0, 2.6, 3.0);
        let center = Vector3::new(1.0, 2.0, 3.0);
        let colliders = [WorldCollider {
            center,
            radius: 0.5,
            tail: None,
        }];

        let result = resolve_all_segment_collisions(head, tail, 0.1, &colliders, &[]);
        let expected = Vector3::new(1.0, 3.8, 3.0);
        assert!(approx_eq(result, expected, 1e-4));
    }

    #[test]
    fn test_tail_collision_distant_sphere_unchanged() {
        let tail = Vector3::new(5.0, 0.0, 0.0);
        let colliders = [WorldCollider {
            center: Vector3::new(0.0, 0.0, 0.0),
            radius: 0.5,
            tail: None,
        }];
        let result = resolve_all_tail_collisions(tail, 0.1, &colliders, &[]);
        assert!(approx_eq(result, tail, 1e-5));
    }

    #[test]
    fn test_tail_collision_penetrating_sphere_pushed_out() {
        let tail = Vector3::new(0.3, 0.0, 0.0);
        let center = Vector3::new(0.0, 0.0, 0.0);
        let colliders = [WorldCollider {
            center,
            radius: 0.5,
            tail: None,
        }];
        let result = resolve_all_tail_collisions(tail, 0.1, &colliders, &[]);
        let dist = magnitude(result - center);
        assert!((dist - 0.6).abs() < 1e-4);
        assert!(result.x > 0.0);
    }

    #[test]
    fn test_tail_collision_capsule_pushed_perpendicular_from_axis_center() {
        let tail = Vector3::new(0.3, 1.0, 0.0);
        let capsule_head = Vector3::new(0.0, 0.0, 0.0);
        let capsule_tail = Vector3::new(0.0, 2.0, 0.0);
        let colliders = [WorldCollider {
            center: capsule_head,
            radius: 0.5,
            tail: Some(capsule_tail),
        }];
        let result = resolve_all_tail_collisions(tail, 0.1, &colliders, &[]);
        assert!(approx_eq(result, Vector3::new(0.6, 1.0, 0.0), 1e-4));
    }

    fn unit_sphere_at_origin() -> [WorldCollider; 1] {
        [WorldCollider {
            center: Vector3::new(0.0, 0.0, 0.0),
            radius: 1.0,
            tail: None,
        }]
    }

    #[test]
    fn rest_clearance_keeps_a_tail_that_rests_inside_the_collider() {
        let tail = Vector3::new(0.8, 0.0, 0.0);
        let colliders = unit_sphere_at_origin();
        let rest = [tail_axis_distance(tail, &colliders[0])];
        let result = resolve_all_tail_collisions(tail, 0.1, &colliders, &rest);
        assert!(approx_eq(result, tail, 1e-6));
    }

    #[test]
    fn rest_clearance_still_pushes_a_tail_deeper_than_its_rest() {
        let colliders = unit_sphere_at_origin();
        let rest = [0.8];
        let result =
            resolve_all_tail_collisions(Vector3::new(0.5, 0.0, 0.0), 0.1, &colliders, &rest);
        assert!(approx_eq(result, Vector3::new(0.8, 0.0, 0.0), 1e-5));
    }

    #[test]
    fn rest_clearance_never_enlarges_the_collider() {
        let colliders = unit_sphere_at_origin();
        let rest = [5.0];
        let result =
            resolve_all_tail_collisions(Vector3::new(1.05, 0.0, 0.0), 0.1, &colliders, &rest);
        assert!(approx_eq(result, Vector3::new(1.1, 0.0, 0.0), 1e-5));
    }

    #[test]
    fn rest_clearance_applies_to_segment_mode_too() {
        let head = Vector3::new(0.9, 1.0, 0.0);
        let tail = Vector3::new(0.9, -1.0, 0.0);
        let colliders = unit_sphere_at_origin();
        let rest = [segment_axis_distance(head, tail, &colliders[0])];
        let result = resolve_all_segment_collisions(head, tail, 0.1, &colliders, &rest);
        assert!(approx_eq(result, tail, 1e-6));
    }

    #[test]
    fn a_rest_clearance_of_zero_keeps_the_collider_active() {
        let colliders = unit_sphere_at_origin();
        let rest = [0.0];
        let result =
            resolve_all_tail_collisions(Vector3::new(0.0, 0.0, 0.0), 0.1, &colliders, &rest);
        assert!((magnitude(result) - 1.1).abs() < 1e-5);
    }

    #[test]
    fn missing_rest_distances_use_the_full_radius() {
        let colliders = unit_sphere_at_origin();
        let result = resolve_all_tail_collisions(Vector3::new(0.8, 0.0, 0.0), 0.1, &colliders, &[]);
        assert!(approx_eq(result, Vector3::new(1.1, 0.0, 0.0), 1e-5));
    }
}
