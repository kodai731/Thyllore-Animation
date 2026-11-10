use crate::editable::components::keyframe::SourceClipId;

/// The selected clip while it is still loaded, otherwise the oldest loaded clip.
pub fn select_kept_clip_id(
    selected: Option<SourceClipId>,
    loaded_ids: impl IntoIterator<Item = SourceClipId>,
) -> Option<SourceClipId> {
    let mut oldest_loaded: Option<SourceClipId> = None;
    for loaded_id in loaded_ids {
        if selected == Some(loaded_id) {
            return selected;
        }
        oldest_loaded = Some(oldest_loaded.map_or(loaded_id, |oldest| oldest.min(loaded_id)));
    }
    oldest_loaded
}

#[cfg(test)]
mod tests {
    use super::*;

    fn select(selected: Option<SourceClipId>, loaded: &[SourceClipId]) -> Option<SourceClipId> {
        select_kept_clip_id(selected, loaded.iter().copied())
    }

    #[test]
    fn keeps_the_selected_clip_while_it_is_loaded() {
        assert_eq!(select(Some(7), &[3, 7, 9]), Some(7));
    }

    #[test]
    fn falls_back_to_the_oldest_loaded_clip() {
        assert_eq!(select(Some(4), &[9, 3, 7]), Some(3));
        assert_eq!(select(None, &[9, 3, 7]), Some(3));
        assert_eq!(select(Some(4), &[]), None);
    }
}
