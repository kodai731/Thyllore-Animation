use std::collections::{HashMap, HashSet};

use crate::animation::editable::{EditableAnimationClip, PropertyType};
use crate::animation::BoneId;

#[derive(Clone, Debug, Default)]
pub struct CurveEditorBuffer {
    pub snapshots: HashMap<(BoneId, PropertyType), Vec<(f32, f32)>>,
}

impl CurveEditorBuffer {
    pub fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }

    pub fn clear(&mut self) {
        self.snapshots.clear();
    }

    pub fn has_snapshot(
        &self,
        bone_id: BoneId,
        property_type: PropertyType,
    ) -> bool {
        self.snapshots.contains_key(&(bone_id, property_type))
    }

    pub fn get_snapshot(
        &self,
        bone_id: BoneId,
        property_type: PropertyType,
    ) -> Option<&Vec<(f32, f32)>> {
        self.snapshots.get(&(bone_id, property_type))
    }

    pub fn capture_buffer(
        &mut self,
        clip: &EditableAnimationClip,
        bone_id: BoneId,
        visible_curves: &HashSet<PropertyType>,
        duration: f32,
        sample_count: usize,
    ) {
        self.snapshots.clear();

        let track = match clip.tracks.get(&bone_id) {
            Some(t) => t,
            None => return,
        };

        let sample_count = sample_count.max(2);

        for &prop in visible_curves {
            let curve = track.get_curve(prop);
            let mut points = Vec::with_capacity(sample_count);

            for i in 0..sample_count {
                let t = duration * (i as f32) / (sample_count - 1) as f32;
                let value = curve.sample(t).unwrap_or(0.0);
                points.push((t, value));
            }

            self.snapshots.insert((bone_id, prop), points);
        }
    }

    pub fn swap_buffer(
        &mut self,
        clip: &mut EditableAnimationClip,
        bone_id: BoneId,
    ) {
        let keys_to_swap: Vec<(BoneId, PropertyType)> = self
            .snapshots
            .keys()
            .filter(|(bid, _)| *bid == bone_id)
            .cloned()
            .collect();

        if keys_to_swap.is_empty() {
            return;
        }

        let track = match clip.tracks.get_mut(&bone_id) {
            Some(t) => t,
            None => return,
        };

        for (_, prop) in &keys_to_swap {
            let curve = track.get_curve(*prop);
            let current_keyframes: Vec<_> = curve
                .keyframes
                .iter()
                .map(|kf| (kf.time, kf.value))
                .collect();

            if let Some(snapshot) = self.snapshots.get_mut(&(bone_id, *prop))
            {
                let old_snapshot =
                    std::mem::replace(snapshot, current_keyframes);

                let curve_mut = track.get_curve_mut(*prop);
                let ids: Vec<_> =
                    curve_mut.keyframes.iter().map(|kf| kf.id).collect();
                for id in ids {
                    curve_mut.remove_keyframe(id);
                }

                for (time, value) in &old_snapshot {
                    curve_mut.add_keyframe(*time, *value);
                }
            }
        }
    }
}
