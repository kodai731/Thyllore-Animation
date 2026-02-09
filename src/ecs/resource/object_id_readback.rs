#[derive(Clone, Debug, Default)]
pub struct ObjectIdReadback {
    pub pending_pixel: Option<(u32, u32)>,
    pub copy_in_flight: bool,
    pub last_read_object_id: Option<u32>,
    pub is_shift: bool,
    pub is_ctrl: bool,
}
