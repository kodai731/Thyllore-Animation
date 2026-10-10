#[derive(Clone, Copy, Debug)]
pub struct HumanoidFrame {
    pub right: [f32; 3],
    pub up: [f32; 3],
    pub forward: [f32; 3],
}

impl HumanoidFrame {
    pub fn to_character(&self, v: [f32; 3]) -> [f32; 3] {
        [
            thyllore_math_core::dot3(v, self.right),
            thyllore_math_core::dot3(v, self.up),
            thyllore_math_core::dot3(v, self.forward),
        ]
    }
}
