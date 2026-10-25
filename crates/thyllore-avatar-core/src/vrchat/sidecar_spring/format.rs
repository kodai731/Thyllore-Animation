use serde::{Deserialize, Serialize};
use thyllore_anim_core::spring_bone::ChainRole;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SidecarCollisionMode {
    #[default]
    VrmCompatible,
    SegmentExtended,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SidecarForceModel {
    Vrm,
    #[default]
    RestRelative,
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
    #[serde(default)]
    pub force_model: SidecarForceModel,
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SidecarSpringJoint {
    pub bone: String,
    pub stiffness: f32,
    pub drag_force: f32,
    pub gravity_power: f32,
    pub gravity_dir: [f32; 3],
    pub hit_radius: f32,
}
