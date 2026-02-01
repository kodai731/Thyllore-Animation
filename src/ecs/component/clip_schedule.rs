use crate::animation::editable::{
    ClipGroup, ClipGroupId, ClipInstance, ClipInstanceId, SourceClipId,
};

#[derive(Clone, Debug, Default)]
pub struct ClipSchedule {
    pub instances: Vec<ClipInstance>,
    pub groups: Vec<ClipGroup>,
    next_instance_id: ClipInstanceId,
    next_group_id: ClipGroupId,
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

    pub fn remove_instance(
        &mut self,
        instance_id: ClipInstanceId,
    ) -> bool {
        let before = self.instances.len();
        self.instances.retain(|i| i.instance_id != instance_id);

        for group in &mut self.groups {
            group.remove_instance(instance_id);
        }

        self.instances.len() < before
    }

    pub fn first_instance(&self) -> Option<&ClipInstance> {
        self.instances.first()
    }

    pub fn active_instances_at(&self, time: f32) -> Vec<&ClipInstance> {
        self.instances
            .iter()
            .filter(|i| {
                if !i.is_active_at(time) {
                    return false;
                }
                if let Some(group) = self.find_group_for_instance(i.instance_id) {
                    return !group.muted;
                }
                true
            })
            .collect()
    }

    pub fn create_group(&mut self, name: String) -> ClipGroupId {
        let id = self.next_group_id;
        self.next_group_id += 1;
        self.groups.push(ClipGroup::new(id, name));
        id
    }

    pub fn remove_group(&mut self, group_id: ClipGroupId) {
        self.groups.retain(|g| g.id != group_id);
    }

    pub fn add_instance_to_group(
        &mut self,
        group_id: ClipGroupId,
        instance_id: ClipInstanceId,
    ) {
        for group in &mut self.groups {
            group.remove_instance(instance_id);
        }

        if let Some(group) = self.groups.iter_mut().find(|g| g.id == group_id) {
            group.add_instance(instance_id);
        }
    }

    pub fn remove_instance_from_group(
        &mut self,
        group_id: ClipGroupId,
        instance_id: ClipInstanceId,
    ) {
        if let Some(group) = self.groups.iter_mut().find(|g| g.id == group_id) {
            group.remove_instance(instance_id);
        }
    }

    pub fn find_group_for_instance(
        &self,
        instance_id: ClipInstanceId,
    ) -> Option<&ClipGroup> {
        self.groups
            .iter()
            .find(|g| g.contains_instance(instance_id))
    }

    pub fn effective_instance_weight(
        &self,
        instance_id: ClipInstanceId,
    ) -> f32 {
        let inst_weight = self
            .instances
            .iter()
            .find(|i| i.instance_id == instance_id)
            .map(|i| i.weight)
            .unwrap_or(0.0);

        match self.find_group_for_instance(instance_id) {
            Some(group) if group.muted => 0.0,
            Some(group) => inst_weight * group.weight,
            None => inst_weight,
        }
    }
}
