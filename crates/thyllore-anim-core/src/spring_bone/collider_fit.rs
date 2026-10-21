use std::sync::LazyLock;

use cgmath::{EuclideanSpace, InnerSpace, Point3, Transform, Vector3};

use thyllore_math_core::{fit_principal_axis, PrincipalAxis};
use thyllore_model_core::{Bone, BoneId, Skeleton, SkinData};
use thyllore_scene_core::{SceneFields, UiParam};

use super::setup::{ColliderShape, SpringBoneSetup, SpringColliderDef, SpringColliderGroup};

#[derive(Clone, Debug, PartialEq, SceneFields)]
#[params(tag = thyllore_scene_core::SingleOwner, owner = Component)]
pub struct CapsuleFitParams {
    /// Skin weight under which a vertex does not count toward its bone's capsule
    #[persist(ui(min = 0.0, max = 1.0, format = "%.2f"))]
    pub min_weight: f32,
    /// Percentile of the vertex distances from the bone axis taken as the capsule radius
    #[persist(ui(min = 0.0, max = 1.0, format = "%.2f"))]
    pub radius_percentile: f32,
    /// Factor applied to the percentile radius
    #[persist(ui(min = 0.0, max = 2.0, format = "%.2f"))]
    pub radius_scale: f32,
    /// Bones with fewer weighted vertices get no capsule
    #[persist(ui(min = 1.0, max = 64.0, format = "%.0f"))]
    pub min_vertex_count: u32,
}

impl Default for CapsuleFitParams {
    fn default() -> Self {
        Self {
            min_weight: 0.5,
            radius_percentile: 0.9,
            radius_scale: 0.85,
            min_vertex_count: 8,
        }
    }
}

pub static CAPSULE_FIT_UI_PARAMS: LazyLock<Vec<UiParam>> = LazyLock::new(|| {
    let mut out = Vec::new();
    CapsuleFitParams::collect_ui("", "", &mut out);
    out
});

#[derive(Clone, Debug)]
pub struct FittedCapsule {
    pub bone_id: BoneId,
    pub head: Vector3<f32>,
    pub tail: Vector3<f32>,
    pub radius: f32,
}

pub const FITTED_BODY_GROUP_NAME: &str = "Fitted Body";

pub fn replace_fitted_body_colliders(setup: &mut SpringBoneSetup, capsules: &[FittedCapsule]) {
    remove_existing_fitted_group(setup);
    let new_group_id = add_new_colliders_and_group(setup, capsules);
    append_group_to_chains(setup, new_group_id);
}

fn remove_existing_fitted_group(setup: &mut SpringBoneSetup) {
    if let Some(group_pos) = setup
        .collider_groups
        .iter()
        .position(|g| g.name == FITTED_BODY_GROUP_NAME)
    {
        let group = &setup.collider_groups[group_pos];
        let group_id = group.id;
        for cid in &group.collider_ids {
            setup.colliders.retain(|c| c.id != *cid);
        }
        setup.collider_groups.remove(group_pos);
        for chain in &mut setup.chains {
            chain.collider_group_ids.retain(|&gid| gid != group_id);
        }
    }
}

fn add_new_colliders_and_group(setup: &mut SpringBoneSetup, capsules: &[FittedCapsule]) -> u32 {
    let mut collider_ids = Vec::new();
    for cap in capsules {
        let id = setup.next_collider_id;
        setup.next_collider_id += 1;
        collider_ids.push(id);
        setup.colliders.push(SpringColliderDef {
            id,
            bone_id: cap.bone_id,
            offset: cap.head,
            shape: ColliderShape::Capsule {
                radius: cap.radius,
                tail: cap.tail,
            },
        });
    }
    let group_id = setup.next_group_id;
    setup.next_group_id += 1;
    setup.collider_groups.push(SpringColliderGroup {
        id: group_id,
        name: FITTED_BODY_GROUP_NAME.to_string(),
        collider_ids,
    });
    group_id
}

fn append_group_to_chains(setup: &mut SpringBoneSetup, group_id: u32) {
    for chain in &mut setup.chains {
        if !chain.collider_group_ids.contains(&group_id) {
            chain.collider_group_ids.push(group_id);
        }
    }
}

pub fn fit_bone_capsules(
    skins: &[&SkinData],
    skeleton: &Skeleton,
    params: &CapsuleFitParams,
) -> Vec<FittedCapsule> {
    let mut capsules = Vec::new();

    for bone in &skeleton.bones {
        let local_points = collect_local_points(skins, bone, params.min_weight);
        if local_points.len() < params.min_vertex_count as usize {
            continue;
        }

        let pa = match fit_principal_axis(&local_points) {
            Some(pa) => pa,
            None => continue,
        };

        let (t_min, t_max) = project_to_axis(&local_points, &pa);
        let radius = compute_radius(&local_points, &pa, params);

        let mut head = pa.mean + pa.axis * (t_min + radius);
        let mut tail = pa.mean + pa.axis * (t_max - radius);

        if t_max - t_min < 2.0 * radius {
            let mid = pa.mean + pa.axis * (t_min + t_max) * 0.5;
            head = mid;
            tail = mid;
        }

        capsules.push(FittedCapsule {
            bone_id: bone.id,
            head,
            tail,
            radius,
        });
    }

    capsules
}

fn collect_local_points(skins: &[&SkinData], bone: &Bone, min_weight: f32) -> Vec<Vector3<f32>> {
    let mut points = Vec::new();
    let bone_id = bone.id as usize;

    for skin in skins {
        for (i, pos) in skin.base_positions.iter().enumerate() {
            let indices = &skin.bone_indices[i];
            let weights = &skin.bone_weights[i];

            let comps: [(u32, f32); 4] = [
                (indices.x, weights.x),
                (indices.y, weights.y),
                (indices.z, weights.z),
                (indices.w, weights.w),
            ];

            if comps
                .iter()
                .any(|(idx, weight)| *idx as usize == bone_id && *weight >= min_weight)
            {
                let local = bone
                    .inverse_bind_pose
                    .transform_point(Point3::from_vec(*pos))
                    .to_vec();
                points.push(local);
            }
        }
    }

    points
}

fn project_to_axis(points: &[Vector3<f32>], pa: &PrincipalAxis) -> (f32, f32) {
    let mut t_min = f32::MAX;
    let mut t_max = f32::MIN;

    for p in points {
        let t = (p - pa.mean).dot(pa.axis);
        if t < t_min {
            t_min = t;
        }
        if t > t_max {
            t_max = t;
        }
    }

    (t_min, t_max)
}

fn compute_radius(points: &[Vector3<f32>], axis: &PrincipalAxis, params: &CapsuleFitParams) -> f32 {
    let mut distances = Vec::new();

    for p in points {
        let diff = *p - axis.mean;
        let proj = diff.dot(axis.axis);
        let projection = axis.mean + axis.axis * proj;
        let dist = (p - projection).magnitude();
        distances.push(dist);
    }

    distances.sort_by(f32::total_cmp);

    let idx =
        ((distances.len() - 1) as f32 * params.radius_percentile.clamp(0.0, 1.0)).round() as usize;
    distances[idx] * params.radius_scale
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spring_bone::{SpringChain, SpringColliderGroup, SpringJointParam};
    use cgmath::{Matrix4, SquareMatrix, Vector4};

    fn make_cylinder_points(radius: f32, length: f32, samples: usize) -> Vec<Vector3<f32>> {
        let mut points = Vec::new();
        for i in 0..samples {
            let angle = 2.0 * std::f32::consts::PI * (i as f32) / (samples as f32);
            let x = length * 0.5;
            let y = radius * angle.cos();
            let z = radius * angle.sin();
            points.push(Vector3::new(x, y, z));
        }
        for i in 0..samples {
            let angle = 2.0 * std::f32::consts::PI * (i as f32) / (samples as f32);
            let x = -length * 0.5;
            let y = radius * angle.cos();
            let z = radius * angle.sin();
            points.push(Vector3::new(x, y, z));
        }
        points
    }

    fn make_test_skin(
        local_points: &[Vector3<f32>],
        inverse_bind_pose: Matrix4<f32>,
    ) -> (SkinData, Skeleton) {
        let bind_pose = inverse_bind_pose.invert().unwrap();
        let world_positions: Vec<Vector3<f32>> = local_points
            .iter()
            .map(|p| {
                let transformed = bind_pose * Vector4::new(p.x, p.y, p.z, 1.0);
                Vector3::new(transformed.x, transformed.y, transformed.z)
            })
            .collect();
        make_test_skin_from_world(&world_positions, inverse_bind_pose)
    }

    fn make_test_skin_from_world(
        world_positions: &[Vector3<f32>],
        inverse_bind_pose: Matrix4<f32>,
    ) -> (SkinData, Skeleton) {
        let mut skeleton = Skeleton::new("test");
        let b0 = skeleton.add_bone("bone0", None);
        let _b1 = skeleton.add_bone("bone1", Some(b0));

        *skeleton.get_bone_mut(b0).unwrap() = Bone {
            id: b0,
            name: "bone0".to_string(),
            parent_id: None,
            children: vec![_b1],
            local_transform: Matrix4::identity(),
            inverse_bind_pose,
            node_index: None,
        };

        let mut skin = SkinData::default();
        for p in world_positions {
            skin.base_positions.push(*p);
            skin.bone_indices.push(cgmath::Vector4::new(b0, 0, 0, 0));
            skin.bone_weights
                .push(cgmath::Vector4::new(1.0, 0.0, 0.0, 0.0));
            skin.base_normals.push(Vector3::new(0.0, 0.0, 1.0));
        }

        (skin, skeleton)
    }

    #[test]
    fn test_cylinder_fit() {
        let local_points = make_cylinder_points(0.5, 2.0, 32);
        let identity = Matrix4::identity();
        let (skin, skeleton) = make_test_skin(&local_points, identity);

        let params = CapsuleFitParams {
            min_weight: 0.1,
            radius_percentile: 0.5,
            radius_scale: 1.0,
            min_vertex_count: 4,
        };

        let capsules = fit_bone_capsules(&[&skin], &skeleton, &params);

        assert_eq!(capsules.len(), 1);
        let cap = &capsules[0];
        assert_eq!(cap.bone_id, 0);

        assert!((cap.radius - 0.5).abs() < 1e-3, "radius: {}", cap.radius);
        assert!((cap.head.x - (-0.5)).abs() < 1e-3, "head.x: {}", cap.head.x);
        assert!((cap.tail.x - 0.5).abs() < 1e-3, "tail.x: {}", cap.tail.x);
    }

    #[test]
    fn test_unassigned_bone_not_output() {
        let local_points = make_cylinder_points(0.5, 2.0, 8);
        let identity = Matrix4::identity();
        let (skin, skeleton) = make_test_skin(&local_points, identity);

        let params = CapsuleFitParams {
            min_weight: 0.1,
            radius_percentile: 0.5,
            radius_scale: 1.0,
            min_vertex_count: 4,
        };

        let capsules = fit_bone_capsules(&[&skin], &skeleton, &params);

        for cap in &capsules {
            assert_ne!(cap.bone_id, 1, "bone 1 should not be in output");
        }
    }

    #[test]
    fn test_inverse_bind_pose_translation() {
        let world_positions = make_cylinder_points(0.5, 2.0, 32);
        let inverse_bind_pose = Matrix4::from_translation(Vector3::new(-1.0, 0.0, 0.0));
        let (skin, skeleton) = make_test_skin_from_world(&world_positions, inverse_bind_pose);

        let params = CapsuleFitParams {
            min_weight: 0.1,
            radius_percentile: 0.5,
            radius_scale: 1.0,
            min_vertex_count: 4,
        };

        let capsules = fit_bone_capsules(&[&skin], &skeleton, &params);

        assert_eq!(capsules.len(), 1);
        let cap = &capsules[0];

        assert!((cap.head.x - (-1.5)).abs() < 1e-3, "head.x: {}", cap.head.x);
        assert!((cap.tail.x - (-0.5)).abs() < 1e-3, "tail.x: {}", cap.tail.x);
    }
    #[test]
    fn test_radius_scale() {
        let local_points = make_cylinder_points(0.5, 2.0, 32);
        let identity = Matrix4::identity();
        let (skin, skeleton) = make_test_skin(&local_points, identity);

        let params = CapsuleFitParams {
            min_weight: 0.1,
            radius_percentile: 0.5,
            radius_scale: 0.8,
            min_vertex_count: 4,
        };

        let capsules = fit_bone_capsules(&[&skin], &skeleton, &params);

        assert_eq!(capsules.len(), 1);
        let cap = &capsules[0];
        assert!((cap.radius - 0.4).abs() < 1e-3, "radius: {}", cap.radius);
    }

    #[test]
    fn test_two_skin_split() {
        let full_points = make_cylinder_points(0.5, 2.0, 32);
        let identity = Matrix4::identity();
        let (skin_full, skeleton) = make_test_skin(&full_points, identity);

        let params = CapsuleFitParams {
            min_weight: 0.1,
            radius_percentile: 0.5,
            radius_scale: 1.0,
            min_vertex_count: 4,
        };

        let capsules_full = fit_bone_capsules(&[&skin_full], &skeleton, &params);
        assert_eq!(capsules_full.len(), 1);
        let cap_full = &capsules_full[0];

        let left_points: Vec<Vector3<f32>> =
            full_points.iter().filter(|p| p.x < 0.0).cloned().collect();
        let right_points: Vec<Vector3<f32>> =
            full_points.iter().filter(|p| p.x >= 0.0).cloned().collect();

        let (skin_left, _) = make_test_skin(&left_points, identity);
        let (skin_right, _) = make_test_skin(&right_points, identity);

        let capsules_split = fit_bone_capsules(&[&skin_left, &skin_right], &skeleton, &params);
        assert_eq!(capsules_split.len(), 1);
        let cap_split = &capsules_split[0];

        assert!(
            (cap_full.head.x - cap_split.head.x).abs() < 1e-3,
            "head.x: full={} split={}",
            cap_full.head.x,
            cap_split.head.x
        );
        assert!(
            (cap_full.tail.x - cap_split.tail.x).abs() < 1e-3,
            "tail.x: full={} split={}",
            cap_full.tail.x,
            cap_split.tail.x
        );
        assert!(
            (cap_full.radius - cap_split.radius).abs() < 1e-3,
            "radius: full={} split={}",
            cap_full.radius,
            cap_split.radius
        );
    }

    #[test]
    fn test_replace_fitted_body_colliders_basic() {
        let mut setup = SpringBoneSetup::default();
        setup.chains.push(SpringChain {
            id: 0,
            name: "chain0".to_string(),
            joints: vec![SpringJointParam {
                bone_id: 0,
                ..Default::default()
            }],
            collider_group_ids: Vec::new(),
            center_bone_id: None,
            enabled: true,
        });
        setup.next_chain_id = 1;

        let capsules = vec![
            FittedCapsule {
                bone_id: 0,
                head: Vector3::new(0.0, 0.0, 0.0),
                tail: Vector3::new(0.0, 1.0, 0.0),
                radius: 0.1,
            },
            FittedCapsule {
                bone_id: 1,
                head: Vector3::new(0.0, 1.0, 0.0),
                tail: Vector3::new(0.0, 2.0, 0.0),
                radius: 0.08,
            },
        ];

        replace_fitted_body_colliders(&mut setup, &capsules);

        assert_eq!(setup.colliders.len(), 2);
        assert_eq!(setup.collider_groups.len(), 1);
        let group = &setup.collider_groups[0];
        assert_eq!(group.name, FITTED_BODY_GROUP_NAME);
        assert_eq!(group.collider_ids.len(), 2);
        assert_eq!(setup.chains[0].collider_group_ids.len(), 1);
        assert_eq!(setup.chains[0].collider_group_ids[0], group.id);
    }

    #[test]
    fn test_replace_fitted_body_colliders_idempotent() {
        let mut setup = SpringBoneSetup::default();
        setup.chains.push(SpringChain {
            id: 0,
            name: "chain0".to_string(),
            joints: vec![SpringJointParam {
                bone_id: 0,
                ..Default::default()
            }],
            collider_group_ids: Vec::new(),
            center_bone_id: None,
            enabled: true,
        });
        setup.next_chain_id = 1;

        let capsules = vec![
            FittedCapsule {
                bone_id: 0,
                head: Vector3::new(0.0, 0.0, 0.0),
                tail: Vector3::new(0.0, 1.0, 0.0),
                radius: 0.1,
            },
            FittedCapsule {
                bone_id: 1,
                head: Vector3::new(0.0, 1.0, 0.0),
                tail: Vector3::new(0.0, 2.0, 0.0),
                radius: 0.08,
            },
        ];

        replace_fitted_body_colliders(&mut setup, &capsules);
        let first_count = setup.colliders.len();
        let first_group_count = setup.collider_groups.len();
        let first_ref_count = setup.chains[0].collider_group_ids.len();

        replace_fitted_body_colliders(&mut setup, &capsules);

        assert_eq!(setup.colliders.len(), first_count);
        assert_eq!(setup.collider_groups.len(), first_group_count);
        assert_eq!(setup.chains[0].collider_group_ids.len(), first_ref_count);
    }

    #[test]
    fn test_replace_fitted_body_colliders_preserves_other_groups() {
        let mut setup = SpringBoneSetup::default();
        setup.chains.push(SpringChain {
            id: 0,
            name: "chain0".to_string(),
            joints: vec![SpringJointParam {
                bone_id: 0,
                ..Default::default()
            }],
            collider_group_ids: Vec::new(),
            center_bone_id: None,
            enabled: true,
        });
        setup.next_chain_id = 1;

        setup.colliders.push(SpringColliderDef {
            id: 0,
            bone_id: 2,
            offset: Vector3::new(1.0, 0.0, 0.0),
            shape: ColliderShape::Sphere { radius: 0.2 },
        });
        setup.next_collider_id = 1;
        setup.collider_groups.push(SpringColliderGroup {
            id: 0,
            name: "User Group".to_string(),
            collider_ids: vec![0],
        });
        setup.next_group_id = 1;
        setup.chains[0].collider_group_ids.push(0);

        let capsules = vec![FittedCapsule {
            bone_id: 0,
            head: Vector3::new(0.0, 0.0, 0.0),
            tail: Vector3::new(0.0, 1.0, 0.0),
            radius: 0.1,
        }];

        replace_fitted_body_colliders(&mut setup, &capsules);

        assert_eq!(setup.collider_groups.len(), 2);
        let user_group = setup
            .collider_groups
            .iter()
            .find(|g| g.name == "User Group")
            .expect("User Group should still exist");
        assert_eq!(user_group.collider_ids, vec![0]);
        let fitted_group = setup
            .collider_groups
            .iter()
            .find(|g| g.name == FITTED_BODY_GROUP_NAME)
            .expect("Fitted Body group should exist");
        assert_eq!(fitted_group.collider_ids.len(), 1);
        assert!(setup.chains[0].collider_group_ids.contains(&user_group.id));
        assert!(setup.chains[0]
            .collider_group_ids
            .contains(&fitted_group.id));
    }
}
