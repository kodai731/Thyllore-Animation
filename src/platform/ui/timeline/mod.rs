use crate::platform::ui::numeric_format::numeric_format;
use imgui::{Condition, MouseButton};

use crate::animation::editable::{BlendMode, SourceClipId};
use crate::animation::BoneId;
use crate::asset::AssetStorage;
use crate::ecs::component::{
    ClipGroupSnapshot, ClipInstanceSnapshot, ClipSchedule, ClipTrackEntry, ClipTrackSnapshot,
};
use crate::ecs::resource::{
    ClipDragState, ClipDragType, ClipLibrary, ClipPreview, CurveEditorBuffer, CurveEditorPlacement,
    CurveEditorState, PoseLibrary, TimelineInteractionState, TimelineState, UiPointerOwnerId,
};
use crate::ecs::systems::clip_track_systems::query_clip_tracks;
use crate::ecs::systems::phases::event_dispatch::clip_instance::ClipInstanceEvent;
use crate::ecs::systems::phases::event_dispatch::hierarchy::HierarchyEvent;
use crate::ecs::systems::phases::event_dispatch::scalar_curve::ScalarCurveEvent;
use crate::ecs::systems::phases::event_dispatch::timeline::TimelineEvent;
use crate::ecs::systems::{
    clip_drag_preview_times, clip_schedule_assign_lanes, find_preview_owner,
    timeline_effective_duration,
};
use crate::ecs::world::World;
use crate::platform::ui::curve_editor::{build_curve_editor_tab, build_placement_toggle};
use crate::platform::ui::pointer::{
    read_ui_pointer, ui_pointer_available, ui_pointer_begin, PointerRegion,
};
use crate::platform::ui::theme::icons::Icon;
use crate::platform::ui::theme::widgets::{
    icon_button, segmented_control, toolbar_divider, ButtonState,
};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

use crate::ecs::resource::LayoutSnapshot;

pub(crate) const TRACK_LABEL_WIDTH: f32 = 150.0;
const TIME_RULER_HEIGHT: f32 = 30.0;
pub(crate) const PIXELS_PER_SECOND: f32 = 80.0;
const PLAYHEAD_HANDLE_SIZE: f32 = 10.0;
const CLIP_TRACK_HEIGHT: f32 = 28.0;
const CLIP_EDGE_DRAG_WIDTH: f32 = 5.0;
const CLIP_BLOCK_MIN_WIDTH: f32 = 12.0;
const CLIP_BLOCK_COLORS: [[f32; 4]; 4] = [
    [0.3, 0.5, 0.8, 0.9],
    [0.5, 0.7, 0.3, 0.9],
    [0.8, 0.4, 0.3, 0.9],
    [0.7, 0.5, 0.7, 0.9],
];

mod clip_block;
mod clip_drag;
mod clip_properties;
mod clip_track;
mod input;
mod ruler;
mod transport;
mod window;

use clip_block::*;
use clip_drag::*;
use clip_properties::*;
use clip_track::*;
use input::*;
use ruler::*;
use transport::*;
