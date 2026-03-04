use crate::animation::editable::SourceClipId;

#[derive(Clone, Debug, Default)]
pub struct PoseLibrary {
    pub pose_ids: Vec<SourceClipId>,
    pub selected_pose_id: Option<SourceClipId>,
    pub naming_active: bool,
    pub name_buffer: String,
}

impl PoseLibrary {
    pub fn add_pose(&mut self, id: SourceClipId) {
        self.pose_ids.push(id);
    }

    pub fn remove_pose(&mut self, id: SourceClipId) {
        self.pose_ids.retain(|&existing| existing != id);
        if self.selected_pose_id == Some(id) {
            self.selected_pose_id = None;
        }
    }

    pub fn contains(&self, id: SourceClipId) -> bool {
        self.pose_ids.contains(&id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state() {
        let lib = PoseLibrary::default();
        assert!(lib.pose_ids.is_empty());
        assert!(lib.selected_pose_id.is_none());
        assert!(!lib.naming_active);
        assert!(lib.name_buffer.is_empty());
    }

    #[test]
    fn test_add_and_remove_pose() {
        let mut lib = PoseLibrary::default();
        lib.add_pose(10);
        lib.add_pose(20);
        assert_eq!(lib.pose_ids.len(), 2);
        assert!(lib.contains(10));
        assert!(lib.contains(20));

        lib.selected_pose_id = Some(10);
        lib.remove_pose(10);
        assert_eq!(lib.pose_ids.len(), 1);
        assert!(!lib.contains(10));
        assert!(lib.selected_pose_id.is_none());
    }
}
