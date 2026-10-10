use crate::animation::editable::{EditableAnimationClip, SourceClipId};
use crate::ecs::resource::{ClipLibrary, EditHistory, TimelineState};
use crate::ecs::systems::edit_history_push_clip_mergeable;
use crate::ecs::world::World;

/// The active clip as it was before an edit, so the edit can be pushed to the history afterwards.
pub(super) struct ClipSnapshot {
    clip_id: SourceClipId,
    before: EditableAnimationClip,
}

impl ClipSnapshot {
    pub(super) fn capture(
        timeline_state: &TimelineState,
        clip_library: &ClipLibrary,
    ) -> Option<Self> {
        let clip_id = timeline_state.current_clip_id?;
        let before = clip_library.get(clip_id)?.clone();
        Some(Self { clip_id, before })
    }

    pub(super) fn record(self, world: &mut World, description: &'static str) {
        let Some(after) = self.clip_after(world) else {
            return;
        };
        if let Some(mut edit_history) = world.get_resource_mut::<EditHistory>() {
            edit_history.push_clip_edit(self.clip_id, self.before, after, description);
        }
    }

    pub(super) fn record_mergeable(self, world: &mut World, description: &'static str) {
        let Some(after) = self.clip_after(world) else {
            return;
        };
        if let Some(mut edit_history) = world.get_resource_mut::<EditHistory>() {
            edit_history_push_clip_mergeable(
                &mut edit_history,
                self.clip_id,
                self.before,
                after,
                description,
            );
        }
    }

    fn clip_after(&self, world: &World) -> Option<EditableAnimationClip> {
        world.resource::<ClipLibrary>().get(self.clip_id).cloned()
    }
}
