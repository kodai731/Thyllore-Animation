use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use anyhow::{Context, Result};

use crate::animation::editable::{EditableAnimationClip, EditableClipId};
use crate::animation::{
    AnimationClip, AnimationClipId, AnimationSystem, BoneId, MorphAnimationSystem,
};

#[derive(Clone, Debug, Default)]
pub struct ClipLibrary {
    pub animation: AnimationSystem,
    pub morph_animation: MorphAnimationSystem,

    editable_clips: HashMap<EditableClipId, EditableAnimationClip>,
    dirty_clips: HashSet<EditableClipId>,
    next_editable_id: EditableClipId,
    editable_to_anim_id: HashMap<EditableClipId, AnimationClipId>,
}

impl ClipLibrary {
    pub fn new() -> Self {
        Self {
            animation: AnimationSystem::new(),
            morph_animation: MorphAnimationSystem::new(),
            editable_clips: HashMap::new(),
            dirty_clips: HashSet::new(),
            next_editable_id: 1,
            editable_to_anim_id: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.animation.clear();
        self.morph_animation = MorphAnimationSystem::new();
        self.clear_editable();
    }

    pub fn clear_editable(&mut self) {
        self.editable_clips.clear();
        self.dirty_clips.clear();
        self.editable_to_anim_id.clear();
    }

    pub fn create_from_imported(
        &mut self,
        clip: &AnimationClip,
        bone_names: &HashMap<BoneId, String>,
    ) -> EditableClipId {
        let id = self.next_editable_id;
        self.next_editable_id += 1;

        let editable = EditableAnimationClip::from_animation_clip(id, clip, bone_names);
        self.editable_clips.insert(id, editable);
        self.editable_to_anim_id.insert(id, clip.id);
        id
    }

    pub fn create_empty(&mut self, name: String) -> EditableClipId {
        let id = self.next_editable_id;
        self.next_editable_id += 1;

        let clip = EditableAnimationClip::new(id, name);
        self.editable_clips.insert(id, clip);
        id
    }

    pub fn register_clip(&mut self, mut clip: EditableAnimationClip) -> EditableClipId {
        let id = self.next_editable_id;
        self.next_editable_id += 1;
        clip.id = id;
        self.editable_clips.insert(id, clip);
        id
    }

    pub fn get(&self, id: EditableClipId) -> Option<&EditableAnimationClip> {
        self.editable_clips.get(&id)
    }

    pub fn get_mut(&mut self, id: EditableClipId) -> Option<&mut EditableAnimationClip> {
        self.dirty_clips.insert(id);
        self.editable_clips.get_mut(&id)
    }

    pub fn remove(&mut self, id: EditableClipId) -> Option<EditableAnimationClip> {
        self.dirty_clips.remove(&id);
        self.editable_clips.remove(&id)
    }

    pub fn to_playable_clip(&self, id: EditableClipId) -> Option<AnimationClip> {
        self.editable_clips.get(&id).map(|clip| clip.to_animation_clip())
    }

    pub fn is_dirty(&self, id: EditableClipId) -> bool {
        self.dirty_clips.contains(&id)
    }

    pub fn mark_clean(&mut self, id: EditableClipId) {
        self.dirty_clips.remove(&id);
    }

    pub fn mark_dirty(&mut self, id: EditableClipId) {
        if self.editable_clips.contains_key(&id) {
            self.dirty_clips.insert(id);
        }
    }

    pub fn dirty_clip_ids(&self) -> impl Iterator<Item = &EditableClipId> {
        self.dirty_clips.iter()
    }

    pub fn all_clip_ids(&self) -> impl Iterator<Item = &EditableClipId> {
        self.editable_clips.keys()
    }

    pub fn clip_count(&self) -> usize {
        self.editable_clips.len()
    }

    pub fn sync_dirty_clips(&mut self) {
        for editable_id in self.dirty_clips.drain() {
            let (clip, anim_id) = match (
                self.editable_clips.get(&editable_id),
                self.editable_to_anim_id.get(&editable_id),
            ) {
                (Some(c), Some(&id)) => (c, id),
                _ => continue,
            };

            let mut playable = clip.to_animation_clip();
            playable.id = anim_id;

            if let Some(target) = self.animation.clips.iter_mut().find(|c| c.id == anim_id) {
                *target = playable;
            }
        }
    }

    pub fn clip_names(&self) -> Vec<(EditableClipId, String)> {
        self.editable_clips
            .iter()
            .map(|(&id, clip)| (id, clip.name.clone()))
            .collect()
    }

    pub fn save_to_file(&self, id: EditableClipId, path: &Path) -> Result<()> {
        let clip = self
            .editable_clips
            .get(&id)
            .context("Clip not found")?;

        let file = fs::File::create(path)
            .with_context(|| format!("Failed to create file: {:?}", path))?;
        let writer = BufWriter::new(file);

        ron::ser::to_writer_pretty(writer, clip, ron::ser::PrettyConfig::default())
            .with_context(|| format!("Failed to serialize clip to: {:?}", path))?;

        crate::log!("Saved animation clip '{}' to {:?}", clip.name, path);
        Ok(())
    }

    pub fn load_from_file(&mut self, path: &Path) -> Result<EditableClipId> {
        let file = fs::File::open(path)
            .with_context(|| format!("Failed to open file: {:?}", path))?;
        let reader = BufReader::new(file);

        let mut clip: EditableAnimationClip = ron::de::from_reader(reader)
            .with_context(|| format!("Failed to deserialize clip from: {:?}", path))?;

        let id = self.next_editable_id;
        self.next_editable_id += 1;
        clip.id = id;
        clip.source_path = Some(path.to_string_lossy().to_string());

        crate::log!(
            "Loaded animation clip '{}' from {:?} (id={})",
            clip.name,
            path,
            id
        );

        self.editable_clips.insert(id, clip);
        Ok(id)
    }
}
