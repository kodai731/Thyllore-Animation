#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(i32)]
pub enum ToneMapOperator {
    None = 0,
    AcesFilmic = 1,
    Reinhard = 2,
}

#[derive(Clone, Debug)]
pub struct ToneMapping {
    pub enabled: bool,
    pub operator: ToneMapOperator,
    pub gamma: f32,
}

impl Default for ToneMapping {
    fn default() -> Self {
        Self {
            enabled: true,
            operator: ToneMapOperator::AcesFilmic,
            gamma: 1.0,
        }
    }
}
