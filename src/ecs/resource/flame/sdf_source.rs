/// The flame SDF file requested at startup; the flame reads it when it builds its SDF texture.
#[derive(Clone, Debug)]
pub struct FlameSdfSource {
    pub path: String,
}
