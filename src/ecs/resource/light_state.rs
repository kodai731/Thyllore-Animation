use crate::app::data::LightMoveTarget;
use cgmath::{InnerSpace, Vector3};

#[derive(Clone, Debug)]
pub struct LightState {
    pub light_position: Vector3<f32>,
    pub shadow_strength: f32,
    pub shadow_normal_offset: f32,
    pub enable_distance_attenuation: bool,
}

impl Default for LightState {
    fn default() -> Self {
        Self {
            light_position: Vector3::new(1.0, 1.0, 1.0),
            shadow_strength: 1.0,
            shadow_normal_offset: 0.5,
            enable_distance_attenuation: false,
        }
    }
}

impl LightState {
    pub fn update_light_position(
        &mut self,
        all_positions: &[Vector3<f32>],
        camera_position: Vector3<f32>,
        move_light_to: LightMoveTarget,
    ) {
        crate::log!("LIGHT MOVE BUTTON PRESSED: {:?}", move_light_to);

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
            "Light position: ({:.2}, {:.2}, {:.2}) -> ({:.2}, {:.2}, {:.2})",
            current_pos.x,
            current_pos.y,
            current_pos.z,
            new_light_pos.x,
            new_light_pos.y,
            new_light_pos.z,
        );

        self.light_position = new_light_pos;
    }
}
