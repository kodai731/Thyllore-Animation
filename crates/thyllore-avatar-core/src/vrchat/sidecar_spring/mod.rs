mod collider_space;
mod export;
mod format;
mod frame;
mod import;

#[cfg(test)]
mod tests;

pub use collider_space::{map_sidecar_colliders, ColliderSpace};
pub use export::build_sidecar_spring;
pub use format::{
    SidecarChainOrigin, SidecarCollider, SidecarColliderGroup, SidecarColliderOrigin,
    SidecarColliderShape, SidecarCollisionMode, SidecarJointForce, SidecarSpring,
    SidecarSpringChain, SidecarSpringJoint,
};
pub use frame::{rest_frames_from_globals, BoneRestFrame};
pub use import::spring_setup_from_sidecar;
