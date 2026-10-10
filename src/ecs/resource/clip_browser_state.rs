#[derive(Clone, Debug, Default)]
pub struct ClipBrowserState {
    pub filter_text: String,
}

crate::startup_resource!(ClipBrowserState, Editor);
