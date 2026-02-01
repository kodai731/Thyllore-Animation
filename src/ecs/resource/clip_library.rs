use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use anyhow::{Context, Result};

use crate::animation::editable::{EditableAnimationClip, SourceClip, SourceClipId};
use crate::animation::{
    AnimationClip, AnimationClipId, AnimationSystem, BoneId, MorphAnimationSystem,
};

#[derive(Clone, Debug, Default)]
pub struct ClipLibrary {
    pub animation: AnimationSystem,
    pub morph_animation: MorphAnimationSystem,

    source_clips: HashMap<SourceClipId, SourceClip>,
    dirty_sources: HashSet<SourceClipId>,
    next_source_id: SourceClipId,
    source_to_anim_id: HashMap<SourceClipId, AnimationClipId>,
}

impl ClipLibrary {
    pub fn new() -> Self {
        Self {
            animation: AnimationSystem::new(),
            morph_animation: MorphAnimationSystem::new(),
            source_clips: HashMap::new(),
            dirty_sources: HashSet::new(),
            next_source_id: 1,
            source_to_anim_id: HashMap::new(),
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
        self.source_to_anim_id.clear();
    }

    pub fn create_from_imported(
        &mut self,
        clip: &AnimationClip,
        bone_names: &HashMap<BoneId, String>,
    ) -> SourceClipId {
        let id = self.next_source_id;
        self.next_source_id += 1;

        let editable = EditableAnimationClip::from_animation_clip(id, clip, bone_names);
        let source = SourceClip::new(id, editable);
        self.source_clips.insert(id, source);
        self.source_to_anim_id.insert(id, clip.id);
        id
    }

    pub fn create_empty(&mut self, name: String) -> SourceClipId {
        let id = self.next_source_id;
        self.next_source_id += 1;

        let editable = EditableAnimationClip::new(id, name);
        let source = SourceClip::new(id, editable);
        self.source_clips.insert(id, source);
        id
    }

    pub fn register_clip(
        &mut self,
        mut clip: EditableAnimationClip,
    ) -> SourceClipId {
        let id = self.next_source_id;
        self.next_source_id += 1;
        clip.id = id;
        let source = SourceClip::new(id, clip);
        self.source_clips.insert(id, source);
        id
    }

    pub fn get(&self, id: SourceClipId) -> Option<&EditableAnimationClip> {
        self.source_clips.get(&id).map(|s| &s.editable_clip)
    }

    pub fn get_mut(
        &mut self,
        id: SourceClipId,
    ) -> Option<&mut EditableAnimationClip> {
        self.dirty_sources.insert(id);
        self.source_clips
            .get_mut(&id)
            .map(|s| &mut s.editable_clip)
    }

    pub fn get_source(&self, id: SourceClipId) -> Option<&SourceClip> {
        self.source_clips.get(&id)
    }

    pub fn get_source_mut(&mut self, id: SourceClipId) -> Option<&mut SourceClip> {
        self.source_clips.get_mut(&id)
    }

    pub fn get_anim_clip_id_for_source(
        &self,
        source_id: SourceClipId,
    ) -> Option<AnimationClipId> {
        self.source_to_anim_id.get(&source_id).copied()
    }

    pub fn find_source_id_for_anim_clip(
        &self,
        anim_id: AnimationClipId,
    ) -> Option<SourceClipId> {
        self.source_to_anim_id
            .iter()
            .find(|(_, &aid)| aid == anim_id)
            .map(|(&sid, _)| sid)
    }

    pub fn remove(
        &mut self,
        id: SourceClipId,
    ) -> Option<EditableAnimationClip> {
        self.dirty_sources.remove(&id);
        self.source_clips
            .remove(&id)
            .map(|s| s.editable_clip)
    }

    pub fn to_playable_clip(
        &self,
        id: SourceClipId,
    ) -> Option<AnimationClip> {
        self.source_clips
            .get(&id)
            .map(|s| s.editable_clip.to_animation_clip())
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

    pub fn sync_dirty_clips(&mut self) {
        for source_id in self.dirty_sources.drain() {
            let (clip, anim_id) = match (
                self.source_clips.get(&source_id),
                self.source_to_anim_id.get(&source_id),
            ) {
                (Some(s), Some(&id)) => (&s.editable_clip, id),
                _ => continue,
            };

            let mut playable = clip.to_animation_clip();
            playable.id = anim_id;

            if let Some(target) =
                self.animation.clips.iter_mut().find(|c| c.id == anim_id)
            {
                *target = playable;
            }
        }
    }

    pub fn clip_names(&self) -> Vec<(SourceClipId, String)> {
        self.source_clips
            .iter()
            .map(|(&id, s)| (id, s.editable_clip.name.clone()))
            .collect()
    }

    pub fn save_to_file(
        &self,
        id: SourceClipId,
        path: &Path,
    ) -> Result<()> {
        let source = self
            .source_clips
            .get(&id)
            .context("Clip not found")?;

        let file = fs::File::create(path)
            .with_context(|| format!("Failed to create file: {:?}", path))?;
        let writer = BufWriter::new(file);

        ron::ser::to_writer_pretty(
            writer,
            &source.editable_clip,
            ron::ser::PrettyConfig::default(),
        )
        .with_context(|| {
            format!("Failed to serialize clip to: {:?}", path)
        })?;

        crate::log!(
            "Saved animation clip '{}' to {:?}",
            source.editable_clip.name,
            path
        );
        Ok(())
    }

    pub fn load_from_file(&mut self, path: &Path) -> Result<SourceClipId> {
        let file = fs::File::open(path)
            .with_context(|| format!("Failed to open file: {:?}", path))?;
        let reader = BufReader::new(file);

        let mut clip: EditableAnimationClip = ron::de::from_reader(reader)
            .with_context(|| {
                format!("Failed to deserialize clip from: {:?}", path)
            })?;

        let id = self.next_source_id;
        self.next_source_id += 1;
        clip.id = id;
        clip.source_path = Some(path.to_string_lossy().to_string());

        crate::log!(
            "Loaded animation clip '{}' from {:?} (id={})",
            clip.name,
            path,
            id
        );

        let source = SourceClip::new(id, clip);
        self.source_clips.insert(id, source);
        Ok(id)
    }
}
