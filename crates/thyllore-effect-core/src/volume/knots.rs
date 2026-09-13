// Mirror of shaders/include/ray_knots.glsl.

pub const RAY_MAX_KNOTS: usize = 56;
pub const RAY_LINEAR_COEFFICIENT_EPSILON: f32 = 1e-7;

/// Ray parameters at which a piecewise-polynomial medium changes form, bracketed by the
/// near and far bounds; once sorted, every consecutive pair is one closed-form piece.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayKnots {
    values: [f32; RAY_MAX_KNOTS],
    count: usize,
}

impl RayKnots {
    pub fn begin(t_near: f32, t_far: f32) -> Self {
        let mut values = [0.0f32; RAY_MAX_KNOTS];
        values[0] = t_near;
        values[1] = t_far;
        Self { values, count: 2 }
    }

    pub fn push(&mut self, t: f32, lo: f32, hi: f32) {
        if t <= lo || t >= hi || self.count >= RAY_MAX_KNOTS {
            return;
        }
        self.values[self.count] = t;
        self.count += 1;
    }

    pub fn push_quadratic_roots(&mut self, a: f32, b: f32, c: f32, lo: f32, hi: f32) {
        if a.abs() < RAY_LINEAR_COEFFICIENT_EPSILON {
            if b.abs() >= RAY_LINEAR_COEFFICIENT_EPSILON {
                self.push(-c / b, lo, hi);
            }
            return;
        }
        let discriminant = b * b - 4.0 * a * c;
        if discriminant < 0.0 {
            return;
        }
        let sqrt_discriminant = discriminant.sqrt();
        self.push((-b - sqrt_discriminant) / (2.0 * a), lo, hi);
        self.push((-b + sqrt_discriminant) / (2.0 * a), lo, hi);
    }

    pub fn sort(&mut self) {
        for i in 1..self.count {
            let value = self.values[i];
            let mut j = i;
            while j > 0 && self.values[j - 1] > value {
                self.values[j] = self.values[j - 1];
                j -= 1;
            }
            self.values[j] = value;
        }
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn values(&self) -> &[f32] {
        &self.values[..self.count]
    }

    pub fn pieces(&self) -> impl Iterator<Item = (f32, f32)> + '_ {
        self.values().windows(2).map(|pair| (pair[0], pair[1]))
    }
}
