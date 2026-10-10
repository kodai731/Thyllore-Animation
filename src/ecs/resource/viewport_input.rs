/// The scene viewport as the UI sees it: the image the engine renders into and the window imgui draws it in.
#[derive(Clone, Debug, Default)]
pub struct ViewportInput {
    pub texture_id: usize,
    pub image_size: [u32; 2],
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub hovered: bool,
    pub focused: bool,
    pub resize_pending: Option<(u32, u32)>,
}

crate::startup_resource!(ViewportInput, Editor);
