#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PostProcessPreview {
    #[default]
    Enabled,
    Disabled,
}

impl PostProcessPreview {
    pub fn toggled(self) -> Self {
        match self {
            PostProcessPreview::Enabled => PostProcessPreview::Disabled,
            PostProcessPreview::Disabled => PostProcessPreview::Enabled,
        }
    }
}
