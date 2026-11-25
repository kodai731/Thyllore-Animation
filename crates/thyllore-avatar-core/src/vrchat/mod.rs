pub(crate) mod blink;
pub(crate) mod gesture;
pub(crate) mod rank;
pub(crate) mod rank_thresholds;
pub(crate) mod sidecar;
pub(crate) mod sidecar_spring;
pub(crate) mod viseme;

pub use blink::{find_blink_candidates, BlinkCandidates};
pub use gesture::{gesture_template_library, Gesture};
pub use rank::{rank_stats, PerformanceRank, Platform, RankItem, RankReport};
pub use rank_thresholds::{default_thresholds, PlatformThresholds, RankBounds, RankThresholds};
pub use sidecar::{
    build_hybrid_bone_map, build_sidecar, count_viseme_channels, read_sidecar_json, sidecar_path,
    write_sidecar_json, AvatarSidecar, SidecarBlink, SidecarExpression, SidecarHiddenTriangles,
    SidecarInput, SidecarSubmeshRole, SIDECAR_SCHEMA_VERSION,
};
pub use sidecar_spring::{
    build_sidecar_spring, map_sidecar_colliders, rest_frames_from_globals,
    spring_setup_from_sidecar, spring_setup_with_collider_ids, BoneRestFrame, ColliderSpace,
    SidecarChainOrigin, SidecarCollider, SidecarColliderGroup, SidecarColliderOrigin,
    SidecarColliderShape, SidecarCollisionMode, SidecarJointForce, SidecarSpring,
    SidecarSpringChain, SidecarSpringJoint,
};
pub use viseme::{find_missing_visemes, VISEME_CHANNELS};
