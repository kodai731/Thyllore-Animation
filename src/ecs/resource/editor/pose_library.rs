use crate::animation::editable::SourceClipId;

#[derive(Clone, Debug)]
pub struct PoseEntry {
    pub id: SourceClipId,
    pub captured_time: f32,
}

#[derive(Clone, Debug, Default)]
pub struct PoseLibrary {
    pub poses: Vec<PoseEntry>,
    pub selected_pose_id: Option<SourceClipId>,
}

impl PoseLibrary {
    pub fn add_pose(&mut self, id: SourceClipId, captured_time: f32) {
        self.poses.push(PoseEntry { id, captured_time });
    }

    pub fn remove_pose(&mut self, id: SourceClipId) {
        self.poses.retain(|entry| entry.id != id);
        if self.selected_pose_id == Some(id) {
            self.selected_pose_id = None;
        }
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
        lib.remove_pose(10);
        assert_eq!(lib.poses.len(), 1);
        assert!(!lib.contains(10));
        assert!(lib.selected_pose_id.is_none());
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
