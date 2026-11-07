use std::sync::LazyLock;

use glam::Vec3;

use thyllore_math_core::{distance_between_segments, fit_principal_axis, PrincipalAxis};
use thyllore_model_core::{Bone, BoneId, Skeleton, SkinData};
use thyllore_scene_core::{RootPath, ScalarParam, SceneFields, UiParam};

use super::setup::{
    ColliderGroupOrigin, ColliderShape, SpringBoneSetup, SpringColliderDef, SpringColliderGroup,
    SpringColliderGroupId, SpringColliderId,
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
    pub head: Vec3,
    pub tail: Vec3,
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
        .position(|g| g.origin == ColliderGroupOrigin::Fitted)
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
        origin: ColliderGroupOrigin::Fitted,
    });
    group_id
}

/// Whether the Fitted Body group already holds a collider on the bone.
pub fn fitted_collider_exists_on(setup: &SpringBoneSetup, bone_id: BoneId) -> bool {
    let Some(group) = setup
        .collider_groups
        .iter()
        .find(|g| g.origin == ColliderGroupOrigin::Fitted)
    else {
        return false;
    };
    setup
        .colliders
        .iter()
        .any(|c| c.bone_id == bone_id && group.collider_ids.contains(&c.id))
}

/// Puts the capsule into the Fitted Body group, replacing the group's collider on the same bone.
pub fn ensure_fitted_collider(
    setup: &mut SpringBoneSetup,
    capsule: &FittedCapsule,
) -> SpringColliderGroupId {
    let group_id = match setup
        .collider_groups
        .iter()
        .position(|g| g.origin == ColliderGroupOrigin::Fitted)
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

/// The capsules of the bones whose weighted skin lies along the bone: a fit that lands beside its
/// bone (a patch of skin, not a limb) is dropped.
pub fn fit_bone_capsules(
    skins: &[&SkinData],
    skeleton: &Skeleton,
    params: &CapsuleFitParams,
) -> Vec<FittedCapsule> {
    skeleton
        .bones
        .iter()
        .filter_map(|bone| fit_bone_capsule(skins, bone, params))
        .filter(|capsule| {
            let (head, tail) = bone_segment_local(skeleton, capsule.bone_id);
            lies_along_segment(capsule, head, tail)
        })
        .collect()
}

/// Whether the capsule's axis comes within its radius of the segment `head`-`tail`.
pub fn lies_along_segment(capsule: &FittedCapsule, head: Vec3, tail: Vec3) -> bool {
    distance_between_segments(capsule.head, capsule.tail, head, tail) <= capsule.radius
}

/// The bone from its head to its first child's head in the bone's bind space; a leaf bone is its head alone.
pub fn bone_segment_local(skeleton: &Skeleton, bone_id: BoneId) -> (Vec3, Vec3) {
    let head = Vec3::new(0.0, 0.0, 0.0);
    let Some(bone) = skeleton.get_bone(bone_id) else {
        return (head, head);
    };
    let tail = bone
        .children
        .first()
        .and_then(|&child| skeleton.get_bone(child))
        .and_then(|child| child.inverse_bind_pose.try_inverse())
        .map(|child_bind| {
            bone.inverse_bind_pose
                .transform_point3(child_bind.w_axis.truncate())
        })
        .unwrap_or(head);
    (head, tail)
}

pub fn fit_bone_capsule(
    skins: &[&SkinData],
    bone: &Bone,
    params: &CapsuleFitParams,
) -> Option<FittedCapsule> {
    let local_points = bone_local_points(skins, bone, params);
    let pa = fit_principal_axis(&local_points)?;
    fit_capsule_along(bone.id, &local_points, pa, params)
}

/// The capsule of `points` (the bone's bind space) on the line `axis` (a point and a unit
/// direction); points too short along it for two end caps give a sphere at their middle.
pub fn fit_capsule_along(
    bone_id: BoneId,
    points: &[Vec3],
    axis: PrincipalAxis,
    params: &CapsuleFitParams,
) -> Option<FittedCapsule> {
    let pa = axis;
    let (t_min, t_max) = project_to_axis(points, &pa);
    let radius = radius_around_axis(points, pa.mean, pa.axis, params)?;

    let mut head = pa.mean + pa.axis * (t_min + radius);
    let mut tail = pa.mean + pa.axis * (t_max - radius);
    if t_max - t_min < 2.0 * radius {
        let mid = pa.mean + pa.axis * (t_min + t_max) * 0.5;
        head = mid;
        tail = mid;
    }

    Some(FittedCapsule {
        bone_id,
        head,
        tail,
        radius,
    })
}

/// The skin positions the bone drives, in the bone's bind space; empty below `min_vertex_count`.
pub fn bone_local_points(skins: &[&SkinData], bone: &Bone, params: &CapsuleFitParams) -> Vec<Vec3> {
    let local_points: Vec<Vec3> = weighted_positions(skins, bone.id, params.min_weight)
        .into_iter()
        .map(|position| bone.inverse_bind_pose.transform_point3(position))
        .collect();
    if local_points.len() < params.min_vertex_count as usize {
        return Vec::new();
    }
    local_points
}

/// The skin positions (model space) the bone drives with at least `min_weight`.
pub fn weighted_positions(skins: &[&SkinData], bone_id: BoneId, min_weight: f32) -> Vec<Vec3> {
    let mut points = Vec::new();

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
                .any(|(idx, weight)| *idx == bone_id && *weight >= min_weight)
            {
                points.push(*pos);
            }
        }
    }

    points
}

fn project_to_axis(points: &[Vec3], pa: &PrincipalAxis) -> (f32, f32) {
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

/// The capsule radius the params pick from the distances of the points to the line through
/// `origin` along the unit `direction`; None without points.
pub fn radius_around_axis(
    points: &[Vec3],
    origin: Vec3,
    direction: Vec3,
    params: &CapsuleFitParams,
) -> Option<f32> {
    let mut distances: Vec<f32> = points
        .iter()
        .map(|p| {
            let diff = *p - origin;
            let projection = origin + direction * diff.dot(direction);
            (p - projection).length()
        })
        .collect();
    if distances.is_empty() {
        return None;
    }

    distances.sort_by(f32::total_cmp);
    let idx =
        ((distances.len() - 1) as f32 * params.radius_percentile.clamp(0.0, 1.0)).round() as usize;
    Some(distances[idx] * params.radius_scale)
}

#[cfg(test)]
mod tests;
