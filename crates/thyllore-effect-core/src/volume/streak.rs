use cgmath::Vector3;

/// Azimuthal streak modulation of a shell: 1 + amplitude * cos(order * (theta - phase) + (rise - twist) * y).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StreakModulation {
    pub order: f32,
    pub twist: f32,
    pub rise_time: f32,
    pub amplitude: f32,
}

impl StreakModulation {
    pub fn sigma(&self, local: Vector3<f32>, rotation_phase: f32) -> f32 {
        let angle = self.order * (local.z.atan2(local.x) - rotation_phase) - self.twist * local.y
            + self.rise_time * local.y;
        1.0 + self.amplitude * angle.cos()
    }

    /// Shortest length along any ray over which the pattern completes one period at `radius`.
    pub fn wavelength(&self, radius: f32) -> f32 {
        let angular = self.order / radius.max(1e-3);
        let vertical = self.rise_time - self.twist;
        std::f32::consts::TAU / (angular * angular + vertical * vertical).sqrt().max(1e-3)
    }
}
