use crate::ecs::resource::{ClipLibrary, CurveEditorBuffer, CurveEditorState, TimelineState};
use crate::ecs::systems::{curve_editor_capture_buffer, curve_editor_swap_buffer};
use crate::ecs::world::World;

use super::history::ClipSnapshot;
use super::TimelineEvent;

const CAPTURE_SAMPLE_COUNT: usize = 100;

pub(super) fn dispatch_curve_buffer_events(events: &[TimelineEvent], world: &mut World) {
    for event in events {
        match event {
            TimelineEvent::CaptureBuffer => capture_buffer(world),
            TimelineEvent::SwapBuffer => swap_buffer(world),
            _ => {}
        }
    }
}

fn capture_buffer(world: &World) {
    let timeline_state = world.resource::<TimelineState>();
    let clip_library = world.resource::<ClipLibrary>();
    let curve_editor = world.resource::<CurveEditorState>();
    let mut curve_buffer = world.resource_mut::<CurveEditorBuffer>();

    let Some(bone_id) = curve_editor.selected_bone_id() else {
        return;
    };
    let Some(clip) = timeline_state
        .current_clip_id
        .and_then(|id| clip_library.get(id))
    else {
        return;
    };
    curve_editor_capture_buffer(
        &mut curve_buffer,
        clip,
        bone_id,
        &curve_editor.visible_curves,
        clip.duration,
        CAPTURE_SAMPLE_COUNT,
    );
}

fn swap_buffer(world: &mut World) {
    let snapshot = {
        let curve_editor = world.resource::<CurveEditorState>();
        let timeline_state = world.resource::<TimelineState>();
        let mut clip_library = world.resource_mut::<ClipLibrary>();
        let mut curve_buffer = world.resource_mut::<CurveEditorBuffer>();
        let snapshot = ClipSnapshot::capture(&timeline_state, &clip_library);

        let bone_id = curve_editor.selected_bone_id();
        let clip = timeline_state
            .current_clip_id
            .and_then(|id| clip_library.get_mut(id));
        if let (Some(bone_id), Some(clip)) = (bone_id, clip) {
            curve_editor_swap_buffer(&mut curve_buffer, clip, bone_id);
        }
        snapshot
    };

    if let Some(snapshot) = snapshot {
        snapshot.record(world, "swap buffer");
    }
}
