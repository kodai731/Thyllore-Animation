use serde::{Deserialize, Serialize};
use thyllore_anim_core::spring_bone::{
    ChainRole, JointForce, RestRelativeJointForce, VrmJointForce,
};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SidecarCollisionMode {
    #[default]
    VrmCompatible,
    SegmentExtended,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SidecarColliderShape {
    Sphere,
    Capsule,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SidecarColliderOrigin {
    Authored,
    Fitted,
}

/// The spring section of the avatar sidecar: bones are referenced by name so the file survives re-export.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SidecarSpring {
    pub collision_mode: SidecarCollisionMode,
    pub colliders: Vec<SidecarCollider>,
    pub collider_groups: Vec<SidecarColliderGroup>,
    pub chains: Vec<SidecarSpringChain>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SidecarCollider {
    pub name: String,
    pub bone: String,
    pub offset: [f32; 3],
    pub shape: SidecarColliderShape,
    pub radius: f32,
    pub tail: Option<[f32; 3]>,
    pub origin: SidecarColliderOrigin,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SidecarColliderGroup {
    pub name: String,
    pub colliders: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SidecarSpringChain {
    pub name: String,
    pub center: Option<String>,
    pub collider_groups: Vec<String>,
    pub joints: Vec<SidecarSpringJoint>,
    /// The bone of the appended VRM tail joint, which is not a simulated joint of the engine chain
    #[serde(default)]
    pub tail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<SidecarChainOrigin>,
}

/// The submesh a generated chain was placed on; absent for hand-authored chains.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SidecarChainOrigin {
    pub mesh: String,
    #[serde(default)]
    pub material: Option<String>,
    pub role: ChainRole,
}

/// The joint's force model and its two magnitudes in that model's units.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "model", rename_all = "snake_case")]
pub enum SidecarJointForce {
    /// VRM 1.0: both in metres per second
    Vrm { stiffness: f32, gravity_power: f32 },
    /// Zero in the rest pose: `bending_relaxation` in m²/s, `gravity_scale` a multiple of Earth gravity
    RestRelative {
        bending_relaxation: f32,
        gravity_scale: f32,
    },
}

impl From<JointForce> for SidecarJointForce {
    fn from(force: JointForce) -> Self {
        match force {
            JointForce::Vrm(force) => SidecarJointForce::Vrm {
                stiffness: force.stiffness,
                gravity_power: force.gravity_power,
            },
            JointForce::RestRelative(force) => SidecarJointForce::RestRelative {
                bending_relaxation: force.bending_relaxation,
                gravity_scale: force.gravity_scale,
            },
        }
    }
}

impl From<SidecarJointForce> for JointForce {
    fn from(force: SidecarJointForce) -> Self {
        match force {
            SidecarJointForce::Vrm {
                stiffness,
                gravity_power,
            } => JointForce::Vrm(VrmJointForce {
                stiffness,
                gravity_power,
            }),
            SidecarJointForce::RestRelative {
                bending_relaxation,
                gravity_scale,
            } => JointForce::RestRelative(RestRelativeJointForce {
                bending_relaxation,
                gravity_scale,
            }),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SidecarSpringJoint {
    pub bone: String,
    pub force: SidecarJointForce,
    pub drag_force: f32,
    pub gravity_dir: [f32; 3],
    pub hit_radius: f32,
}
