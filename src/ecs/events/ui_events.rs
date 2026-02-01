use cgmath::{Quaternion, Vector3};

use crate::animation::editable::{
    BezierHandle, BlendMode, ClipGroupId, ClipInstanceId, InterpolationType,
    KeyframeId, PropertyType, SourceClipId,
};
use crate::ecs::resource::SelectionModifier;
use crate::animation::BoneId;
use crate::app::data::LightMoveTarget;
use crate::ecs::world::Entity;

#[derive(Clone, Debug)]
pub enum UIEvent {
    LoadModel { path: String },

    ResetCamera,
    ResetCameraUp,
    MoveCameraToModel,
    MoveCameraToLightGizmo,

    SetLightPosition(Vector3<f32>),
    MoveLightToBounds(LightMoveTarget),

    TakeScreenshot,

    DebugShadowInfo,
    DebugBillboardDepth,
    DumpDebugInfo,

    SelectEntity(Entity),
    DeselectAll,
    ToggleEntitySelection(Entity),
    ExpandEntity(Entity),
    CollapseEntity(Entity),
    SetSearchFilter(String),

    SetEntityVisible(Entity, bool),
    SetEntityTranslation(Entity, Vector3<f32>),
    SetEntityRotation(Entity, Quaternion<f32>),
    SetEntityScale(Entity, Vector3<f32>),
    RenameEntity(Entity, String),
    FocusOnEntity(Entity),

    TimelinePlay,
    TimelinePause,
    TimelineStop,
    TimelineSetTime(f32),
    TimelineSetSpeed(f32),
    TimelineToggleLoop,
    TimelineSelectClip(SourceClipId),
    TimelineToggleTrack(BoneId),
    TimelineExpandTrack(BoneId),
    TimelineCollapseTrack(BoneId),
    TimelineSelectKeyframe {
        bone_id: BoneId,
        property_type: PropertyType,
        keyframe_id: KeyframeId,
        modifier: SelectionModifier,
    },
    TimelineAddKeyframe {
        bone_id: BoneId,
        property_type: PropertyType,
        time: f32,
        value: f32,
    },
    TimelineDeleteSelectedKeyframes,
    TimelineMoveKeyframe {
        bone_id: BoneId,
        property_type: PropertyType,
        keyframe_id: KeyframeId,
        new_time: f32,
        new_value: f32,
    },
    TimelineZoomIn,
    TimelineZoomOut,
    TimelineSetKeyframeInterpolation {
        bone_id: BoneId,
        property_type: PropertyType,
        keyframe_id: KeyframeId,
        interpolation: InterpolationType,
    },
    TimelineSetKeyframeTangent {
        bone_id: BoneId,
        property_type: PropertyType,
        keyframe_id: KeyframeId,
        in_tangent: BezierHandle,
        out_tangent: BezierHandle,
    },
    TimelineAutoTangent {
        bone_id: BoneId,
        property_type: PropertyType,
        keyframe_id: KeyframeId,
    },

    TimelineToggleViewMode,
    TimelineSetSnapToFrame(bool),
    TimelineSetSnapToKey(bool),
    TimelineSetFrameRate(f32),

    TimelineCopyKeyframes,
    TimelinePasteKeyframes { paste_time: f32 },
    TimelineMirrorPaste { paste_time: f32 },

    TimelineCaptureBuffer,
    TimelineSwapBuffer,

    ClipInstanceSelect { entity: Entity, instance_id: ClipInstanceId },
    ClipInstanceDeselect,
    ClipInstanceMove { entity: Entity, instance_id: ClipInstanceId, new_start_time: f32 },
    ClipInstanceTrimStart { entity: Entity, instance_id: ClipInstanceId, new_clip_in: f32 },
    ClipInstanceTrimEnd { entity: Entity, instance_id: ClipInstanceId, new_clip_out: f32 },
    ClipInstanceToggleMute { entity: Entity, instance_id: ClipInstanceId },
    ClipInstanceDelete { entity: Entity, instance_id: ClipInstanceId },
    ClipInstanceSetWeight { entity: Entity, instance_id: ClipInstanceId, weight: f32 },
    ClipInstanceSetBlendMode { entity: Entity, instance_id: ClipInstanceId, blend_mode: BlendMode },

    ClipGroupCreate { entity: Entity, name: String },
    ClipGroupDelete { entity: Entity, group_id: ClipGroupId },
    ClipGroupAddInstance { entity: Entity, group_id: ClipGroupId, instance_id: ClipInstanceId },
    ClipGroupRemoveInstance { entity: Entity, group_id: ClipGroupId, instance_id: ClipInstanceId },
    ClipGroupToggleMute { entity: Entity, group_id: ClipGroupId },
    ClipGroupSetWeight { entity: Entity, group_id: ClipGroupId, weight: f32 },

    SaveScene,
}

#[derive(Default)]
pub struct UIEventQueue {
    events: Vec<UIEvent>,
}

impl UIEventQueue {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn send(&mut self, event: UIEvent) {
        self.events.push(event);
    }

    pub fn drain(&mut self) -> impl Iterator<Item = UIEvent> + '_ {
        self.events.drain(..)
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }
}
