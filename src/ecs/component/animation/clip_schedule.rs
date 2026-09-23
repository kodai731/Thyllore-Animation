use crate::animation::editable::{ClipGroup, ClipGroupId, ClipInstance, ClipInstanceId};

#[derive(Clone, Debug, Default)]
pub struct ClipSchedule {
    pub instances: Vec<ClipInstance>,
    pub groups: Vec<ClipGroup>,
    pub next_instance_id: ClipInstanceId,
    pub next_group_id: ClipGroupId,
}

impl ClipSchedule {
    pub fn new() -> Self {
        Self {
            instances: Vec::new(),
            groups: Vec::new(),
            next_instance_id: 1,
            next_group_id: 1,
        }
    }

    pub fn first_instance(&self) -> Option<&ClipInstance> {
        self.instances.first()
    }
}
