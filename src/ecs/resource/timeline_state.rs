use std::collections::HashSet;

use crate::animation::editable::{EditableClipId, KeyframeId, PropertyType};
use crate::animation::BoneId;
use crate::ecs::world::Entity;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SelectedKeyframe {
    pub bone_id: BoneId,
    pub property_type: PropertyType,
    pub keyframe_id: KeyframeId,
}

impl SelectedKeyframe {
    pub fn new(bone_id: BoneId, property_type: PropertyType, keyframe_id: KeyframeId) -> Self {
        Self {
            bone_id,
            property_type,
            keyframe_id,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TimelineState {
    pub current_clip_id: Option<EditableClipId>,
    pub current_time: f32,
    pub playing: bool,
    pub looping: bool,
    pub speed: f32,
    pub zoom_level: f32,
    pub scroll_offset: f32,
    pub selected_keyframes: HashSet<SelectedKeyframe>,
    pub expanded_tracks: HashSet<BoneId>,
    pub show_translation: bool,
    pub show_rotation: bool,
    pub show_scale: bool,
    pub target_entity: Option<Entity>,
    pub scrubbing: bool,
}

impl TimelineState {
    pub fn new() -> Self {
        Self {
            current_clip_id: None,
            current_time: 0.0,
            playing: false,
            looping: true,
            speed: 1.0,
            zoom_level: 1.0,
            scroll_offset: 0.0,
            selected_keyframes: HashSet::new(),
            expanded_tracks: HashSet::new(),
            show_translation: true,
            show_rotation: true,
            show_scale: true,
            target_entity: None,
            scrubbing: false,
        }
    }

    pub fn select_keyframe(&mut self, keyframe: SelectedKeyframe) {
        self.selected_keyframes.clear();
        self.selected_keyframes.insert(keyframe);
    }

    pub fn add_keyframe_to_selection(&mut self, keyframe: SelectedKeyframe) {
        self.selected_keyframes.insert(keyframe);
    }

    pub fn remove_keyframe_from_selection(&mut self, keyframe: &SelectedKeyframe) {
        self.selected_keyframes.remove(keyframe);
    }

    pub fn clear_selection(&mut self) {
        self.selected_keyframes.clear();
    }

    pub fn is_keyframe_selected(&self, keyframe: &SelectedKeyframe) -> bool {
        self.selected_keyframes.contains(keyframe)
    }

    pub fn toggle_track_expanded(&mut self, bone_id: BoneId) {
        if self.expanded_tracks.contains(&bone_id) {
            self.expanded_tracks.remove(&bone_id);
        } else {
            self.expanded_tracks.insert(bone_id);
        }
    }

    pub fn is_track_expanded(&self, bone_id: BoneId) -> bool {
        self.expanded_tracks.contains(&bone_id)
    }

    pub fn expand_track(&mut self, bone_id: BoneId) {
        self.expanded_tracks.insert(bone_id);
    }

    pub fn collapse_track(&mut self, bone_id: BoneId) {
        self.expanded_tracks.remove(&bone_id);
    }

    pub fn set_time(&mut self, time: f32) {
        self.current_time = time.max(0.0);
    }

    pub fn zoom_in(&mut self) {
        self.zoom_level = (self.zoom_level * 1.2).min(10.0);
    }

    pub fn zoom_out(&mut self) {
        self.zoom_level = (self.zoom_level / 1.2).max(0.1);
    }
}

impl Default for TimelineState {
    fn default() -> Self {
        Self::new()
    }
}
