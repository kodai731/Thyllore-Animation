use crate::animation::editable::{ClipInstance, ClipInstanceId, SourceClipId};

#[derive(Clone, Debug, Default)]
pub struct ClipSchedule {
    pub instances: Vec<ClipInstance>,
    next_instance_id: ClipInstanceId,
}

impl ClipSchedule {
    pub fn new() -> Self {
        Self {
            instances: Vec::new(),
            next_instance_id: 1,
        }
    }

    pub fn add_instance(
        &mut self,
        source_id: SourceClipId,
        duration: f32,
    ) -> ClipInstanceId {
        let id = self.next_instance_id;
        self.next_instance_id += 1;
        let instance = ClipInstance::new(id, source_id, duration);
        self.instances.push(instance);
        id
    }

    pub fn remove_instance(&mut self, instance_id: ClipInstanceId) -> bool {
        let before = self.instances.len();
        self.instances.retain(|i| i.instance_id != instance_id);
        self.instances.len() < before
    }

    pub fn first_instance(&self) -> Option<&ClipInstance> {
        self.instances.first()
    }

    pub fn active_instances_at(&self, time: f32) -> Vec<&ClipInstance> {
        self.instances
            .iter()
            .filter(|i| i.is_active_at(time))
            .collect()
    }
}
