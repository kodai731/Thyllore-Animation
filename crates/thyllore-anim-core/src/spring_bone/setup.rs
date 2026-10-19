use std::sync::LazyLock;

use cgmath::Vector3;

use thyllore_scene_core::{RootPath, ScalarParam, SceneFields, UiParam};

use crate::BoneId;

pub type SpringChainId = u32;
pub type SpringColliderId = u32;
pub type SpringColliderGroupId = u32;

#[derive(Clone, Debug, SceneFields)]
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

impl Default for SpringJointParam {
    fn default() -> Self {
        Self {
            bone_id: 0,
            stiffness: 0.5,
            drag_force: 0.4,
            gravity_power: 1.0,
            gravity_dir: Vector3::new(0.0, -1.0, 0.0),
            hit_radius: 0.02,
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

#[derive(Clone, Debug, Default)]
pub struct SpringBoneSetup {
    pub chains: Vec<SpringChain>,
    pub colliders: Vec<SpringColliderDef>,
    pub collider_groups: Vec<SpringColliderGroup>,
    pub next_chain_id: SpringChainId,
    pub next_collider_id: SpringColliderId,
    pub next_group_id: SpringColliderGroupId,
    pub collision_mode: SpringCollisionMode,
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
