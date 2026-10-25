pub mod avatar_setup_bridge;
pub mod chain_gen_bridge;
pub mod chain_register_bridge;
pub mod collider_bridge;
pub mod copilot;
#[cfg(feature = "debug-log")]
pub mod debug_log;
pub mod degrade;
pub mod feedback;
pub mod mode;
pub mod model_path;
pub mod param_rows;
pub mod spring_edit_bridge;
pub mod submesh_bridge;

#[cfg(feature = "python")]
mod pybindings;

pub use copilot::*;
pub use mode::CurveCopilotMode;

pub use thyllore_model_core::{Bone, BoneId, Skeleton, SkeletonId};
