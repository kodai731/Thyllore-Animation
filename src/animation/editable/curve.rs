use serde::{Deserialize, Serialize};

use super::keyframe::{CurveId, EditableKeyframe, KeyframeId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PropertyType {
    TranslationX,
    TranslationY,
    TranslationZ,
    RotationX,
    RotationY,
    RotationZ,
    RotationW,
    ScaleX,
    ScaleY,
    ScaleZ,
}

impl PropertyType {
    pub fn display_name(&self) -> &'static str {
        match self {
            PropertyType::TranslationX => "Position X",
            PropertyType::TranslationY => "Position Y",
            PropertyType::TranslationZ => "Position Z",
            PropertyType::RotationX => "Rotation X",
            PropertyType::RotationY => "Rotation Y",
            PropertyType::RotationZ => "Rotation Z",
            PropertyType::RotationW => "Rotation W",
            PropertyType::ScaleX => "Scale X",
            PropertyType::ScaleY => "Scale Y",
            PropertyType::ScaleZ => "Scale Z",
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            PropertyType::TranslationX => "Pos.X",
            PropertyType::TranslationY => "Pos.Y",
            PropertyType::TranslationZ => "Pos.Z",
            PropertyType::RotationX => "Rot.X",
            PropertyType::RotationY => "Rot.Y",
            PropertyType::RotationZ => "Rot.Z",
            PropertyType::RotationW => "Rot.W",
            PropertyType::ScaleX => "Scl.X",
            PropertyType::ScaleY => "Scl.Y",
            PropertyType::ScaleZ => "Scl.Z",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PropertyCurve {
    pub id: CurveId,
    pub property_type: PropertyType,
    pub keyframes: Vec<EditableKeyframe>,
    next_keyframe_id: KeyframeId,
}

impl PropertyCurve {
    pub fn new(id: CurveId, property_type: PropertyType) -> Self {
        Self {
            id,
            property_type,
            keyframes: Vec::new(),
            next_keyframe_id: 1,
        }
    }

    pub fn add_keyframe(&mut self, time: f32, value: f32) -> KeyframeId {
        let id = self.next_keyframe_id;
        self.next_keyframe_id += 1;

        let keyframe = EditableKeyframe::new(id, time, value);
        self.keyframes.push(keyframe);
        self.sort_keyframes();
        id
    }

    pub fn remove_keyframe(&mut self, keyframe_id: KeyframeId) -> bool {
        if let Some(pos) = self.keyframes.iter().position(|k| k.id == keyframe_id) {
            self.keyframes.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn get_keyframe(&self, keyframe_id: KeyframeId) -> Option<&EditableKeyframe> {
        self.keyframes.iter().find(|k| k.id == keyframe_id)
    }

    pub fn get_keyframe_mut(&mut self, keyframe_id: KeyframeId) -> Option<&mut EditableKeyframe> {
        self.keyframes.iter_mut().find(|k| k.id == keyframe_id)
    }

    pub fn set_keyframe_time(&mut self, keyframe_id: KeyframeId, time: f32) {
        if let Some(kf) = self.get_keyframe_mut(keyframe_id) {
            kf.time = time;
        }
        self.sort_keyframes();
    }

    pub fn set_keyframe_value(&mut self, keyframe_id: KeyframeId, value: f32) {
        if let Some(kf) = self.get_keyframe_mut(keyframe_id) {
            kf.value = value;
        }
    }

    fn sort_keyframes(&mut self) {
        self.keyframes
            .sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal));
    }

    pub fn sample(&self, time: f32) -> Option<f32> {
        if self.keyframes.is_empty() {
            return None;
        }

        if self.keyframes.len() == 1 {
            return Some(self.keyframes[0].value);
        }

        if time <= self.keyframes[0].time {
            return Some(self.keyframes[0].value);
        }

        let last = self.keyframes.last().unwrap();
        if time >= last.time {
            return Some(last.value);
        }

        for i in 0..self.keyframes.len() - 1 {
            let k0 = &self.keyframes[i];
            let k1 = &self.keyframes[i + 1];

            if time >= k0.time && time < k1.time {
                let t = (time - k0.time) / (k1.time - k0.time);
                return Some(k0.value + (k1.value - k0.value) * t);
            }
        }

        Some(last.value)
    }

    pub fn is_empty(&self) -> bool {
        self.keyframes.is_empty()
    }

    pub fn keyframe_count(&self) -> usize {
        self.keyframes.len()
    }
}

impl Default for PropertyCurve {
    fn default() -> Self {
        Self {
            id: 0,
            property_type: PropertyType::TranslationX,
            keyframes: Vec::new(),
            next_keyframe_id: 1,
        }
    }
}
