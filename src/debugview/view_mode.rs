use cgmath::Vector3;
use crate::log;
use crate::scene::CubeModel;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DebugViewMode {
    Final = 0,
    Position = 1,
    Normal = 2,
    ShadowMask = 3,
    NdotL = 4,
    LightDirection = 5,
    ViewDepth = 6,
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
            6 => DebugViewMode::ViewDepth,
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
            DebugViewMode::ViewDepth => "View Depth (R=billboard, G=gbuffer)",
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
    pub cube_size: f32,
    pub cube_size_changed: bool,
    pub actual_cube_top: Option<Vector3<f32>>,
}

impl Default for RayTracingDebugState {
    fn default() -> Self {
        Self {
            light_position: Vector3::new(5.0, 5.0, 5.0),
            debug_view_mode: DebugViewMode::Final,
            shadow_strength: 1.0,
            shadow_normal_offset: 0.5,
            enable_distance_attenuation: false,
            cube_size: 100.0,
            cube_size_changed: false,
            actual_cube_top: None,
        }
    }
}

impl RayTracingDebugState {
    pub fn set_cube_size(&mut self, size: f32) {
        self.cube_size = size;
        self.cube_size_changed = true;
    }

    pub fn set_actual_cube_top(&mut self, size: f32, position: [f32; 3]) {
        let top_y = position[1] + size / 2.0;
        self.actual_cube_top = Some(Vector3::new(position[0], top_y, position[2]));
    }

    pub fn get_cube_top(&self) -> Option<Vector3<f32>> {
        static mut LOG_COUNTER: u32 = 0;
        unsafe {
            LOG_COUNTER += 1;
            if LOG_COUNTER % 60 == 1 {
                if let Some(top) = self.actual_cube_top {
                    log!("get_cube_top: cube_size={:.2}, actual_top=({:.2},{:.2},{:.2})",
                        self.cube_size, top.x, top.y, top.z);
                } else {
                    log!("get_cube_top: cube_size={:.2}, actual_top=None", self.cube_size);
                }
            }
        }
        self.actual_cube_top
    }
}

#[derive(Clone, Debug, Default)]
pub struct DebugViewData {
    pub cube_model: Option<CubeModel>,
}
