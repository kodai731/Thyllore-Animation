mod copilot;
pub mod degrade;
pub mod feedback;
mod mode;
pub mod model_path;
pub mod onnx_runtime;

#[cfg(feature = "python")]
mod pybindings;

pub use copilot::*;
pub use mode::CurveCopilotMode;

pub use thyllore_model_core::{Bone, BoneId, Skeleton, SkeletonId};
