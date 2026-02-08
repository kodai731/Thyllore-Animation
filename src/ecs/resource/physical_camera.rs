#[derive(Clone, Debug)]
pub struct PhysicalCameraParameters {
    pub focal_length_mm: f32,
    pub sensor_height_mm: f32,
    pub aperture_f_stops: f32,
    pub shutter_speed_s: f32,
    pub sensitivity_iso: f32,
}

impl Default for PhysicalCameraParameters {
    fn default() -> Self {
        Self {
            focal_length_mm: 35.0,
            sensor_height_mm: 18.66,
            aperture_f_stops: 16.0,
            shutter_speed_s: 1.0 / 125.0,
            sensitivity_iso: 100.0,
        }
    }
}
