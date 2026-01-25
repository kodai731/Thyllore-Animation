use serde::{Deserialize, Serialize};

pub type EditableClipId = u64;
pub type KeyframeId = u64;
pub type CurveId = u64;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BezierHandle {
    pub time_offset: f32,
    pub value_offset: f32,
}

impl BezierHandle {
    pub fn new(time_offset: f32, value_offset: f32) -> Self {
        Self {
            time_offset,
            value_offset,
        }
    }

    pub fn linear() -> Self {
        Self {
            time_offset: 0.0,
            value_offset: 0.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EditableKeyframe {
    pub id: KeyframeId,
    pub time: f32,
    pub value: f32,
    pub in_tangent: BezierHandle,
    pub out_tangent: BezierHandle,
}

impl EditableKeyframe {
    pub fn new(id: KeyframeId, time: f32, value: f32) -> Self {
        Self {
            id,
            time,
            value,
            in_tangent: BezierHandle::linear(),
            out_tangent: BezierHandle::linear(),
        }
    }

    pub fn with_tangents(
        id: KeyframeId,
        time: f32,
        value: f32,
        in_tangent: BezierHandle,
        out_tangent: BezierHandle,
    ) -> Self {
        Self {
            id,
            time,
            value,
            in_tangent,
            out_tangent,
        }
    }
}

impl Default for EditableKeyframe {
    fn default() -> Self {
        Self {
            id: 0,
            time: 0.0,
            value: 0.0,
            in_tangent: BezierHandle::linear(),
            out_tangent: BezierHandle::linear(),
        }
    }
}
