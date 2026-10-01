/// Scene form of a rotation: `[w, x, y, z]`, matching the Python placement argument.
pub mod quaternion_wxyz {
    use cgmath::Quaternion;

    pub fn get(rotation: &Quaternion<f32>) -> [f32; 4] {
        [rotation.s, rotation.v.x, rotation.v.y, rotation.v.z]
    }

    pub fn set(rotation: &mut Quaternion<f32>, value: [f32; 4]) {
        *rotation = Quaternion::new(value[0], value[1], value[2], value[3]);
    }
}
