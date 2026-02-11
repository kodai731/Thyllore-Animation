use cgmath::Deg;

use crate::ecs::resource::{Camera, Exposure, PhysicalCameraParameters};

pub fn compute_fov_from_physical_params(params: &PhysicalCameraParameters) -> Deg<f32> {
    let half_angle = (params.sensor_height_mm / (2.0 * params.focal_length_mm)).atan();
    Deg(half_angle.to_degrees() * 2.0)
}

pub fn compute_ev100_from_physical_params(params: &PhysicalCameraParameters) -> f32 {
    let aperture_sq = params.aperture_f_stops * params.aperture_f_stops;
    (aperture_sq / params.shutter_speed_s).log2() - (params.sensitivity_iso / 100.0).log2()
}

pub fn compute_exposure_value(ev100: f32) -> f32 {
    1.0 / (2.0_f32.powf(ev100) * 1.2)
}

pub fn update_camera_from_physical_params(camera: &mut Camera, params: &PhysicalCameraParameters) {
    camera.fov_y = compute_fov_from_physical_params(params);
}

pub fn update_exposure_from_physical_params(
    exposure: &mut Exposure,
    params: &PhysicalCameraParameters,
) {
    exposure.ev100 = compute_ev100_from_physical_params(params);
    exposure.exposure_value = compute_exposure_value(exposure.ev100);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_fov_from_35mm_lens() {
        let params = PhysicalCameraParameters {
            focal_length_mm: 35.0,
            sensor_height_mm: 24.0,
            ..Default::default()
        };
        let fov = compute_fov_from_physical_params(&params);
        let expected = 2.0 * (24.0_f32 / (2.0 * 35.0)).atan().to_degrees();
        assert!(
            (fov.0 - expected).abs() < 0.01,
            "FOV was {} but expected {}",
            fov.0,
            expected
        );
    }

    #[test]
    fn test_compute_fov_from_50mm_lens() {
        let params = PhysicalCameraParameters {
            focal_length_mm: 50.0,
            sensor_height_mm: 24.0,
            ..Default::default()
        };
        let fov = compute_fov_from_physical_params(&params);
        let expected = 2.0 * (24.0_f32 / (2.0 * 50.0)).atan().to_degrees();
        assert!(
            (fov.0 - expected).abs() < 0.01,
            "FOV was {} but expected {}",
            fov.0,
            expected
        );
    }

    #[test]
    fn test_compute_ev100_sunny_16() {
        let params = PhysicalCameraParameters {
            aperture_f_stops: 16.0,
            shutter_speed_s: 1.0 / 100.0,
            sensitivity_iso: 100.0,
            ..Default::default()
        };
        let ev100 = compute_ev100_from_physical_params(&params);
        let expected = (16.0_f32 * 16.0 / (1.0 / 100.0)).log2();
        assert!(
            (ev100 - expected).abs() < 0.1,
            "EV100 was {} but expected ~{}",
            ev100,
            expected
        );
    }

    #[test]
    fn test_exposure_value_calculation() {
        let ev100 = 9.7_f32;
        let exposure = compute_exposure_value(ev100);
        let expected = 1.0 / (2.0_f32.powf(9.7) * 1.2);
        assert!(
            (exposure - expected).abs() < 1e-6,
            "Exposure was {} but expected {}",
            exposure,
            expected
        );
        assert!(
            exposure > 0.0 && exposure < 0.01,
            "Exposure {} should be small positive",
            exposure
        );
    }

    #[test]
    fn test_default_physical_camera_fov_reasonable() {
        let params = PhysicalCameraParameters::default();
        let fov = compute_fov_from_physical_params(&params);
        assert!(
            fov.0 > 20.0 && fov.0 < 90.0,
            "Default FOV {} should be between 20 and 90 degrees",
            fov.0
        );
    }

    #[test]
    fn test_update_camera_from_physical_params() {
        let mut camera = Camera::default();
        let params = PhysicalCameraParameters {
            focal_length_mm: 50.0,
            sensor_height_mm: 24.0,
            ..Default::default()
        };
        update_camera_from_physical_params(&mut camera, &params);
        let expected_fov = compute_fov_from_physical_params(&params);
        assert!((camera.fov_y.0 - expected_fov.0).abs() < 0.01);
    }

    #[test]
    fn test_update_exposure_from_physical_params() {
        let mut exposure = Exposure::default();
        let params = PhysicalCameraParameters::default();
        update_exposure_from_physical_params(&mut exposure, &params);
        let expected_ev100 = compute_ev100_from_physical_params(&params);
        assert!((exposure.ev100 - expected_ev100).abs() < 0.01);
        let expected_val = compute_exposure_value(expected_ev100);
        assert!((exposure.exposure_value - expected_val).abs() < 1e-6);
    }
}
