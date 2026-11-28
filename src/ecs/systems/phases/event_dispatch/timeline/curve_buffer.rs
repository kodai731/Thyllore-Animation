use crate::ecs::resource::{ClipLibrary, CurveEditorBuffer, CurveEditorState, TimelineState};
use crate::ecs::systems::timeline::keyframe_edit::edit_clip;
use crate::ecs::systems::{curve_editor_capture_buffer, curve_editor_swap_buffer};
use crate::ecs::world::World;

use super::history::ClipSnapshot;
use super::TimelineEvent;

const CAPTURE_SAMPLE_COUNT: usize = 100;

pub(super) fn dispatch_curve_buffer_events(events: &[TimelineEvent], world: &mut World) {
    for event in events {
        dispatch_curve_buffer_event(event, world);
    }
}

fn dispatch_curve_buffer_event(event: &TimelineEvent, world: &mut World) {
    match event {
        TimelineEvent::CaptureBuffer => capture_buffer(world),
        TimelineEvent::SwapBuffer => swap_buffer(world),
        _ => {}
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

        let Some(bone_id) = curve_editor.selected_bone_id() else {
            return;
        };
        let Some(clip_id) = timeline_state.current_clip_id else {
            return;
        };
        let snapshot = ClipSnapshot::capture(&timeline_state, &clip_library);

        let was_swapped = edit_clip(&mut clip_library, clip_id, |clip| {
            curve_editor_swap_buffer(&mut curve_buffer, clip, bone_id)
        });
        if !was_swapped {
            return;
        }
        snapshot
    };

    if let Some(snapshot) = snapshot {
        snapshot.record(world, "swap buffer");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset::AssetStorage;
    use crate::ecs::resource::EditHistory;
    use crate::ecs::systems::clip_library_register_and_activate;
    use thyllore_anim_core::editable::{EditableAnimationClip, SourceClipId};

    fn world_playing_a_model_clip() -> (World, SourceClipId) {
        let mut world = World::new();
        let mut assets = AssetStorage::new();
        let mut library = ClipLibrary::new();
        let clip_id = clip_library_register_and_activate(
            &mut library,
            &mut assets,
            EditableAnimationClip::new(0, "model".to_string()),
        );
        library.model_clip_ids.insert(clip_id);
        library.mark_clean(clip_id);
        let timeline_state = TimelineState {
            current_clip_id: Some(clip_id),
            ..TimelineState::default()
        };

        world.insert_resource(library);
        world.insert_resource(timeline_state);
        world.insert_resource(CurveEditorState::default());
        world.insert_resource(CurveEditorBuffer::default());
        world.insert_resource(EditHistory::default());

        (world, clip_id)
    }

    #[test]
    fn swap_without_a_selected_bone_leaves_the_model_clip_untouched() {
        let (mut world, clip_id) = world_playing_a_model_clip();

        swap_buffer(&mut world);

        let library = world.resource::<ClipLibrary>();
        assert!(library.is_model_clip(clip_id));
        assert!(!library.is_dirty(clip_id));
        assert!(!world.resource::<EditHistory>().can_undo());
    }

    #[test]
    fn swap_with_an_empty_buffer_leaves_the_model_clip_untouched() {
        let (mut world, clip_id) = world_playing_a_model_clip();
        world.resource_mut::<CurveEditorState>().select_bone(0);

        swap_buffer(&mut world);

        let library = world.resource::<ClipLibrary>();
        assert!(library.is_model_clip(clip_id));
        assert!(!world.resource::<EditHistory>().can_undo());
    }
}
