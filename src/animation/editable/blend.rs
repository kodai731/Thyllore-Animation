#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BlendMode {
    #[default]
    Override,
    Additive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum EaseType {
    #[default]
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    Stepped,
}
