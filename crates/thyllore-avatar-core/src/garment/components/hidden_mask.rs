#[derive(Clone, Debug)]
pub struct HiddenMaskParams {
    pub cover_distance: f32,
    pub cone_angle_deg: f32,
    pub ray_count: usize,
    pub hidden_ratio: f32,
}
