pub const BASE_PIXELS_PER_SECOND: f32 = 80.0;
pub const WHEEL_ZOOM_STEP: f32 = 1.15;

#[derive(Clone, Copy, Debug)]
pub struct TimeAxis {
    pub origin_x: f32,
    pub px_per_sec: f32,
    pub time_offset: f32,
}

impl TimeAxis {
    pub fn new(px_per_sec: f32) -> Self {
        Self {
            origin_x: 0.0,
            px_per_sec,
            time_offset: 0.0,
        }
    }

    pub fn time_to_x(&self, t: f32) -> f32 {
        self.origin_x + (t - self.time_offset) * self.px_per_sec
    }

    pub fn x_to_time(&self, x: f32) -> f32 {
        self.time_offset + (x - self.origin_x) / self.px_per_sec
    }

    pub fn zoom_at(&mut self, cursor_x: f32, factor: f32) {
        let cursor_time = self.x_to_time(cursor_x);
        self.px_per_sec *= factor;
        self.origin_x = cursor_x - (cursor_time - self.time_offset) * self.px_per_sec;
    }

    pub fn pan_pixels(&mut self, dx: f32) {
        self.origin_x += dx;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_time_to_x_and_back() {
        let axis = TimeAxis::new(BASE_PIXELS_PER_SECOND);
        for t in [0.0, 1.0, 5.0, 10.5, -2.0] {
            let x = axis.time_to_x(t);
            let recovered = axis.x_to_time(x);
            assert!((recovered - t).abs() < 1e-6, "t={}", t);
        }
    }

    #[test]
    fn round_trip_x_to_time_and_back() {
        let axis = TimeAxis::new(BASE_PIXELS_PER_SECOND);
        for x in [0.0, 80.0, 400.0, -80.0] {
            let t = axis.x_to_time(x);
            let recovered = axis.time_to_x(t);
            assert!((recovered - x).abs() < 1e-6, "x={}", x);
        }
    }

    #[test]
    fn zoom_at_keeps_cursor_time() {
        let mut axis = TimeAxis::new(BASE_PIXELS_PER_SECOND);
        let cursor_x = 200.0;
        let time_before = axis.x_to_time(cursor_x);

        axis.zoom_at(cursor_x, 2.0);

        let time_after = axis.x_to_time(cursor_x);
        assert!((time_after - time_before).abs() < 1e-6);
    }

    #[test]
    fn pan_shifts_times_by_expected_amount() {
        let mut axis = TimeAxis::new(BASE_PIXELS_PER_SECOND);
        let time_before = axis.x_to_time(0.0);

        axis.pan_pixels(80.0);

        let time_after = axis.x_to_time(0.0);
        assert!((time_after - time_before + 1.0).abs() < 1e-6);
    }

    #[test]
    fn wheel_zoom_step_keeps_cursor_time_invariant() {
        let mut axis = TimeAxis::new(BASE_PIXELS_PER_SECOND);
        let cursor_x = 320.0;
        let time_before = axis.x_to_time(cursor_x);

        axis.zoom_at(cursor_x, WHEEL_ZOOM_STEP);

        let time_after = axis.x_to_time(cursor_x);
        assert!((time_after - time_before).abs() < 1e-6);
    }
}
