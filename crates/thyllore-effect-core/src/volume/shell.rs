#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct VolumeShell {
    pub height: f32,
    pub radius_base: f32,
    pub radius_slope: f32,
    pub radius_offset_q: f32,
    pub width_q: f32,
    pub strength: f32,
    pub h_top: f32,
    pub top_fade: f32,
    pub sigma_t: f32,
}

impl VolumeShell {
    pub(crate) fn wall_radius(&self, h: f32) -> f32 {
        self.radius_base + self.radius_slope * h
    }

    pub(crate) fn wall_radius_sq(&self, h: f32) -> f32 {
        let radius = self.wall_radius(h);
        radius * radius + self.radius_offset_q
    }
}
