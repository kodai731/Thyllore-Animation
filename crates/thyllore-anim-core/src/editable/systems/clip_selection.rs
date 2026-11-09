use crate::editable::components::keyframe::SourceClipId;

/// The selected clip while it is still loaded, otherwise the oldest loaded clip.
pub fn select_kept_clip_id(
    selected: Option<SourceClipId>,
    is_loaded: impl Fn(SourceClipId) -> bool,
    loaded_ids: impl Iterator<Item = SourceClipId>,
) -> Option<SourceClipId> {
    selected
        .filter(|id| is_loaded(*id))
        .or_else(|| loaded_ids.min())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn select(selected: Option<SourceClipId>, loaded: &[SourceClipId]) -> Option<SourceClipId> {
        select_kept_clip_id(selected, |id| loaded.contains(&id), loaded.iter().copied())
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
