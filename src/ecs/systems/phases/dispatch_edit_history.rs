use crate::ecs::events::UIEvent;
use crate::ecs::resource::{ClipLibrary, EditHistory};
use crate::ecs::systems::{apply_redo, apply_undo};
use crate::ecs::world::World;

pub fn dispatch_edit_history_events(events: &[UIEvent], world: &mut World) {
    for event in events {
        match event {
            UIEvent::Undo => {
                if !world.contains_resource::<EditHistory>() {
                    return;
                }

                let mut edit_history = world.resource_mut::<EditHistory>();
                if !edit_history.can_undo() {
                    return;
                }
                let edit_history_ptr: *mut EditHistory = &mut *edit_history;
                drop(edit_history);

                let mut clip_library = world.resource_mut::<ClipLibrary>();
                let clip_library_ptr: *mut ClipLibrary = &mut *clip_library;
                drop(clip_library);

                unsafe {
                    apply_undo(&mut *edit_history_ptr, &mut *clip_library_ptr, world);
                }
            }

            UIEvent::Redo => {
                if !world.contains_resource::<EditHistory>() {
                    return;
                }

                let mut edit_history = world.resource_mut::<EditHistory>();
                if !edit_history.can_redo() {
                    return;
                }
                let edit_history_ptr: *mut EditHistory = &mut *edit_history;
                drop(edit_history);

                let mut clip_library = world.resource_mut::<ClipLibrary>();
                let clip_library_ptr: *mut ClipLibrary = &mut *clip_library;
                drop(clip_library);

                unsafe {
                    apply_redo(&mut *edit_history_ptr, &mut *clip_library_ptr, world);
                }
            }

            _ => {}
        }
    }
}
