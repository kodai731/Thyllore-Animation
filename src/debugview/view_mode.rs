use crate::app::data::LightMoveTarget;
use crate::app::GUIData;
use crate::log;
use crate::scene::CubeModel;
use cgmath::{InnerSpace, Vector3};

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
    pub cube_model: Option<CubeModel>,
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
            cube_model: None,
        }
    }
}

impl RayTracingDebugState {
    pub fn update_light_position(
        &mut self,
        all_positions: &[Vector3<f32>],
        camera_position: Vector3<f32>,
        move_light_to: LightMoveTarget,
    ) {
        crate::log!("========================================");
        crate::log!("LIGHT MOVE BUTTON PRESSED: {:?}", move_light_to);
        crate::log!("========================================");

        if all_positions.is_empty() {
            crate::log!("WARNING: No model positions found!");
            return;
        }

        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        let mut min_z = f32::MAX;
        let mut max_z = f32::MIN;

        for pos in all_positions.iter() {
            min_x = min_x.min(pos.x);
            max_x = max_x.max(pos.x);
            min_y = min_y.min(pos.y);
            max_y = max_y.max(pos.y);
            min_z = min_z.min(pos.z);
            max_z = max_z.max(pos.z);
        }

        let size_x = (max_x - min_x).abs();
        let size_y = (max_y - min_y).abs();
        let size_z = (max_z - min_z).abs();
        let model_size = (size_x + size_y + size_z) / 3.0;

        let offset = 2.0;
        let current_pos = self.light_position;
        let new_light_pos = match move_light_to {
            LightMoveTarget::XMin => Vector3::new(min_x - offset, current_pos.y, current_pos.z),
            LightMoveTarget::XMax => Vector3::new(max_x + offset, current_pos.y, current_pos.z),
            LightMoveTarget::YMin => Vector3::new(current_pos.x, min_y - offset, current_pos.z),
            LightMoveTarget::YMax => Vector3::new(current_pos.x, max_y + offset, current_pos.z),
            LightMoveTarget::ZMin => Vector3::new(current_pos.x, current_pos.y, min_z - offset),
            LightMoveTarget::ZMax => Vector3::new(current_pos.x, current_pos.y, max_z + offset),
            LightMoveTarget::None => current_pos,
        };

        self.shadow_normal_offset = (model_size * 0.005).max(0.5);

        crate::log!("=== LIGHT POSITION DEBUG ===");
        crate::log!(
            "Model size: {:.2}, Shadow normal offset: {:.2}",
            model_size,
            self.shadow_normal_offset
        );
        crate::log!(
            "Model bounds: X[{:.2}, {:.2}], Y[{:.2}, {:.2}], Z[{:.2}, {:.2}]",
            min_x,
            max_x,
            min_y,
            max_y,
            min_z,
            max_z
        );
        crate::log!(
            "Model center: ({:.2}, {:.2}, {:.2})",
            (min_x + max_x) / 2.0,
            (min_y + max_y) / 2.0,
            (min_z + max_z) / 2.0
        );
        crate::log!(
            "Calculated light position: ({:.2}, {:.2}, {:.2})",
            new_light_pos.x,
            new_light_pos.y,
            new_light_pos.z
        );
        crate::log!(
            "CAMERA position: ({:.2}, {:.2}, {:.2})",
            camera_position.x,
            camera_position.y,
            camera_position.z
        );

        let closest_vertex = all_positions.iter().min_by(|a, b| {
            let dist_a = (new_light_pos - **a).magnitude();
            let dist_b = (new_light_pos - **b).magnitude();
            dist_a.partial_cmp(&dist_b).unwrap()
        });
        let farthest_vertex = all_positions.iter().max_by(|a, b| {
            let dist_a = (new_light_pos - **a).magnitude();
            let dist_b = (new_light_pos - **b).magnitude();
            dist_a.partial_cmp(&dist_b).unwrap()
        });

        if let Some(closest) = closest_vertex {
            let dist = (new_light_pos - *closest).magnitude();
            crate::log!(
                "Closest vertex to light: ({:.2}, {:.2}, {:.2}), distance: {:.2}",
                closest.x,
                closest.y,
                closest.z,
                dist
            );
        }
        if let Some(farthest) = farthest_vertex {
            let dist = (new_light_pos - *farthest).magnitude();
            crate::log!(
                "Farthest vertex from light: ({:.2}, {:.2}, {:.2}), distance: {:.2}",
                farthest.x,
                farthest.y,
                farthest.z,
                dist
            );
        }

        if move_light_to == LightMoveTarget::XMax {
            crate::log!("XMax: Light should be to the RIGHT of all vertices");
            crate::log!(
                "  Light X: {:.2}, Model X range: [{:.2}, {:.2}]",
                new_light_pos.x,
                min_x,
                max_x
            );
            if new_light_pos.x <= max_x {
                crate::log!(
                    "  WARNING: Light X ({:.2}) is NOT greater than max X ({:.2})!",
                    new_light_pos.x,
                    max_x
                );
            } else {
                crate::log!(
                    "  OK: Light X ({:.2}) > max X ({:.2})",
                    new_light_pos.x,
                    max_x
                );
            }
        }

        self.light_position = new_light_pos;

        crate::log!(
            "Light position SET in rt_debug_state: ({:.2}, {:.2}, {:.2})",
            self.light_position.x,
            self.light_position.y,
            self.light_position.z
        );
        crate::log!("(light_gizmo_data will be synced later in this frame)");
        crate::log!("========================================");
    }
    pub fn should_load_cube(&self, gui_data: &GUIData) -> bool {
        gui_data.load_cube || self.cube_size_changed
    }

    pub fn finish_cube_load(&mut self) {
        self.cube_size_changed = false;
    }
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
                    log!(
                        "get_cube_top: cube_size={:.2}, actual_top=({:.2},{:.2},{:.2})",
                        self.cube_size,
                        top.x,
                        top.y,
                        top.z
                    );
                } else {
                    log!(
                        "get_cube_top: cube_size={:.2}, actual_top=None",
                        self.cube_size
                    );
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
