use cgmath::{InnerSpace, Vector3};

const DEGENERATE_SEGMENT_LENGTH_SQ: f32 = 1e-8;
const MIN_TAIL_LEVER: f32 = 1e-3;

pub struct WorldCollider {
    pub center: Vector3<f32>,
    pub radius: f32,
    pub tail: Option<Vector3<f32>>,
}

/// Resolves the tail against every collider; `rest_axis_distances[i]` caps the clearance the joint
/// had from collider `i` at rest so the rest pose is never pushed.
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
        Some(&rest) => full.min(rest),
        None => full,
    }
}

/// Distance from the tail to the collider axis (center or capsule segment).
pub fn tail_axis_distance(tail: Vector3<f32>, collider: &WorldCollider) -> f32 {
    let closest = match collider.tail {
        Some(capsule_tail) => closest_point_on_segment(tail, collider.center, capsule_tail),
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
    let (s, t) = find_closest_segment_parameters(head, tail, collider.center, collider_tail);
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
        Some(capsule_tail) => closest_point_on_segment(tail, collider.center, capsule_tail),
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

fn closest_point_on_segment(p: Vector3<f32>, a: Vector3<f32>, b: Vector3<f32>) -> Vector3<f32> {
    let ab = b - a;
    let len_sq = ab.x * ab.x + ab.y * ab.y + ab.z * ab.z;
    if len_sq < 1e-8 {
        return a;
    }
    let t = ((p.x - a.x) * ab.x + (p.y - a.y) * ab.y + (p.z - a.z) * ab.z) / len_sq;
    let clamped_t = t.max(0.0).min(1.0);
    a + ab * clamped_t
}

fn resolve_segment_collision(
    head: Vector3<f32>,
    tail: Vector3<f32>,
    min_distance: f32,
    collider: &WorldCollider,
) -> Vector3<f32> {
    let collider_tail = collider.tail.unwrap_or(collider.center);
    let (s, t) = find_closest_segment_parameters(head, tail, collider.center, collider_tail);
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

// Ericson, Real-Time Collision Detection 5.1.9.
fn find_closest_segment_parameters(
    p1: Vector3<f32>,
    q1: Vector3<f32>,
    p2: Vector3<f32>,
    q2: Vector3<f32>,
) -> (f32, f32) {
    let d1 = q1 - p1;
    let d2 = q2 - p2;
    let r = p1 - p2;
    let a = d1.dot(d1);
    let e = d2.dot(d2);
    let f = d2.dot(r);

    if a <= DEGENERATE_SEGMENT_LENGTH_SQ && e <= DEGENERATE_SEGMENT_LENGTH_SQ {
        return (0.0, 0.0);
    }
    if a <= DEGENERATE_SEGMENT_LENGTH_SQ {
        return (0.0, (f / e).clamp(0.0, 1.0));
    }

    let c = d1.dot(r);
    if e <= DEGENERATE_SEGMENT_LENGTH_SQ {
        return ((-c / a).clamp(0.0, 1.0), 0.0);
    }

    let b = d1.dot(d2);
    let denom = a * e - b * b;
    let s = if denom > 0.0 {
        ((b * f - c * e) / denom).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let t = (b * s + f) / e;

    if t < 0.0 {
        ((-c / a).clamp(0.0, 1.0), 0.0)
    } else if t > 1.0 {
        (((b - c) / a).clamp(0.0, 1.0), 1.0)
    } else {
        (s, t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: Vector3<f32>, b: Vector3<f32>, eps: f32) -> bool {
        (a.x - b.x).abs() < eps && (a.y - b.y).abs() < eps && (a.z - b.z).abs() < eps
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
    fn missing_rest_distances_use_the_full_radius() {
        let colliders = unit_sphere_at_origin();
        let result = resolve_all_tail_collisions(Vector3::new(0.8, 0.0, 0.0), 0.1, &colliders, &[]);
        assert!(approx_eq(result, Vector3::new(1.1, 0.0, 0.0), 1e-5));
    }
}
