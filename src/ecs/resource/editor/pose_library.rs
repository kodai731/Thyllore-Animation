use thyllore_anim_core::editable::SourceClipId;

#[derive(Clone, Debug)]
pub struct PoseEntry {
    pub id: SourceClipId,
    pub captured_time: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PoseSelection {
    Selected,
    NotSelected,
}

/// A pose taken out of the library with its place in the list, so it can be put back.
#[derive(Clone, Debug)]
pub struct RemovedPose {
    pub index: usize,
    pub entry: PoseEntry,
    pub selection: PoseSelection,
}

#[derive(Clone, Debug, Default)]
pub struct PoseLibrary {
    pub poses: Vec<PoseEntry>,
    pub selected_pose_id: Option<SourceClipId>,
}

crate::startup_resource!(PoseLibrary, Editor);

impl PoseLibrary {
    pub fn add_pose(&mut self, id: SourceClipId, captured_time: f32) {
        self.poses.push(PoseEntry { id, captured_time });
    }

    /// `None` for a clip that is not a pose.
    pub fn take_pose(&mut self, id: SourceClipId) -> Option<RemovedPose> {
        let index = self.poses.iter().position(|entry| entry.id == id)?;
        let selection = if self.selected_pose_id == Some(id) {
            self.selected_pose_id = None;
            PoseSelection::Selected
        } else {
            PoseSelection::NotSelected
        };

        Some(RemovedPose {
            index,
            entry: self.poses.remove(index),
            selection,
        })
    }

    pub fn restore_pose(&mut self, removed: RemovedPose) {
        let index = removed.index.min(self.poses.len());
        if removed.selection == PoseSelection::Selected {
            self.selected_pose_id = Some(removed.entry.id);
        }

        self.poses.insert(index, removed.entry);
    }

    pub fn contains(&self, id: SourceClipId) -> bool {
        self.poses.iter().any(|entry| entry.id == id)
    }

    pub fn pose_ids(&self) -> Vec<SourceClipId> {
        self.poses.iter().map(|entry| entry.id).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state() {
        let lib = PoseLibrary::default();
        assert!(lib.poses.is_empty());
        assert!(lib.selected_pose_id.is_none());
    }

    #[test]
    fn test_add_and_remove_pose() {
        let mut lib = PoseLibrary::default();
        lib.add_pose(10, 1.0);
        lib.add_pose(20, 2.5);
        assert_eq!(lib.poses.len(), 2);
        assert!(lib.contains(10));
        assert!(lib.contains(20));

        lib.selected_pose_id = Some(10);
        let removed = lib.take_pose(10).expect("pose 10");
        assert_eq!(lib.poses.len(), 1);
        assert!(!lib.contains(10));
        assert!(lib.selected_pose_id.is_none());

        lib.restore_pose(removed);
        assert_eq!(lib.pose_ids(), vec![10, 20]);
        assert_eq!(lib.selected_pose_id, Some(10));
        assert_eq!(lib.poses[0].captured_time, 1.0);
    }

    #[test]
    fn taking_a_clip_that_is_not_a_pose_takes_nothing() {
        let mut lib = PoseLibrary::default();
        lib.add_pose(10, 1.0);

        assert!(lib.take_pose(99).is_none());
        assert_eq!(lib.pose_ids(), vec![10]);
    }

    #[test]
    fn test_pose_ids() {
        let mut lib = PoseLibrary::default();
        lib.add_pose(5, 0.0);
        lib.add_pose(15, 3.0);
        let ids = lib.pose_ids();
        assert_eq!(ids, vec![5, 15]);
    }
}
