#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum RestPose {
    TPose,
    APose,
    #[default]
    Unknown,
}
