use cgmath::{Deg, InnerSpace, Vector3};

#[derive(Clone, Debug)]
pub struct Camera {
    pub pivot: Vector3<f32>,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub fov_y: Deg<f32>,
    pub near_plane: f32,

    pub initial_pivot: Vector3<f32>,
    pub initial_yaw: f32,
    pub initial_pitch: f32,
    pub initial_distance: f32,
}

impl Default for Camera {
    fn default() -> Self {
        let initial_pos = Vector3::new(5.0_f32, 5.0, 5.0);
        let pivot = Vector3::new(0.0_f32, 0.0, 0.0);
        let diff = initial_pos - pivot;
        let distance: f32 = diff.magnitude();
        let yaw: f32 = diff.x.atan2(diff.z);
        let pitch: f32 = (diff.y / distance).asin();

        Self {
            pivot,
            yaw,
            pitch,
            distance,
            fov_y: Deg(45.0),
            near_plane: 0.1,
            initial_pivot: pivot,
            initial_yaw: yaw,
            initial_pitch: pitch,
            initial_distance: distance,
        }
    }
}
