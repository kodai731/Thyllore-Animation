use crate::animation::editable::{EditableAnimationClip, SourceClipId};
use crate::ecs::resource::{EditCommand, EditCommandAfter, EditHistory};

pub fn edit_history_push_clip_mergeable(
    history: &mut EditHistory,
    clip_id: SourceClipId,
    before: EditableAnimationClip,
    after: EditableAnimationClip,
    description: &'static str,
) {
    if let Some(last) = history.last_undo_mut() {
        if matches_clip_target(last, clip_id, description) {
            last.after = EditCommandAfter::Clip(after);
            return;
        }
    }
    history.push_clip_edit(clip_id, before, after, description);
}

fn matches_clip_target(
    entry: &crate::ecs::resource::EditEntry,
    clip_id: SourceClipId,
    description: &str,
) -> bool {
    match &entry.command {
        EditCommand::ClipModified {
            clip_id: cid,
            description: desc,
            ..
        } => *cid == clip_id && *desc == description,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_dummy_clip(name: &str) -> EditableAnimationClip {
        EditableAnimationClip::new(1, name.to_string())
    }

    #[test]
    fn test_mergeable_updates_after_only() {
        let mut history = EditHistory::new(100);
        let before = make_dummy_clip("original");
        let mid = make_dummy_clip("mid");
        let final_state = make_dummy_clip("final");

        let mid_clone = mid.clone();
        edit_history_push_clip_mergeable(&mut history, 1, before, mid, "drag keyframe");
        edit_history_push_clip_mergeable(&mut history, 1, mid_clone, final_state, "drag keyframe");

        let entry = history.pop_undo().unwrap();
        assert!(history.pop_undo().is_none());

        match &entry.command {
            EditCommand::ClipModified { before, .. } => {
                assert_eq!(before.name, "original");
            }
            _ => panic!("unexpected command type"),
        }
        match &entry.after {
            EditCommandAfter::Clip(after) => {
                assert_eq!(after.name, "final");
            }
            _ => panic!("unexpected after type"),
        }
    }

    #[test]
    fn test_mergeable_different_description_creates_new_entry() {
        let mut history = EditHistory::new(100);
        let a = make_dummy_clip("a");
        let b = make_dummy_clip("b");
        let c = make_dummy_clip("c");
        let d = make_dummy_clip("d");

        edit_history_push_clip_mergeable(&mut history, 1, a, b, "drag keyframe");
        edit_history_push_clip_mergeable(&mut history, 1, c, d, "move tangent");

        assert!(history.pop_undo().is_some());
        assert!(history.pop_undo().is_some());
        assert!(history.pop_undo().is_none());
    }
}
