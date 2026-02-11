use crate::animation::editable::{
    ClipGroup, ClipGroupId, ClipInstance, ClipInstanceId, SourceClipId,
};
use crate::ecs::component::ClipSchedule;

pub fn clip_schedule_add_instance(
    schedule: &mut ClipSchedule,
    source_id: SourceClipId,
    duration: f32,
) -> ClipInstanceId {
    let id = schedule.next_instance_id;
    schedule.next_instance_id += 1;
    let instance = ClipInstance::new(id, source_id, duration);
    schedule.instances.push(instance);
    id
}

pub fn clip_schedule_remove_instance(
    schedule: &mut ClipSchedule,
    instance_id: ClipInstanceId,
) -> bool {
    let before = schedule.instances.len();
    schedule.instances.retain(|i| i.instance_id != instance_id);

    for group in &mut schedule.groups {
        group.remove_instance(instance_id);
    }

    schedule.instances.len() < before
}

pub fn clip_schedule_active_instances(schedule: &ClipSchedule, time: f32) -> Vec<&ClipInstance> {
    schedule
        .instances
        .iter()
        .filter(|i| {
            if !i.is_active_at(time) {
                return false;
            }
            if let Some(group) = clip_schedule_find_group(schedule, i.instance_id) {
                return !group.muted;
            }
            true
        })
        .collect()
}

pub fn clip_schedule_create_group(schedule: &mut ClipSchedule, name: String) -> ClipGroupId {
    let id = schedule.next_group_id;
    schedule.next_group_id += 1;
    schedule.groups.push(ClipGroup::new(id, name));
    id
}

pub fn clip_schedule_remove_group(schedule: &mut ClipSchedule, group_id: ClipGroupId) {
    schedule.groups.retain(|g| g.id != group_id);
}

pub fn clip_schedule_add_to_group(
    schedule: &mut ClipSchedule,
    group_id: ClipGroupId,
    instance_id: ClipInstanceId,
) {
    for group in &mut schedule.groups {
        group.remove_instance(instance_id);
    }

    if let Some(group) = schedule.groups.iter_mut().find(|g| g.id == group_id) {
        group.add_instance(instance_id);
    }
}

pub fn clip_schedule_remove_from_group(
    schedule: &mut ClipSchedule,
    group_id: ClipGroupId,
    instance_id: ClipInstanceId,
) {
    if let Some(group) = schedule.groups.iter_mut().find(|g| g.id == group_id) {
        group.remove_instance(instance_id);
    }
}

pub fn clip_schedule_find_group(
    schedule: &ClipSchedule,
    instance_id: ClipInstanceId,
) -> Option<&ClipGroup> {
    schedule
        .groups
        .iter()
        .find(|g| g.contains_instance(instance_id))
}

pub fn clip_schedule_effective_weight(schedule: &ClipSchedule, instance_id: ClipInstanceId) -> f32 {
    let inst_weight = schedule
        .instances
        .iter()
        .find(|i| i.instance_id == instance_id)
        .map(|i| i.weight)
        .unwrap_or(0.0);

    match clip_schedule_find_group(schedule, instance_id) {
        Some(group) if group.muted => 0.0,
        Some(group) => inst_weight * group.weight,
        None => inst_weight,
    }
}
