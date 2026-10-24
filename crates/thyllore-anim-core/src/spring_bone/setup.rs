use std::sync::LazyLock;

use cgmath::Vector3;

use thyllore_scene_core::{RootPath, ScalarParam, SceneFields, UiParam};

use super::origin::ChainOrigin;
use crate::BoneId;

pub type SpringChainId = u32;
pub type SpringColliderId = u32;
pub type SpringColliderGroupId = u32;

#[derive(Clone, Debug, PartialEq, SceneFields)]
#[params(tag = thyllore_scene_core::SingleOwner, owner = Component)]
pub struct SpringJointParam {
    pub bone_id: BoneId,
    #[persist(ui(min = 0.0, max = 4.0))]
    pub stiffness: f32,
    #[persist(ui(min = 0.0, max = 1.0))]
    pub drag_force: f32,
    #[persist(ui(min = 0.0, max = 2.0))]
    pub gravity_power: f32,
    #[persist(ui(kind = Offset, min = -1.0, max = 1.0))]
    pub gravity_dir: Vector3<f32>,
    #[persist(ui(min = 0.0, max = 0.5, format = "%.3f"))]
    pub hit_radius: f32,
}

pub const DEFAULT_SIMULATION_STEP: f32 = 1.0 / 30.0;
pub const EARTH_GRAVITY_M_PER_S2: f32 = 9.80665;
const DEFAULT_DRAG_FORCE: f32 = 0.4;

/// The strand lag that defines the hair material: a strand of this length trails its root moving at this speed by this angle.
pub const STRAND_LAG_REFERENCE_SPEED_M_PER_S: f32 = 0.5;
pub const STRAND_LAG_REFERENCE_LENGTH_M: f32 = 0.1;
pub const STRAND_LAG_REFERENCE_ANGLE_DEG: f32 = 20.0;

/// Bending relaxation `D` [m²/s]: a strand of length `L` returns at the rate `D / L²`, so its lag is `v·L / D`.
pub fn strand_bending_relaxation() -> f32 {
    STRAND_LAG_REFERENCE_SPEED_M_PER_S * STRAND_LAG_REFERENCE_LENGTH_M
        / STRAND_LAG_REFERENCE_ANGLE_DEG.to_radians().tan()
}

impl Default for SpringJointParam {
    fn default() -> Self {
        Self {
            bone_id: 0,
            stiffness: strand_bending_relaxation(),
            drag_force: DEFAULT_DRAG_FORCE,
            gravity_power: 1.0,
            gravity_dir: Vector3::new(0.0, -1.0, 0.0),
            hit_radius: 0.0,
        }
    }
}

pub static SPRING_JOINT_SCALAR_PARAMS: LazyLock<Vec<ScalarParam<SpringJointParam>>> =
    LazyLock::new(|| {
        let mut out: Vec<ScalarParam<SpringJointParam>> = Vec::new();
        SpringJointParam::collect_scalars::<SpringJointParam, RootPath<SpringJointParam>>(
            "", &mut out,
        );
        out
    });

pub static SPRING_JOINT_UI_PARAMS: LazyLock<Vec<UiParam>> = LazyLock::new(|| {
    let mut out: Vec<UiParam> = Vec::new();
    SpringJointParam::collect_ui("", "", &mut out);
    out
});

#[derive(Clone, Debug)]
pub struct SpringChain {
    pub id: SpringChainId,
    pub name: String,
    pub joints: Vec<SpringJointParam>,
    pub collider_group_ids: Vec<SpringColliderGroupId>,
    pub center_bone_id: Option<BoneId>,
    pub enabled: bool,
    pub origin: ChainOrigin,
}

impl Default for SpringChain {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            joints: Vec::new(),
            collider_group_ids: Vec::new(),
            center_bone_id: None,
            enabled: true,
            origin: ChainOrigin::Authored,
        }
    }
}

#[derive(Clone, Debug)]
pub enum ColliderShape {
    Sphere { radius: f32 },
    Capsule { radius: f32, tail: Vector3<f32> },
}

impl ColliderShape {
    pub fn default_sphere() -> Self {
        ColliderShape::Sphere { radius: 0.1 }
    }

    pub fn default_capsule() -> Self {
        ColliderShape::Capsule {
            radius: 0.1,
            tail: Vector3::new(0.0, 0.1, 0.0),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SpringColliderDef {
    pub id: SpringColliderId,
    pub bone_id: BoneId,
    pub offset: Vector3<f32>,
    pub shape: ColliderShape,
}

#[derive(Clone, Debug)]
pub struct SpringColliderGroup {
    pub id: SpringColliderGroupId,
    pub name: String,
    pub collider_ids: Vec<SpringColliderId>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpringCollisionMode {
    #[default]
    VrmCompatible,
    SegmentExtended,
}

/// How the joint forces are evaluated: the VRM formula, or forces that vanish in the rest pose.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpringForceModel {
    Vrm,
    #[default]
    RestRelative,
}

#[derive(Clone, Debug, Default)]
pub struct SpringBoneSetup {
    pub chains: Vec<SpringChain>,
    pub colliders: Vec<SpringColliderDef>,
    pub collider_groups: Vec<SpringColliderGroup>,
    pub next_chain_id: SpringChainId,
    pub next_collider_id: SpringColliderId,
    pub next_group_id: SpringColliderGroupId,
    pub collision_mode: SpringCollisionMode,
    pub force_model: SpringForceModel,
}

pub fn set_chain_collider_groups(
    setup: &mut SpringBoneSetup,
    chain_ids: &[SpringChainId],
    group_ids: &[SpringColliderGroupId],
) {
    let known: Vec<SpringColliderGroupId> = group_ids
        .iter()
        .copied()
        .filter(|id| setup.collider_groups.iter().any(|group| group.id == *id))
        .collect();
    for chain in &mut setup.chains {
        if chain_ids.contains(&chain.id) {
            chain.collider_group_ids = known.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joint_ui_params_follow_declaration_order() {
        let names: Vec<&str> = SPRING_JOINT_UI_PARAMS
            .iter()
            .map(|p| p.name.as_ref())
            .collect();
        assert_eq!(
            names,
            [
                "stiffness",
                "drag_force",
                "gravity_power",
                "gravity_dir",
                "hit_radius"
            ]
        );
    }
}
