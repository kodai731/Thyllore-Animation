#[derive(Clone, Debug, PartialEq)]
pub enum ToneMapOperator {
    None,
    AcesFilmic,
    Reinhard,
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
            gamma: 2.2,
        }
    }
}

impl ToneMapOperator {
    pub fn as_int(&self) -> i32 {
        match self {
            ToneMapOperator::None => 0,
            ToneMapOperator::AcesFilmic => 1,
            ToneMapOperator::Reinhard => 2,
        }
    }
}
