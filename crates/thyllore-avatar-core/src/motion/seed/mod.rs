pub(crate) mod components;
pub(crate) mod systems;

pub use components::motion_spec::{MotionSide, MotionSpec};
pub use components::pose_table::{
    BodyRegion, MotionDef, MotionPose, NamedPose, PoseEntry, PoseSide, PoseTable, RoleKey,
    RotationAxis, SettleDef, SETTLE_MAX_FRAMES,
};
pub use systems::compose_motion::{compose_motion, settle_requests, SeedKey, SettleRequest};
