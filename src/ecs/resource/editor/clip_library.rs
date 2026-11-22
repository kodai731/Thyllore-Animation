use std::collections::{HashMap, HashSet};

use crate::asset::AssetId;
use thyllore_anim_core::editable::{EditableAnimationClip, SourceClip, SourceClipId};
use thyllore_anim_core::AnimationSystem;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipOrigin {
    Model,
    User,
}

#[derive(Clone, Debug)]
pub struct RemovedClip {
    pub source: SourceClip,
    pub origin: ClipOrigin,
}

#[derive(Clone, Debug, Default)]
pub struct ClipLibrary {
    pub animation: AnimationSystem,

    pub source_clips: HashMap<SourceClipId, SourceClip>,
    pub dirty_sources: HashSet<SourceClipId>,
    pub next_source_id: SourceClipId,
    pub source_to_asset_id: HashMap<SourceClipId, AssetId>,
    pub model_clip_ids: HashSet<SourceClipId>,
    released_asset_ids: Vec<AssetId>,
}

impl ClipLibrary {
    pub fn new() -> Self {
        Self {
            animation: AnimationSystem::new(),
            source_clips: HashMap::new(),
            dirty_sources: HashSet::new(),
            next_source_id: 1,
            source_to_asset_id: HashMap::new(),
            model_clip_ids: HashSet::new(),
            released_asset_ids: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.animation.clear();
        self.clear_editable();
    }

    pub fn clear_editable(&mut self) {
        self.source_clips.clear();
        self.dirty_sources.clear();
        self.source_to_asset_id.clear();
    }

    pub fn clear_model_clips(&mut self) {
        self.animation.clear();
        let ids: HashSet<SourceClipId> = self.model_clip_ids.clone();
        self.source_clips.retain(|id, _| !ids.contains(id));
        self.dirty_sources.retain(|id| !ids.contains(id));
        self.source_to_asset_id.retain(|id, _| !ids.contains(id));
        self.model_clip_ids.clear();
    }

    /// A loaded clip the model did not bring stays selected across a model load.
    pub fn is_user_clip_selected(&self, selected: Option<SourceClipId>) -> bool {
        selected.is_some_and(|id| self.get(id).is_some() && !self.is_model_clip(id))
    }

    /// A model clip is still what the model file brought, so the model file is its only source.
    pub fn is_model_clip(&self, id: SourceClipId) -> bool {
        self.model_clip_ids.contains(&id)
    }

    pub fn user_clip_ids(&self) -> Vec<SourceClipId> {
        let mut ids: Vec<SourceClipId> = self
            .all_clip_ids()
            .copied()
            .filter(|id| !self.is_model_clip(*id))
            .collect();
        ids.sort_unstable();
        ids
    }

    pub fn get(&self, id: SourceClipId) -> Option<&EditableAnimationClip> {
        self.source_clips.get(&id).map(|s| &s.editable_clip)
    }

    /// Editing a model clip makes it the user's clip.
    pub fn get_mut(&mut self, id: SourceClipId) -> Option<&mut EditableAnimationClip> {
        self.model_clip_ids.remove(&id);
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

    pub fn origin(&self, id: SourceClipId) -> ClipOrigin {
        if self.is_model_clip(id) {
            ClipOrigin::Model
        } else {
            ClipOrigin::User
        }
    }

    /// The playable asset of the removed clip is released by the next playable asset sync.
    pub fn remove(&mut self, id: SourceClipId) -> Option<RemovedClip> {
        let origin = self.origin(id);

        self.dirty_sources.remove(&id);
        self.model_clip_ids.remove(&id);
        self.released_asset_ids
            .extend(self.source_to_asset_id.remove(&id));

        self.source_clips
            .remove(&id)
            .map(|source| RemovedClip { source, origin })
    }

    /// The restored clip gets its playable asset from the next playable asset sync.
    pub fn restore(&mut self, id: SourceClipId, removed: RemovedClip) {
        self.source_clips.insert(id, removed.source);
        self.dirty_sources.insert(id);

        match removed.origin {
            ClipOrigin::Model => self.model_clip_ids.insert(id),
            ClipOrigin::User => self.model_clip_ids.remove(&id),
        };
    }

    pub fn take_released_asset_ids(&mut self) -> Vec<AssetId> {
        std::mem::take(&mut self.released_asset_ids)
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

    pub fn find_source_by_name(&self, name: &str) -> Option<SourceClipId> {
        self.source_clips
            .iter()
            .find(|(_, s)| s.editable_clip.name == name)
            .map(|(id, _)| *id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLIP_ID: SourceClipId = 7;
    const ASSET_ID: AssetId = 107;

    fn library_with_model_clip() -> ClipLibrary {
        let mut library = ClipLibrary::new();
        let clip = EditableAnimationClip::new(CLIP_ID, "clip".to_string());
        library
            .source_clips
            .insert(CLIP_ID, SourceClip::new(CLIP_ID, clip));
        library.source_to_asset_id.insert(CLIP_ID, ASSET_ID);
        library.model_clip_ids.insert(CLIP_ID);
        library.dirty_sources.insert(CLIP_ID);
        library
    }

    #[test]
    fn remove_leaves_no_record_of_the_clip() {
        let mut library = library_with_model_clip();

        let removed = library.remove(CLIP_ID).expect("removed clip");

        assert_eq!(removed.origin, ClipOrigin::Model);
        assert!(library.get(CLIP_ID).is_none());
        assert!(!library.is_model_clip(CLIP_ID));
        assert!(!library.is_dirty(CLIP_ID));
        assert!(library.get_asset_id_for_source(CLIP_ID).is_none());
        assert_eq!(library.take_released_asset_ids(), vec![ASSET_ID]);
    }

    #[test]
    fn remove_of_an_unknown_clip_releases_nothing() {
        let mut library = ClipLibrary::new();

        assert!(library.remove(CLIP_ID).is_none());
        assert!(library.take_released_asset_ids().is_empty());
    }

    #[test]
    fn restore_brings_back_the_clip_with_its_origin() {
        let mut library = library_with_model_clip();
        let removed = library.remove(CLIP_ID).expect("removed clip");

        library.restore(CLIP_ID, removed);

        assert!(library.get(CLIP_ID).is_some());
        assert!(library.is_model_clip(CLIP_ID));
        assert!(library.is_dirty(CLIP_ID));
    }
}
