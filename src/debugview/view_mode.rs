use cgmath::Vector3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DebugViewMode {
    Final = 0,
    Position = 1,
    Normal = 2,
    ShadowMask = 3,
    NdotL = 4,
    LightDirection = 5,
}

impl Default for DebugViewMode {
    fn default() -> Self {
        DebugViewMode::Final
    }
}

impl DebugViewMode {
    pub fn as_int(&self) -> i32 {
        *self as i32
    }

    pub fn from_int(value: i32) -> Self {
        match value {
            0 => DebugViewMode::Final,
            1 => DebugViewMode::Position,
            2 => DebugViewMode::Normal,
            3 => DebugViewMode::ShadowMask,
            4 => DebugViewMode::NdotL,
            5 => DebugViewMode::LightDirection,
            _ => DebugViewMode::Final,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            DebugViewMode::Final => "Final (Lit + Shadow)",
            DebugViewMode::Position => "Position (World Space)",
            DebugViewMode::Normal => "Normal (World Space)",
            DebugViewMode::ShadowMask => "Shadow Mask",
            DebugViewMode::NdotL => "N dot L (Green=Lit, Red=Back)",
            DebugViewMode::LightDirection => "Light Direction",
        }
    }
}

#[derive(Clone, Debug)]
pub struct RayTracingDebugState {
    pub light_position: Vector3<f32>,
    pub debug_view_mode: DebugViewMode,
    pub shadow_strength: f32,
    pub shadow_normal_offset: f32,
    pub enable_distance_attenuation: bool,
}

impl Default for RayTracingDebugState {
    fn default() -> Self {
        Self {
            light_position: Vector3::new(5.0, 5.0, 5.0),
            debug_view_mode: DebugViewMode::Final,
            shadow_strength: 1.0,
            shadow_normal_offset: 0.5,
            enable_distance_attenuation: false,
        }
    }
}
