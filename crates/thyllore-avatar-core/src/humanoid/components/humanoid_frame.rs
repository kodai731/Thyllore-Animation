#[derive(Clone, Copy, Debug)]
pub struct HumanoidFrame {
    pub right: [f32; 3],
    pub up: [f32; 3],
    pub forward: [f32; 3],
}

impl HumanoidFrame {
    pub fn to_character(&self, v: [f32; 3]) -> [f32; 3] {
        [dot(v, self.right), dot(v, self.up), dot(v, self.forward)]
    }
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
