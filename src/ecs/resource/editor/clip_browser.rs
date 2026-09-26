use crate::animation::editable::SourceClipId;

#[derive(Clone, Debug, Default)]
pub struct ClipBrowserState {
    pub selected_clip_id: Option<SourceClipId>,
    pub filter_text: String,
}
