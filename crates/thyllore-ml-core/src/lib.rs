pub mod copilot;
pub mod degrade;
pub mod feedback;
pub mod mode;
pub mod model_path;

#[cfg(feature = "python")]
mod pybindings;

pub use copilot::*;
pub use mode::CurveCopilotMode;

pub use thyllore_model_core::{Bone, BoneId, Skeleton, SkeletonId};
