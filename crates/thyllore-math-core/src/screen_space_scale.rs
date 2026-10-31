use cgmath::{InnerSpace, Vector3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenSpaceScale {
    camera_position: Vector3<f32>,
    world_per_pixel_at_unit_distance: f32,
}

impl ScreenSpaceScale {
    pub fn new(camera_position: Vector3<f32>, fov_y_radians: f32, viewport_height_px: f32) -> Self {
        let frustum_height_at_unit_distance = 2.0 * (fov_y_radians * 0.5).tan();
        Self {
            camera_position,
            world_per_pixel_at_unit_distance: frustum_height_at_unit_distance
                / viewport_height_px.max(1.0),
        }
    }

    pub fn world_size(&self, position: [f32; 3], pixels: f32) -> f32 {
        let distance = (Vector3::from(position) - self.camera_position).magnitude();
        distance * self.world_per_pixel_at_unit_distance * pixels
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_grows_with_distance_and_shrinks_with_resolution() {
        let fov = 90f32.to_radians();
        let near = ScreenSpaceScale::new(Vector3::new(0.0, 0.0, 0.0), fov, 1000.0);
        let one_meter = near.world_size([0.0, 0.0, 1.0], 10.0);
        let two_meters = near.world_size([0.0, 0.0, 2.0], 10.0);
        assert!((one_meter - 0.02).abs() < 1e-5);
        assert!((two_meters - 0.04).abs() < 1e-5);

        let tall = ScreenSpaceScale::new(Vector3::new(0.0, 0.0, 0.0), fov, 2000.0);
        assert!((tall.world_size([0.0, 0.0, 1.0], 10.0) - 0.01).abs() < 1e-5);
    }
}
