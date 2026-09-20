#[derive(Clone, Debug, Default)]
pub struct ViewportInput {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub hovered: bool,
    pub focused: bool,
    pub resize_pending: Option<(u32, u32)>,
}
