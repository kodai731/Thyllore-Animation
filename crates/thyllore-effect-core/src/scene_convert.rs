/// Scene form of a rotation: `[w, x, y, z]`, matching the Python placement argument.
pub mod quaternion_wxyz {
    use glam::Quat;

    pub fn get(rotation: &Quat) -> [f32; 4] {
        [rotation.w, rotation.x, rotation.y, rotation.z]
    }

    pub fn set(rotation: &mut Quat, value: [f32; 4]) {
        *rotation = Quat::from_xyzw(value[1], value[2], value[3], value[0]);
    }
}
