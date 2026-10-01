#[derive(Clone, Debug, PartialEq)]
pub struct UnityKey {
    pub time: f32,
    pub value: f32,
    pub in_slope: f32,
    pub out_slope: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnityFloatCurve {
    pub path: String,
    pub attribute: String,
    pub keys: Vec<UnityKey>,
}
