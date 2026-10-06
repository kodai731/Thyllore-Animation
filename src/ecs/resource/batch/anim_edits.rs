use std::path::PathBuf;

use thyllore_anim_core::editable::PropertyType;

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
    RoleKey {
        role: thyllore_avatar_core::humanoid::components::role::HumanoidRole,
        axis: RoleAxis,
        time: f32,
        value: f32,
    },
    KeyAtPlayhead {
        property_type: PropertyType,
    },
    TrimEnd {
        seconds: f32,
    },
    NewRoleClip {
        name: String,
    },
    Template {
        path: PathBuf,
    },
    Save {
        path: PathBuf,
    },
    CopilotExtend {
        role: thyllore_avatar_core::humanoid::components::role::HumanoidRole,
        axis: RoleAxis,
        time: f32,
        frames: usize,
    },
    Clear,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RoleAxis {
    RotationX,
    RotationY,
    RotationZ,
    TranslationX,
    TranslationY,
    TranslationZ,
}
