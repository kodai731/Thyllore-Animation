#[derive(Clone, Debug)]
pub struct MuscleCurves {
    pub sample_rate: u32,
    pub duration: f32,
    pub times: Vec<f32>,
    pub frames: Vec<Vec<f32>>,
}
