use crate::editable::components::keyframe::SourceClipId;

/// The clip the timeline stays on when a model without animations replaces the scene model: the selected
/// clip while it is still loaded, otherwise the oldest loaded clip.
pub fn select_kept_clip_id(
    selected: Option<SourceClipId>,
    loaded_ids: &[SourceClipId],
) -> Option<SourceClipId> {
    selected
        .filter(|id| loaded_ids.contains(id))
        .or_else(|| loaded_ids.iter().min().copied())
}

/// A loaded clip the model did not bring stays selected across a model load.
pub fn is_user_clip_selected(
    selected: Option<SourceClipId>,
    is_loaded: impl Fn(SourceClipId) -> bool,
    is_model_clip: impl Fn(SourceClipId) -> bool,
) -> bool {
    selected.is_some_and(|id| is_loaded(id) && !is_model_clip(id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_selected_clip_while_it_is_loaded() {
        assert_eq!(select_kept_clip_id(Some(7), &[3, 7, 9]), Some(7));
    }

    #[test]
    fn falls_back_to_the_oldest_loaded_clip() {
        assert_eq!(select_kept_clip_id(Some(4), &[9, 3, 7]), Some(3));
        assert_eq!(select_kept_clip_id(None, &[9, 3, 7]), Some(3));
        assert_eq!(select_kept_clip_id(Some(4), &[]), None);
    }

    #[test]
    fn only_a_loaded_non_model_clip_counts_as_a_user_selection() {
        let loaded = |id: SourceClipId| id == 1 || id == 2;
        let from_model = |id: SourceClipId| id == 2;

        assert!(is_user_clip_selected(Some(1), loaded, from_model));
        assert!(!is_user_clip_selected(Some(2), loaded, from_model));
        assert!(!is_user_clip_selected(Some(5), loaded, from_model));
        assert!(!is_user_clip_selected(None, loaded, from_model));
    }
}
