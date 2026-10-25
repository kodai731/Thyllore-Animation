use std::sync::LazyLock;

use cgmath::{EuclideanSpace, InnerSpace, Point3, Transform, Vector3};

use thyllore_math_core::{fit_principal_axis, PrincipalAxis};
use thyllore_model_core::{Bone, BoneId, Skeleton, SkinData};
use thyllore_scene_core::{RootPath, ScalarParam, SceneFields, UiParam};

use super::setup::{
    ColliderShape, SpringBoneSetup, SpringColliderDef, SpringColliderGroup, SpringColliderGroupId,
    SpringColliderId,
};

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

pub static CAPSULE_FIT_SCALAR_PARAMS: LazyLock<Vec<ScalarParam<CapsuleFitParams>>> =
    LazyLock::new(|| {
        let mut out: Vec<ScalarParam<CapsuleFitParams>> = Vec::new();
        CapsuleFitParams::collect_scalars::<CapsuleFitParams, RootPath<CapsuleFitParams>>(
            "", &mut out,
        );
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
        setup.colliders.push(capsule_collider(id, cap));
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

/// Puts the capsule into the Fitted Body group, replacing the group's collider on the same bone.
pub fn ensure_fitted_collider(
    setup: &mut SpringBoneSetup,
    capsule: &FittedCapsule,
) -> SpringColliderGroupId {
    let group_id = match setup
        .collider_groups
        .iter()
        .position(|g| g.name == FITTED_BODY_GROUP_NAME)
    {
        Some(group_pos) => {
            remove_group_colliders_on_bone(setup, group_pos, capsule.bone_id);
            setup.collider_groups[group_pos].id
        }
        None => add_new_colliders_and_group(setup, &[]),
    };

    let collider_id = setup.next_collider_id;
    setup.next_collider_id += 1;
    setup.colliders.push(capsule_collider(collider_id, capsule));
    let group = setup
        .collider_groups
        .iter_mut()
        .find(|g| g.id == group_id)
        .expect("the fitted group was just found or created");
    group.collider_ids.push(collider_id);
    group_id
}

fn remove_group_colliders_on_bone(setup: &mut SpringBoneSetup, group_pos: usize, bone_id: BoneId) {
    let group = &mut setup.collider_groups[group_pos];
    let on_bone: Vec<SpringColliderId> = group
        .collider_ids
        .iter()
        .copied()
        .filter(|cid| {
            setup
                .colliders
                .iter()
                .any(|c| c.id == *cid && c.bone_id == bone_id)
        })
        .collect();
    group.collider_ids.retain(|cid| !on_bone.contains(cid));
    setup.colliders.retain(|c| !on_bone.contains(&c.id));
}

fn capsule_collider(id: SpringColliderId, cap: &FittedCapsule) -> SpringColliderDef {
    SpringColliderDef {
        id,
        bone_id: cap.bone_id,
        offset: cap.head,
        shape: ColliderShape::Capsule {
            radius: cap.radius,
            tail: cap.tail,
        },
    }
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
    skeleton
        .bones
        .iter()
        .filter_map(|bone| fit_bone_capsule(skins, bone, params))
        .collect()
}

pub fn fit_bone_capsule(
    skins: &[&SkinData],
    bone: &Bone,
    params: &CapsuleFitParams,
) -> Option<FittedCapsule> {
    let local_points = collect_local_points(skins, bone, params.min_weight);
    if local_points.len() < params.min_vertex_count as usize {
        return None;
    }

    let pa = fit_principal_axis(&local_points)?;
    let (t_min, t_max) = project_to_axis(&local_points, &pa);
    let radius = compute_radius(&local_points, &pa, params);

    let mut head = pa.mean + pa.axis * (t_min + radius);
    let mut tail = pa.mean + pa.axis * (t_max - radius);
    if t_max - t_min < 2.0 * radius {
        let mid = pa.mean + pa.axis * (t_min + t_max) * 0.5;
        head = mid;
        tail = mid;
    }

    Some(FittedCapsule {
        bone_id: bone.id,
        head,
        tail,
        radius,
    })
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
mod tests;
