#[derive(Clone, Debug)]
pub struct BoneInput {
    pub name: String,
    pub parent: Option<usize>,
    pub rest_position: [f32; 3],
}
