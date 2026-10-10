use std::path::PathBuf;

use thyllore_anim_core::editable::PropertyType;
use thyllore_avatar_core::motion::seed::components::motion_spec::MotionSpec;

#[derive(Clone, Debug, PartialEq)]
pub enum BatchAnimEdit {
    DebugKeys {
        seed: u64,
    },
    Key {
        property_type: PropertyType,
        time: f32,
        value: f32,
    },
    BoneKey {
        bone_name: String,
        axis: BoneAxis,
        time: f32,
        value: f32,
    },
    KeyAtPlayhead {
        property_type: PropertyType,
    },
    TrimEnd {
        seconds: f32,
    },
    NewClip {
        name: String,
    },
    Template {
        path: PathBuf,
    },
    Save {
        path: PathBuf,
    },
    Compose {
        spec: MotionSpec,
    },
    CopilotExtend {
        bone_name: String,
        axis: BoneAxis,
        time: f32,
        frames: usize,
    },
    Clear,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BoneAxis {
    RotationX,
    RotationY,
    RotationZ,
    TranslationX,
    TranslationY,
    TranslationZ,
}
