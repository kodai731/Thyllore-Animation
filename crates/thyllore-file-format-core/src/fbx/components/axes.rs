#[derive(Clone, Debug)]
pub struct FbxAxesInfo {
    pub up_axis: i32,
    pub up_axis_sign: i32,
    pub front_axis: i32,
    pub front_axis_sign: i32,
    pub coord_axis: i32,
    pub coord_axis_sign: i32,
}

impl Default for FbxAxesInfo {
    fn default() -> Self {
        Self {
            up_axis: 1,
            up_axis_sign: 1,
            front_axis: 2,
            front_axis_sign: 1,
            coord_axis: 0,
            coord_axis_sign: 1,
        }
    }
}
