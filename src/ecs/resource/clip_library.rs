use std::collections::{HashMap, HashSet};

use crate::animation::editable::{EditableAnimationClip, SourceClip, SourceClipId};
use crate::animation::{AnimationSystem, MorphAnimationSystem};
use crate::asset::AssetId;

#[derive(Clone, Debug, Default)]
pub struct ClipLibrary {
    pub animation: AnimationSystem,
    pub morph_animation: MorphAnimationSystem,

    pub source_clips: HashMap<SourceClipId, SourceClip>,
    pub dirty_sources: HashSet<SourceClipId>,
    pub next_source_id: SourceClipId,
    pub source_to_asset_id: HashMap<SourceClipId, AssetId>,
}

impl ClipLibrary {
    pub fn new() -> Self {
        Self {
            animation: AnimationSystem::new(),
            morph_animation: MorphAnimationSystem::new(),
            source_clips: HashMap::new(),
            dirty_sources: HashSet::new(),
            next_source_id: 1,
            source_to_asset_id: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.animation.clear();
        self.morph_animation = MorphAnimationSystem::new();
        self.clear_editable();
    }

    pub fn clear_editable(&mut self) {
        self.source_clips.clear();
        self.dirty_sources.clear();
        self.source_to_asset_id.clear();
    }

    pub fn get(&self, id: SourceClipId) -> Option<&EditableAnimationClip> {
        self.source_clips.get(&id).map(|s| &s.editable_clip)
    }

    pub fn get_mut(&mut self, id: SourceClipId) -> Option<&mut EditableAnimationClip> {
        self.dirty_sources.insert(id);
        self.source_clips.get_mut(&id).map(|s| &mut s.editable_clip)
    }

    pub fn get_source(&self, id: SourceClipId) -> Option<&SourceClip> {
        self.source_clips.get(&id)
    }

    pub fn get_source_mut(&mut self, id: SourceClipId) -> Option<&mut SourceClip> {
        self.source_clips.get_mut(&id)
    }

    pub fn get_asset_id_for_source(&self, source_id: SourceClipId) -> Option<AssetId> {
        self.source_to_asset_id.get(&source_id).copied()
    }

    pub fn remove(&mut self, id: SourceClipId) -> Option<EditableAnimationClip> {
        self.dirty_sources.remove(&id);
        self.source_clips.remove(&id).map(|s| s.editable_clip)
    }

    pub fn is_dirty(&self, id: SourceClipId) -> bool {
        self.dirty_sources.contains(&id)
    }

    pub fn mark_clean(&mut self, id: SourceClipId) {
        self.dirty_sources.remove(&id);
    }

    pub fn mark_dirty(&mut self, id: SourceClipId) {
        if self.source_clips.contains_key(&id) {
            self.dirty_sources.insert(id);
        }
    }

    pub fn dirty_clip_ids(&self) -> impl Iterator<Item = &SourceClipId> {
        self.dirty_sources.iter()
    }

    pub fn all_clip_ids(&self) -> impl Iterator<Item = &SourceClipId> {
        self.source_clips.keys()
    }

    pub fn clip_count(&self) -> usize {
        self.source_clips.len()
    }
}
