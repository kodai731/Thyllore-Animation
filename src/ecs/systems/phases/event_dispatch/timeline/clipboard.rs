use crate::ecs::resource::{ClipLibrary, KeyframeCopyBuffer, TimelineState};
use crate::ecs::systems::process_keyframe_clipboard_events;
use crate::ecs::world::World;

use super::history::ClipSnapshot;
use super::TimelineEvent;

pub(super) fn dispatch_keyframe_clipboard_events(events: &[TimelineEvent], world: &mut World) {
    let has_paste = events.iter().any(|e| {
        matches!(
            e,
            TimelineEvent::PasteKeyframes { .. } | TimelineEvent::MirrorPaste { .. }
        )
    });

    let snapshot = {
        let timeline_state = world.resource::<TimelineState>();
        let mut clip_library = world.resource_mut::<ClipLibrary>();
        let mut copy_buffer = world.resource_mut::<KeyframeCopyBuffer>();
        let snapshot = has_paste
            .then(|| ClipSnapshot::capture(&timeline_state, &clip_library))
            .flatten();
        process_keyframe_clipboard_events(
            events,
            &timeline_state,
            &mut clip_library,
            &mut copy_buffer,
        );
        snapshot
    };

    if let Some(snapshot) = snapshot {
        snapshot.record(world, "paste keyframes");
    }
}
