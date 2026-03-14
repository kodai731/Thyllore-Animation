use crate::ecs::resource::ClipDragState;

#[derive(Clone, Debug, Default)]
pub struct TimelineInteractionState {
    pub scrubbing: bool,
    pub dragging_clip: Option<ClipDragState>,
}
