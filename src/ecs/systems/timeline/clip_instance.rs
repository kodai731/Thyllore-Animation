use crate::animation::editable::{
    ClipGroup, ClipGroupId, ClipInstance, ClipInstanceId, EditableAnimationClip, SourceClipId,
};
use crate::ecs::component::ClipSchedule;
use crate::ecs::resource::{ClipLibrary, HumanoidRigState, TimelineState};
use crate::ecs::systems::clip_schedule_systems::{
    clip_schedule_add_instance, clip_schedule_remove_instance,
};
use crate::ecs::systems::humanoid_bake_systems::unresolved_clip_roles;
use crate::ecs::systems::timeline::timeline_select_clip;
use crate::ecs::world::{Entity, World};

pub fn add_clip_instance(
    world: &mut World,
    entity: Entity,
    source_id: SourceClipId,
    start_time: f32,
) {
    let duration = {
        let library = world.resource::<ClipLibrary>();
        let clip = library.get(source_id);
        if let Some(clip) = clip {
            warn_unresolved_roles(world, clip);
        }
        clip.map(|c| c.duration).unwrap_or(1.0)
    };

    let Some(schedule) = world.get_component_mut::<ClipSchedule>(entity) else {
        return;
    };
    clip_schedule_add_instance(schedule, source_id, duration);
    if let Some(last) = schedule.instances.last_mut() {
        last.start_time = start_time;
    }
}

fn warn_unresolved_roles(world: &World, clip: &EditableAnimationClip) {
    let rig_state = world.get_resource::<HumanoidRigState>();
    let Some(rig) = rig_state.as_ref().and_then(|state| state.rig.as_ref()) else {
        return;
    };
    let unresolved = unresolved_clip_roles(clip, Some(&rig.mapping));
    if !unresolved.is_empty() {
        log_warn!(
            "clip '{}' has unresolved roles: {:?}",
            clip.name,
            unresolved
        );
    }
}

pub fn select_clip_instance(world: &mut World, entity: Entity, instance_id: ClipInstanceId) {
    let source_id = world
        .get_component::<ClipSchedule>(entity)
        .and_then(|schedule| {
            schedule
                .instances
                .iter()
                .find(|i| i.instance_id == instance_id)
        })
        .map(|instance| instance.source_id);

    let mut timeline_state = world.resource_mut::<TimelineState>();
    timeline_state.selected_clip_instance = Some((entity, instance_id));
    if let Some(source_id) = source_id {
        let clip_library = world.resource::<ClipLibrary>();
        timeline_select_clip(&mut timeline_state, &clip_library, source_id);
    }
}

pub fn remove_clip_instance(world: &mut World, entity: Entity, instance_id: ClipInstanceId) {
    if let Some(schedule) = world.get_component_mut::<ClipSchedule>(entity) {
        clip_schedule_remove_instance(schedule, instance_id);
    }

    let mut timeline_state = world.resource_mut::<TimelineState>();
    if timeline_state.selected_clip_instance == Some((entity, instance_id)) {
        timeline_state.selected_clip_instance = None;
    }
}

pub fn modify_clip_instance(
    world: &mut World,
    entity: Entity,
    instance_id: ClipInstanceId,
    edit: impl FnOnce(&mut ClipInstance),
) {
    let Some(schedule) = world.get_component_mut::<ClipSchedule>(entity) else {
        return;
    };
    if let Some(instance) = schedule
        .instances
        .iter_mut()
        .find(|i| i.instance_id == instance_id)
    {
        edit(instance);
    }
}

pub fn modify_clip_group(
    world: &mut World,
    entity: Entity,
    group_id: ClipGroupId,
    edit: impl FnOnce(&mut ClipGroup),
) {
    let Some(schedule) = world.get_component_mut::<ClipSchedule>(entity) else {
        return;
    };
    if let Some(group) = schedule.groups.iter_mut().find(|g| g.id == group_id) {
        edit(group);
    }
}
