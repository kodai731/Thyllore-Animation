use std::collections::{HashMap, HashSet};

use crate::animation::editable::{EditableAnimationClip, SourceClipId};
use crate::asset::AssetId;

#[derive(Default)]
pub struct BakedRoleClips {
    pub by_key: HashMap<(SourceClipId, crate::ecs::world::Entity), BakedRoleClip>,
    pub failed: HashSet<(SourceClipId, crate::ecs::world::Entity)>,
}

pub struct BakedRoleClip {
    pub fps: u32,
    pub asset_id: AssetId,
    pub clip: EditableAnimationClip,
}

impl BakedRoleClips {
    pub fn invalidate_source(&mut self, source_id: SourceClipId) -> Vec<AssetId> {
        self.failed.retain(|(sid, _)| *sid != source_id);
        let keys_to_remove: Vec<_> = self
            .by_key
            .keys()
            .filter(|(sid, _)| *sid == source_id)
            .copied()
            .collect();
        let mut asset_ids = HashSet::new();
        for key in keys_to_remove {
            if let Some(entry) = self.by_key.remove(&key) {
                asset_ids.insert(entry.asset_id);
            }
        }
        asset_ids.into_iter().collect()
    }

    pub fn invalidate_entity(&mut self, entity: crate::ecs::world::Entity) -> Vec<AssetId> {
        self.failed.retain(|(_, e)| *e != entity);
        let keys_to_remove: Vec<_> = self
            .by_key
            .keys()
            .filter(|(_, e)| *e == entity)
            .copied()
            .collect();
        let mut asset_ids = HashSet::new();
        for key in keys_to_remove {
            if let Some(entry) = self.by_key.remove(&key) {
                asset_ids.insert(entry.asset_id);
            }
        }
        asset_ids.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(
        source_id: SourceClipId,
        entity: crate::ecs::world::Entity,
        asset_id: AssetId,
    ) -> BakedRoleClip {
        BakedRoleClip {
            fps: 30,
            asset_id,
            clip: EditableAnimationClip::new(0, "test".to_string()),
        }
    }

    #[test]
    fn invalidate_source_removes_every_entity_entry() {
        let mut clips = BakedRoleClips::default();
        let source: SourceClipId = 1;
        let e1: crate::ecs::world::Entity = 0;
        let e2: crate::ecs::world::Entity = 1;
        clips.by_key.insert((source, e1), make_entry(source, e1, 1));
        clips.by_key.insert((source, e2), make_entry(source, e2, 2));

        let removed = clips.invalidate_source(source);

        assert_eq!(clips.by_key.len(), 0);
        assert_eq!(removed.len(), 2);
        assert!(removed.contains(&1));
        assert!(removed.contains(&2));
    }
}
